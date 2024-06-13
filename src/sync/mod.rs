use std::sync::mpsc::{channel, Receiver, Sender, TryIter};

use bevy_ecs::{
    component::{Component, TableStorage},
    entity::Entity,
};

pub enum ServerMessage {}

pub enum ClientMessage {
    LoadWorld(Vec<(Entity, Vec<Box<dyn Component<Storage = TableStorage>>>)>),
}

pub struct ServerHandle {
    send: Sender<ServerMessage>,
    recv: Receiver<ClientMessage>,
}

impl ServerHandle {
    pub fn send(&self, msg: ServerMessage) {
        self.send.send(msg).expect("BOOOOOO");
    }
    pub fn recv(&self) -> TryIter<ClientMessage> {
        self.recv.try_iter()
    }
}

pub struct ClientHandle {
    send: Sender<ClientMessage>,
    recv: Receiver<ServerMessage>,
}

impl ClientHandle {
    pub fn send(&self, msg: ClientMessage) {
        self.send.send(msg).expect("YOU HAVE FAILED THE MISSION");
    }
    pub fn recv(&self) -> TryIter<ServerMessage> {
        self.recv.try_iter()
    }
}

pub fn client_server_channel() -> (ClientHandle, ServerHandle) {
    let (cs, sr) = channel();
    let (ss, cr) = channel();
    let sh = ServerHandle { send: ss, recv: sr };
    let ch = ClientHandle { send: cs, recv: cr };
    (ch, sh)
}
