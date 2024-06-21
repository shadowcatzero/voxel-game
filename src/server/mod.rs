mod chunk;
mod client;
mod rsc;
mod system;
mod test;

pub use client::*;

use crate::common::{
    component::{
        ChunkBundle, ChunkData, ChunkMap, ChunkMesh, ChunkPos, Orientation, PlayerBundle, Pos,
        VoxelGrid, VoxelGridBundle,
    },
    ClientChannel, ClientMessage, ServerMessage,
};
use bevy_ecs::{entity::Entity, system::SystemId, world::World};
use chunk::ChunkManager;
use client::{ClientBroadcast, ServerClient, ServerClients};
use rsc::UPDATE_TIME;
use std::time::{Duration, Instant};
use test::spawn_test_stuff;

pub struct Server {
    update_time: Duration,
    target: Instant,
    clients: ServerClients,
    world: World,
    systems: ServerSystems,
    mov: Vec<Entity>,
    stop: bool,
}

pub struct ServerSystems {
    sync_pos: SystemId,
    sync_chunks: SystemId,
}

impl ServerSystems {
    pub fn new(world: &mut World) -> Self {
        Self {
            sync_pos: world.register_system(system::sync::pos),
            sync_chunks: world.register_system(system::sync::chunks),
        }
    }
}

impl Server {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(ClientBroadcast::new());
        world.insert_resource(ChunkMap::new());
        world.insert_non_send_resource(ChunkManager::new());
        let systems = ServerSystems::new(&mut world);
        Self {
            clients: ServerClients::new(),
            world,
            systems,
            target: Instant::now(),
            update_time: UPDATE_TIME,
            mov: Vec::new(),
            stop: false,
        }
    }

    pub fn from_client(client: ClientChannel) -> Self {
        let mut s = Self::new();
        s.add_client(ServerClient::Local(client));
        s
    }

    pub fn add_client(&mut self, client: ServerClient) {
        let id = self.world.spawn(ClientComponent::new()).id();
        self.clients.add(id, client);
    }

    pub fn start(ch: ClientChannel) {
        Self::from_client(ch).run();
    }

    pub fn run(&mut self) {
        spawn_test_stuff(&mut self.world);
        loop {
            self.recv();
            let now = Instant::now();
            if now >= self.target {
                self.target += self.update_time;
                self.tick();
            }
            if self.stop {
                break;
            }
            self.send();
        }
    }

    pub fn tick(&mut self) {
        let mut q = self.world.query::<(Entity, &mut Pos)>();
        for (e, mut p) in q.iter_mut(&mut self.world) {
            if self.mov.contains(&e) {
                p.x += 0.1;
            }
        }
        self.world.run_system(self.systems.sync_pos).unwrap();
        self.world.run_system(self.systems.sync_chunks).unwrap();
        self.world.clear_trackers();
    }

    pub fn recv(&mut self) {
        for (id, client) in &mut self.clients {
            for msg in client.recv() {
                match msg {
                    ServerMessage::Join => {
                        let mut q = self
                            .world
                            .query::<(Entity, &Pos, &Orientation, &VoxelGrid)>();
                        // ePOG
                        for (e, p, o, g) in q.iter(&self.world) {
                            client.send(ClientMessage::SpawnVoxelGrid(
                                e,
                                VoxelGridBundle {
                                    pos: *p,
                                    orientation: *o,
                                    grid: g.clone(),
                                },
                            ))
                        }
                        let mut q = self
                            .world
                            .query::<(Entity, &ChunkPos, &ChunkData, &ChunkMesh)>();
                        for (e, p, c, m) in q.iter(&self.world) {
                            client.send(ClientMessage::LoadChunk(
                                e,
                                ChunkBundle {
                                    pos: *p,
                                    data: c.clone(),
                                    mesh: m.clone(),
                                },
                            ))
                        }
                        self.world.entity_mut(*id).insert(PlayerBundle::new());
                    }
                    ServerMessage::SpawnVoxelGrid(grid) => {
                        let e = self.world.spawn(grid.clone()).id();
                        self.mov.push(e);
                        self.world
                            .resource_mut::<ClientBroadcast>()
                            .send(ClientMessage::SpawnVoxelGrid(e, grid));
                    }
                    ServerMessage::Stop => {
                        self.stop = true;
                    }
                }
            }
        }
    }

    pub fn send(&mut self) {
        let msgs = self.world.resource_mut::<ClientBroadcast>().take();
        for msg in &msgs {
            for (_, client) in &mut self.clients {
                client.send(msg.clone());
            }
        }
        let mut q = self.world.query::<(Entity, &mut ClientComponent)>();
        for (e, mut c) in q.iter_mut(&mut self.world) {
            if let Some(sc) = self.clients.get(&e) {
                for msg in c.take() {
                    sc.send(msg);
                }
            }
        }
    }
}
