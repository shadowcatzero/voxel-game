mod mesh;
pub use mesh::*;

use std::collections::{HashMap, HashSet};

use crate::util::oct_tree::OctTree;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource};
use nalgebra::Vector3;

pub const SCALE: u32 = 9;
pub const SIDE_LENGTH: usize = 2usize.pow(SCALE);
pub const SHAPE: (usize, usize, usize) = (SIDE_LENGTH, SIDE_LENGTH, SIDE_LENGTH);
pub const DIMENSIONS: Vector3<usize> = Vector3::new(SIDE_LENGTH, SIDE_LENGTH, SIDE_LENGTH);
pub const LEN: usize = SHAPE.0 * SHAPE.1 * SHAPE.2;

#[derive(Debug, Component, Clone, Deref, DerefMut)]
pub struct ChunkData {
    #[deref]
    data: OctTree,
}

impl ChunkData {
    pub fn from_tree(t: OctTree) -> Self {
        Self { data: t }
    }
    pub fn empty() -> Self {
        Self {
            data: OctTree::from_leaf(0, SCALE),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Default, Deref, DerefMut)]
pub struct ChunkPos(pub Vector3<i32>);
impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(Vector3::new(x, y, z))
    }
}
impl From<Vector3<i32>> for ChunkPos {
    fn from(val: Vector3<i32>) -> Self {
        ChunkPos(val)
    }
}

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub struct LoadedChunks {
    loaded: HashSet<ChunkPos>,
}

impl LoadedChunks {
    pub fn new() -> Self {
        Self {
            loaded: HashSet::new(),
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ChunkMap {
    #[deref]
    map: HashMap<ChunkPos, Entity>,
    pub generating: HashSet<ChunkPos>,
}

impl ChunkMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            generating: HashSet::new(),
        }
    }
}

#[derive(Bundle, Clone)]
pub struct ChunkBundle {
    pub pos: ChunkPos,
    pub data: ChunkData,
    pub mesh: ChunkMesh,
}
