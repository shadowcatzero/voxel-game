use nalgebra::{Projective3, Vector3};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct VoxelGroup {
    pub transform: Projective3<f32>,
    pub transform_inv: Projective3<f32>,
    pub scale: u32,
    pub offset: u32,
}

unsafe impl bytemuck::Pod for VoxelGroup {}
