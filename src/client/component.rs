use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;

use super::render::RendererChannel;

#[derive(Resource)]
pub struct RenderResource(pub RendererChannel);
impl Deref for RenderResource {
    type Target = RendererChannel;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for RenderResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
