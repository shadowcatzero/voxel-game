pub mod chunk;
mod grid;

use chunk::LoadedChunks;
pub use chunk::{ChunkBundle, ChunkData, ChunkMap, ChunkMesh, ChunkPos};
pub use grid::*;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{bundle::Bundle, component::Component};
use nalgebra::{Rotation3, Vector3};

#[derive(Debug, Clone, Copy, Component, Default, Deref, DerefMut)]
pub struct Pos(pub Vector3<f32>);

impl Pos {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vector3::new(x, y, z))
    }
}
impl From<Vector3<f32>> for Pos {
    fn from(val: Vector3<f32>) -> Self {
        Pos(val)
    }
}

#[derive(Debug, Clone, Copy, Component, Default, Deref, DerefMut)]
pub struct Orientation(pub Rotation3<f32>);
impl Orientation {
    pub fn from_axis_angle<SB: nalgebra::Storage<f32, nalgebra::Const<3>>>(
        axis: &nalgebra::Unit<nalgebra::Matrix<f32, nalgebra::Const<3>, nalgebra::Const<1>, SB>>,
        angle: f32,
    ) -> Self {
        Self(Rotation3::from_axis_angle(axis, angle))
    }
}
impl From<Rotation3<f32>> for Orientation {
    fn from(val: Rotation3<f32>) -> Self {
        Orientation(val)
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Player;

#[derive(Debug, Clone, Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub loaded_chunks: LoadedChunks,
    pub pos: Pos,
    pub orientation: Orientation,
}

impl PlayerBundle {
    pub fn new() -> Self {
        Self {
            player: Player,
            loaded_chunks: LoadedChunks::new(),
            pos: Pos::default(),
            orientation: Orientation::default(),
        }
    }
}
