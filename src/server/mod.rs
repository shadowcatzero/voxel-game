mod rsc;

use crate::{
    sync::{client_server_channel, ClientHandle, ServerHandle},
    world::generation::generate,
};
use bevy_ecs::world::World;
use rsc::UPDATE_TIME;
use std::time::{Duration, Instant};

pub struct Server {
    update_time: Duration,
    target: Instant,
    client: ClientHandle,
    world: World,
}

impl Server {
    pub fn new(ch: ClientHandle) -> Self {
        Self {
            client: ch,
            world: World::new(),
            target: Instant::now(),
            update_time: UPDATE_TIME,
        }
    }

    pub fn spawn() -> ServerHandle {
        let (ch, sh) = client_server_channel();
        std::thread::spawn(|| {
            Self::new(ch).run();
        });
        sh
    }

    pub fn run(&mut self) {
        generate(&mut self.world);
        loop {
            self.recv();
            let now = Instant::now();
            if now >= self.target {
                self.target += self.update_time;
            }
        }
    }

    pub fn recv(&mut self) {
        for msg in self.client.recv() {

        }
    }
}
