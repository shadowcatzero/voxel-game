use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::Resource;

use super::render::RenderCommand;

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderCommands(pub Vec<RenderCommand>);
