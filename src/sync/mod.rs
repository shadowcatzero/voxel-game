pub mod thread;

use crate::world::component::{Pos, VoxelGridBundle};
use bevy_ecs::{entity::Entity, system::Resource};
use std::sync::mpsc::Sender;
use thread::{ThreadChannel, ThreadHandle};

pub enum ServerMessage {
    Stop,
    LoadWorld,
    SpawnVoxelGrid(VoxelGridBundle),
}

pub enum ClientMessage {
    SpawnVoxelGrid(Entity, VoxelGridBundle),
    PosUpdate(Entity, Pos),
}

pub type ClientChannel = ThreadChannel<ClientMessage, ServerMessage>;
pub type ServerHandle = ThreadHandle<ServerMessage, ClientMessage>;

#[derive(Resource, Clone)]
pub struct ClientSender(pub Sender<ClientMessage>);
impl ClientSender {
    pub fn send(&self, msg: ClientMessage) {
        self.0.send(msg).expect("YOU HAVE FAILED THE MISSION");
    }
}
