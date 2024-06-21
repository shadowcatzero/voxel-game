use std::collections::{HashMap, HashSet, VecDeque};

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
    i: usize,
    n: usize,
    map: HashMap<ChunkPos, Entity>,
    generating: HashSet<ChunkPos>,
}

impl ChunkManager {
    pub fn new() -> Self {
        let n = 4;
        Self {
            handles: std::iter::repeat_with(|| ThreadHandle::spawn(chunk_loader_main))
                .take(n)
                .collect(),
            i: 0,
            n,
            map: HashMap::new(),
            generating: HashSet::new(),
        }
    }
    pub fn entity_at(&self, pos: &ChunkPos) -> Option<&Entity> {
        self.map.get(pos)
    }
    pub fn is_generating(&self, pos: &ChunkPos) -> bool {
        self.generating.contains(pos)
    }
    pub fn queue(&mut self, pos: ChunkPos) {
        if !self.is_generating(&pos) {
            self.handles[self.i].send(ChunkLoaderMsg::Generate(pos));
            self.i = (self.i + 1) % self.n;
            self.generating.insert(pos);
        }
    }
    pub fn update(&mut self, commands: &mut Commands) {
        for msg in self.handles.iter_mut().flat_map(|h| h.recv()) {
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
    let mut to_generate = VecDeque::new();
    'outer: loop {
        let msg = channel.recv_wait();
        match msg {
            ChunkLoaderMsg::Generate(pos) => {
                to_generate.push_back(pos);
            }
            ChunkLoaderMsg::Exit => {
                break 'outer;
            }
        }
        if let Some(pos) = to_generate.pop_front() {
            let data = generate(pos);
            let mesh = ChunkMesh::from_data(&data);
            let data = if pos.y > 0 || pos.y < -1 {
                ChunkData::empty()
            } else {
                ChunkData::from_tree(OctTree::from_arr(data.slice(s![
                    1..data.len_of(Axis(0)) - 1,
                    1..data.len_of(Axis(1)) - 1,
                    1..data.len_of(Axis(2)) - 1
                ])))
            };
            channel.send(ServerChunkMsg::ChunkGenerated(GeneratedChunk {
                pos,
                data,
                mesh,
            }));
        }
    }
}

fn generate(pos: ChunkPos) -> Array3<VoxelColor> {
    let shape = [chunk::SIDE_LENGTH + 2; 3];
    if pos.y > 0 {
        return Array3::from_elem(shape, VoxelColor::none());
    }
    if pos.y < -1 {
        return Array3::from_elem(shape, VoxelColor::none());
    }
    let posf: Vector3<f32> = (pos.cast() * chunk::SIDE_LENGTH as f32) - Vector3::from_element(1.0);
    let (a, b, c, d) = (0.0, 50.0, 100.0, 127.0);
    let (noise, ..) = NoiseBuilder::gradient_2d_offset(
        posf.x,
        chunk::SIDE_LENGTH + 2,
        posf.z,
        chunk::SIDE_LENGTH + 2,
    )
    .with_seed(0)
    .with_freq(0.005)
    .generate();
    Array3::from_shape_fn(shape, |(x, y, z)| {
        let y = y as f32 + posf.y;
        let n = (noise[x + z * (chunk::SIDE_LENGTH + 2)] + 0.022) * (1.0 / 0.044) * d;
        if y < n.max(b) {
            if y < b {
                VoxelColor {
                    r: 100,
                    g: 100,
                    b: 255,
                    a: 255,
                }
            } else if y < c {
                VoxelColor {
                    r: 100,
                    g: 255,
                    b: 100,
                    a: 255,
                }
            } else {
                VoxelColor {
                    r: 150,
                    g: 150,
                    b: 150,
                    a: 255,
                }
            }
        } else {
            VoxelColor::none()
        }
    })
}
