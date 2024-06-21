use nalgebra::Matrix4x3;

// this has cost me more than a couple of hours trying to figure out alignment :skull:
// putting transform at the beginning so I don't have to deal with its alignment
// I should probably look into encase (crate)
#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct GridInfo {
    pub transform: Matrix4x3<f32>,
    pub width: u32,
    pub height: u32,
}

unsafe impl bytemuck::Pod for GridInfo {}

impl Default for GridInfo {
    fn default() -> Self {
        Self {
            transform: Matrix4x3::identity(),
            width: 0,
            height: 0,
        }
    }
}
