use std::collections::{HashMap, HashSet};

use crate::{
    client::render::voxel::{VoxelColor, VoxelFace},
    util::oct_tree::OctTree,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource};
use block_mesh::{ndshape::RuntimeShape, UnitQuadBuffer, RIGHT_HANDED_Y_UP_CONFIG};
use nalgebra::Vector3;
use ndarray::{s, Array3, Axis};

pub const SIDE_LENGTH: usize = 16 * 16;
pub const SHAPE: (usize, usize, usize) = (SIDE_LENGTH, SIDE_LENGTH, SIDE_LENGTH);
pub const DIMENSIONS: Vector3<usize> = Vector3::new(SIDE_LENGTH, SIDE_LENGTH, SIDE_LENGTH);
pub const LEN: usize = SHAPE.0 * SHAPE.1 * SHAPE.2;

#[derive(Debug, Component, Clone, Deref, DerefMut)]
pub struct ChunkData {
    #[deref]
    data: OctTree<VoxelColor>,
}

impl ChunkData {
    pub fn empty() -> Self {
        Self {
            data: OctTree::Leaf(VoxelColor::none()),
        }
    }

    pub fn from_tree(t: OctTree<VoxelColor>) -> Self {
        Self { data: t }
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

#[derive(Debug, Clone, Component)]
pub struct ChunkMesh {
    pub faces: [Vec<VoxelFace>; 6],
}

impl ChunkMesh {
    pub fn from_data(data: &Array3<VoxelColor>) -> Self {
        let dim_pad = Vector3::new(
            data.len_of(Axis(0)) as u32,
            data.len_of(Axis(1)) as u32,
            data.len_of(Axis(2)) as u32,
        );
        let dim = dim_pad - Vector3::from_element(2);
        let mut buffer = UnitQuadBuffer::new();
        let shape = RuntimeShape::<u32, 3>::new(dim_pad.into());
        let slice = data.as_slice().unwrap();
        block_mesh::visible_block_faces(
            slice,
            &shape,
            [0; 3],
            (dim_pad - Vector3::new(1, 1, 1)).into(),
            &RIGHT_HANDED_Y_UP_CONFIG.faces,
            &mut buffer,
        );
        let faces = [2, 1, 0, 5, 4, 3].map(|f| {
            buffer.groups[f]
                .iter()
                .map(|a| {
                    let i = (a.minimum[0]-1) + (a.minimum[1]-1) * dim.y + (a.minimum[2]-1) * dim.y * dim.x;
                    let i_pad = a.minimum[0] + a.minimum[1] * dim_pad.y + a.minimum[2] * dim_pad.y * dim_pad.x;
                    VoxelFace {
                        index: i,
                        color: slice[i_pad as usize],
                    }
                })
                .collect()
        });
        Self { faces }
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
