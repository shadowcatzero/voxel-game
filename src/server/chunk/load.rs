use std::collections::{HashMap, HashSet};

use bevy_ecs::{entity::Entity, system::Commands};
use nalgebra::Vector3;
use ndarray::{s, Array3, Axis};
use simdnoise::NoiseBuilder;

use crate::{
    client::render::voxel::VoxelColor,
    common::component::{chunk, ChunkBundle, ChunkData, ChunkMesh, ChunkPos},
    util::{
        oct_tree::OctTree,
        thread::{ExitType, ThreadChannel, ThreadHandle},
    },
};

pub struct ChunkManager {
    handles: Vec<ThreadHandle<ChunkLoaderMsg, ServerChunkMsg>>,
    n: usize,
    map: HashMap<ChunkPos, Entity>,
    generating: HashSet<ChunkPos>,
    available: Vec<usize>,
}

impl ChunkManager {
    pub fn new() -> Self {
        let n = 1;
        Self {
            handles: std::iter::repeat_with(|| ThreadHandle::spawn(chunk_loader_main))
                .take(n)
                .collect(),
            n,
            map: HashMap::new(),
            generating: HashSet::new(),
            available: (0..n).collect(),
        }
    }
    pub fn entity_at(&self, pos: &ChunkPos) -> Option<&Entity> {
        self.map.get(pos)
    }
    pub fn is_generating(&self, pos: &ChunkPos) -> bool {
        self.generating.contains(pos)
    }
    pub fn try_load(&mut self, pos: ChunkPos) -> bool {
        if !self.is_generating(&pos) {
            if let Some(i) = self.available.pop() {
                self.handles[i].send(ChunkLoaderMsg::Generate(pos));
                self.generating.insert(pos);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn update(&mut self, commands: &mut Commands) {
        self.handles.iter_mut().enumerate().for_each(|(i, h)| {
            if let Some(msg) = h.recv().next() {
                self.available.push(i);
                match msg {
                    ServerChunkMsg::ChunkGenerated(chunk) => {
                        let id = commands
                            .spawn(ChunkBundle {
                                pos: chunk.pos,
                                data: chunk.data,
                                mesh: chunk.mesh,
                            })
                            .id();
                        self.map.insert(chunk.pos, id);
                        self.generating.remove(&chunk.pos);
                    }
                }
            }
        });
    }
}

pub struct GeneratedChunk {
    pub pos: ChunkPos,
    pub data: ChunkData,
    pub mesh: ChunkMesh,
}

impl Drop for ChunkManager {
    fn drop(&mut self) {
        for h in &mut self.handles {
            h.send(ChunkLoaderMsg::Exit);
            h.join();
        }
    }
}

enum ServerChunkMsg {
    ChunkGenerated(GeneratedChunk),
}

enum ChunkLoaderMsg {
    Generate(ChunkPos),
    Exit,
}

impl ExitType for ChunkLoaderMsg {
    fn exit() -> Self {
        Self::Exit
    }
}

fn chunk_loader_main(channel: ThreadChannel<ServerChunkMsg, ChunkLoaderMsg>) {
    'outer: loop {
        match channel.recv_wait() {
            ChunkLoaderMsg::Generate(pos) => {
                let start = std::time::Instant::now();
                let tree = ChunkData::from_tree(generate_tree(pos));
                let tree_time = std::time::Instant::now() - start;

                // let start = std::time::Instant::now();
                // let mut data = generate(pos);
                // let data_time = std::time::Instant::now() - start;
                //
                // let start = std::time::Instant::now();
                // let shape = s![
                //     1..data.len_of(Axis(0)) - 1,
                //     1..data.len_of(Axis(1)) - 1,
                //     1..data.len_of(Axis(2)) - 1
                // ];
                // let mut slice = data.slice_mut(shape);
                // let mut iter = tree.into_iter();
                // slice.assign(&Array3::from_shape_fn(chunk::SHAPE, |_| {
                //     iter.next().unwrap()
                // }));
                // let convert_time = std::time::Instant::now() - start;
                //
                // let start = std::time::Instant::now();
                // let mesh = ChunkMesh::from_data(data.map(|i| COLOR_MAP[*i as usize]).view());
                // let mesh_time = std::time::Instant::now() - start;
                //
                // println!(
                //     "data: {:<5?} mesh: {:<5?} convert: {:<5?} tree: {:<5?}",
                //     data_time, mesh_time, convert_time, tree_time
                // );
                println!("gen time: {:<5?}", tree_time);

                channel.send(ServerChunkMsg::ChunkGenerated(GeneratedChunk {
                    pos,
                    data: tree,
                    mesh: ChunkMesh {},
                }));
            }
            ChunkLoaderMsg::Exit => {
                break 'outer;
            }
        }
    }
}

fn generate(pos: ChunkPos) -> Array3<u32> {
    let shape = [chunk::SIDE_LENGTH + 2; 3];
    if pos.y > 0 || pos.y < -1 {
        return Array3::from_elem(shape, 0);
    }
    let posf: Vector3<f32> = (pos.cast() * chunk::SIDE_LENGTH as f32) - Vector3::from_element(1.0);
    let (noise, min, max) = NoiseBuilder::gradient_2d_offset(
        posf.x,
        chunk::SIDE_LENGTH + 2,
        posf.z,
        chunk::SIDE_LENGTH + 2,
    )
    .with_seed(0)
    .with_freq(0.005)
    .generate();
    Array3::from_shape_fn(shape, |(x, y, z)| {
        generate_at(Vector3::new(x, y, z), posf, &noise, min, max)
    })
}

fn generate_tree(pos: ChunkPos) -> OctTree {
    if pos.y > 0 || pos.y < -1 {
        return OctTree::from_leaf(0, 8);
    }
    let posf: Vector3<f32> = pos.cast() * chunk::SIDE_LENGTH as f32;
    let (noise, min, max) =
        NoiseBuilder::gradient_2d_offset(posf.x, chunk::SIDE_LENGTH, posf.z, chunk::SIDE_LENGTH)
            .with_seed(0)
            .with_freq(1.0 / (chunk::SIDE_LENGTH as f32))
            .generate();
    OctTree::from_fn_rec(&mut |p| generate_at(p, posf, &noise, min, max), chunk::SCALE)
}

fn generate_at(p: Vector3<usize>, posf: Vector3<f32>, noise: &[f32], min: f32, max: f32) -> u32 {
    // 0 air 1 stone 2 "sand" 3 water
    let y = p.y as f32 + posf.y;
    // highest heights, 0.0 .. 1.0 relative to chunk size
    let [water, grass, top] = [0.18, 0.35, 0.5].map(|f| chunk::SIDE_LENGTH as f32 * f);
    let n = ((noise[p.x + p.z * chunk::SIDE_LENGTH] - min) / (max - min) * 2.0).exp2() * top * 0.25;
    if y < n {
        if y < water {
            1
        } else if y < grass {
            2
        } else {
            1
        }
    } else if y <= water {
        3
    } else {
        0
    }
}

const COLOR_MAP: [VoxelColor; 4] = [
    VoxelColor {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    VoxelColor {
        r: 150,
        g: 150,
        b: 150,
        a: 255,
    },
    VoxelColor {
        r: 100,
        g: 255,
        b: 100,
        a: 255,
    },
    VoxelColor {
        r: 100,
        g: 100,
        b: 255,
        a: 200,
    },
];
