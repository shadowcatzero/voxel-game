use nalgebra::{Transform3, Translation3};

use crate::client::render::uniform::UniformData;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct View {
    pub transform: Transform3<f32>,
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
            transform: Transform3::identity(),
        }
    }
}

impl UniformData for View {
    fn update(&mut self, data: &crate::client::render::RenderUpdateData) -> bool {
        let camera = data.state.camera;
        let new = Transform3::identity() * Translation3::from(camera.pos) * camera.orientation;
        if new == self.transform
            && data.size.width == self.width
            && data.size.height == self.height
            && camera.scale == self.zoom
        {
            false
        } else {
            *self = Self {
                width: data.size.width,
                height: data.size.height,
                zoom: camera.scale,
                transform: new,
            };
            true
        }
    }
}
