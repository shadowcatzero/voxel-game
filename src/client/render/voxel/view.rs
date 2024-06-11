use nalgebra::Transform3;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct View {
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub padding: u32,
    pub transform: Transform3<f32>,
}

unsafe impl bytemuck::Pod for View {}

impl Default for View {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            zoom: 1.0,
            padding: 0,
            transform: Transform3::identity(),
        }
    }
}
