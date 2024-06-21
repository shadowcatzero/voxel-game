use bytemuck::Zeroable;

use super::VoxelColor;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable)]
pub struct VoxelFace {
    pub index: u32,
    pub color: VoxelColor,
}

unsafe impl bytemuck::Pod for VoxelFace {}
