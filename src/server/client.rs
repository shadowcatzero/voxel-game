use std::collections::{hash_map, HashMap};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, entity::Entity, system::Resource};

use crate::common::{ClientChannel, ClientMessage, ServerMessage};

pub enum ServerClient {
    Local(ClientChannel),
}

impl ServerClient {
    pub fn recv(&mut self) -> Vec<ServerMessage> {
        match self {
            Self::Local(ch) => ch.recv().collect(),
        }
    }
    pub fn send(&self, msg: ClientMessage) {
        match self {
            Self::Local(ch) => ch.send(msg),
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct ServerClients {
    map: HashMap<Entity, ServerClient>,
}

impl ServerClients {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn add(&mut self, id: Entity, client: ServerClient) {
        self.map.insert(id, client);
    }
}

impl<'a> IntoIterator for &'a mut ServerClients {
    type Item = (&'a Entity, &'a mut ServerClient);
    type IntoIter = hash_map::IterMut<'a, Entity, ServerClient>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}

// I don't think it's worth putting a reciever in here rn
// and moving that stuff into the ecs but we'll see
#[derive(Component)]
pub struct ClientComponent {
    send: Vec<ClientMessage>,
}

impl ClientComponent {
    pub fn new() -> Self {
        Self { send: Vec::new() }
    }
    pub fn send(&mut self, msg: ClientMessage) {
        self.send.push(msg);
    }
    pub fn take(&mut self) -> Vec<ClientMessage> {
        std::mem::take(&mut self.send)
    }
}

#[derive(Resource)]
pub struct ClientBroadcast(Vec<ClientMessage>);
impl ClientBroadcast {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn send(&mut self, msg: ClientMessage) {
        self.0.push(msg);
    }
    pub fn take(&mut self) -> Vec<ClientMessage> {
        std::mem::take(&mut self.0)
    }
}
