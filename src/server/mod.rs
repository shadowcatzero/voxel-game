mod rsc;
mod system;

use crate::{
    sync::{ClientChannel, ClientMessage, ClientSender, ServerMessage},
    world::{
        component::{Orientation, Pos, Synced, VoxelGrid, VoxelGridBundle},
        generation::generate,
    },
};
use bevy_ecs::{entity::Entity, query::With, system::SystemId, world::World};
use rsc::UPDATE_TIME;
use std::time::{Duration, Instant};

pub struct Server {
    update_time: Duration,
    target: Instant,
    client: ClientChannel,
    world: World,
    systems: ServerSystems,
    mov: Vec<Entity>,
    stop: bool,
}

pub struct ServerSystems {
    sync_pos: SystemId,
}

impl ServerSystems {
    pub fn new(world: &mut World) -> Self {
        Self {
            sync_pos: world.register_system(system::sync::pos),
        }
    }
}

impl Server {
    pub fn new(client: ClientChannel) -> Self {
        let mut world = World::new();
        world.insert_resource(ClientSender(client.sender()));
        let systems = ServerSystems::new(&mut world);
        Self {
            client,
            world,
            systems,
            target: Instant::now(),
            update_time: UPDATE_TIME,
            mov: Vec::new(),
            stop: false,
        }
    }

    pub fn start(ch: ClientChannel) {
        Self::new(ch).run();
    }

    pub fn run(&mut self) {
        generate(&mut self.world);
        loop {
            self.recv();
            let now = Instant::now();
            if now >= self.target {
                self.target += self.update_time;
                let mut q = self.world.query::<(Entity, &mut Pos)>();
                for (e, mut p) in q.iter_mut(&mut self.world) {
                    if self.mov.contains(&e) {
                        p.x += 0.1;
                    }
                }
                self.world.run_system(self.systems.sync_pos).unwrap();
                self.world.clear_trackers();
            }
            if self.stop {
                break;
            }
        }
    }

    pub fn recv(&mut self) {
        for msg in self.client.recv() {
            match msg {
                ServerMessage::LoadWorld => {
                    let mut q = self
                        .world
                        .query_filtered::<(Entity, &Pos, &Orientation, &VoxelGrid), With<Synced>>();
                    // ePOG
                    for (e, p, o, g) in q.iter(&self.world) {
                        self.client.send(ClientMessage::SpawnVoxelGrid(
                            e,
                            VoxelGridBundle {
                                pos: *p,
                                orientation: *o,
                                grid: g.clone(),
                            },
                        ))
                    }
                }
                ServerMessage::SpawnVoxelGrid(grid) => {
                    let e = self.world.spawn((grid.clone(), Synced)).id();
                    self.mov.push(e);
                    self.client.send(ClientMessage::SpawnVoxelGrid(e, grid));
                }
                ServerMessage::Stop => {
                    self.stop = true;
                }
            }
        }
    }
}
