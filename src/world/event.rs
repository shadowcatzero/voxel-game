use nalgebra::{Rotation3, Vector3};

use super::component::VoxelGrid;

#[derive(GlobalEvent)]
pub struct SpawnVoxelGrid {
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
    pub grid: VoxelGrid,
}

