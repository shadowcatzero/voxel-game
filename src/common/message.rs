use crate::{
    common::component::{ChunkBundle, Pos, VoxelGridBundle},
    util::thread::{ExitType, ThreadChannel, ThreadHandle},
};
use bevy_ecs::entity::Entity;

#[derive(Clone)]
pub enum ServerMessage {
    Stop,
    Join,
    SpawnVoxelGrid(VoxelGridBundle),
}

impl ExitType for ServerMessage {
    fn exit() -> Self {
        ServerMessage::Stop
    }
}

#[derive(Clone)]
pub enum ClientMessage {
    SpawnVoxelGrid(Entity, VoxelGridBundle),
    LoadChunk(Entity, ChunkBundle),
    PosUpdate(Entity, Pos),
}

pub type ClientChannel = ThreadChannel<ClientMessage, ServerMessage>;
pub type ServerHandle = ThreadHandle<ServerMessage, ClientMessage>;
