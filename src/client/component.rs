use std::ops::{Deref, DerefMut};

use evenio::component::Component;

use super::render::RendererChannel;

#[derive(Component)]
pub struct RenderComponent(pub RendererChannel);
impl Deref for RenderComponent {
    type Target = RendererChannel;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for RenderComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
