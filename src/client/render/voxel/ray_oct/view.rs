use nalgebra::Transform3;

use crate::common::component::chunk::SCALE;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct View {
    pub transform: Transform3<f32>,
    pub zoom: f32,
    pub chunk_scale: u32,
    pub chunk_radius: u32,
}

unsafe impl bytemuck::Pod for View {}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            transform: Transform3::identity(),
            chunk_scale: SCALE,
            chunk_radius: 2,
        }
    }
}
