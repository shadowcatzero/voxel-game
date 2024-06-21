use nalgebra::Matrix4;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct View {
    pub transform: Matrix4<f32>,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
}

unsafe impl bytemuck::Pod for View {}

impl Default for View {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            zoom: 1.0,
            transform: Matrix4::identity(),
        }
    }
}
