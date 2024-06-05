use nalgebra::{Transform3, Vector3};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct VoxelGroup {
    pub transform: Transform3<f32>,
    pub dimensions: Vector3<u32>,
    pub offset: u32,
}

unsafe impl bytemuck::Pod for VoxelGroup {}
