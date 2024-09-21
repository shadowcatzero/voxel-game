use std::collections::{HashMap, HashSet};

use bevy_ecs::{entity::Entity, system::Commands};

use crate::{
    common::component::{ChunkBundle, ChunkData, ChunkMesh, ChunkPos},
    server::generation::generate_tree,
    util::{oct_tree::OctTree, thread::{ExitType, ThreadChannel, ThreadHandle}},
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

                // let worst = OctTree::from_fn(f_leaf, f_node, levels);

                println!(
                    "gen time: {:<5?}; size: {} nodes = {} bytes",
                    tree_time,
                    tree.raw().len(),
                    std::mem::size_of_val(tree.raw())
                );

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
