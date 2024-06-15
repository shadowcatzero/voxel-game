use crate::client::render::voxel::VoxelColor;

use super::component::TrackedGrid;

pub struct Chunk {
    grid: TrackedGrid<VoxelColor>
}
