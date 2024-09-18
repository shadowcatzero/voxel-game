mod app;
mod camera;
mod component;
mod handle_input;
mod input;
pub mod render;
mod rsc;
mod state;
mod system;

pub use app::*;
use bevy_ecs::{entity::Entity, system::SystemId, world::World};
use component::RenderCommands;
use render::RenderCommand;
use rsc::FRAME_TIME;
pub use state::*;
use system::render::add_grid;

use crate::{
    common::{ClientMessage, ServerHandle, ServerMessage},
    server::Server,
};

use self::{input::Input, render::Renderer, ClientState};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub struct Client<'a> {
    window: Arc<Window>,
    state: ClientState,
    renderer: Renderer<'a>,
    render_commands: Vec<RenderCommand>,
    exit: bool,
    input: Input,
    prev_update: Instant,
    grabbed_cursor: bool,
    keep_cursor: bool,
    world: World,
    server: ServerHandle,
    server_id_map: HashMap<Entity, Entity>,
    systems: ClientSystems,
    frame_target: Instant,
    frame_time: Duration,
    second_target: Instant,
    the_thing: bool,
}

pub struct ClientSystems {
    render_add_grid: SystemId,
    render_update_transform: SystemId,
    render_add_chunk: SystemId,
}

impl Client<'_> {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let mut world = World::new();
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .expect("Failed to create window"),
        );

        let renderer = Renderer::spawn(window.clone());
        world.insert_resource(RenderCommands(Vec::new()));

        let state = ClientState::new();
        let server = ServerHandle::spawn(Server::start);
        server.send(ServerMessage::Join);

        Self {
            window,
            exit: false,
            renderer,
            render_commands: Vec::new(),
            state,
            input: Input::new(),
            prev_update: Instant::now(),
            grabbed_cursor: false,
            keep_cursor: false,
            systems: ClientSystems {
                render_add_grid: world.register_system(add_grid),
                render_update_transform: world.register_system(system::render::update_transform),
                render_add_chunk: world.register_system(system::render::add_chunk),
            },
            world,
            server,
            server_id_map: HashMap::new(),
            frame_target: Instant::now(),
            frame_time: FRAME_TIME,
            second_target: Instant::now(),
            the_thing: false,
        }
    }

    pub fn update(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.prev_update;
        self.prev_update = now;

        self.handle_input(&dt);
        self.input.end();

        self.recv();
        self.world
            .run_system(self.systems.render_add_grid)
            .expect("WHAT v2");
        self.world
            .run_system(self.systems.render_update_transform)
            .expect("WHAT");
        self.world
            .run_system(self.systems.render_add_chunk)
            .expect("WHAT v3");
        self.world.clear_trackers();

        if self.state.camera.pos.y < -10.0 {
            self.the_thing = !self.the_thing;
            if self.the_thing == true {
                // let thing = include_bytes!("../../../../videos/meme/rab_falls_and_dies.mp4");
            }
        }

        if now >= self.frame_target {
            self.frame_target += self.frame_time;
            let mut commands = std::mem::take(&mut self.render_commands);
            let world_cmds = std::mem::take(&mut self.world.resource_mut::<RenderCommands>().0);
            commands.extend(world_cmds);
            self.renderer.handle_commands(commands);
            self.renderer.draw();
        }

        if now >= self.second_target {
            self.second_target += Duration::from_secs(1);
            // let timer = self.renderer.timer();
            // println!(
            //     "avg: {:4?}; max: {:4?}; fps: {:4?}",
            //     timer.avg(),
            //     timer.max(),
            //     timer.per_sec(),
            // );
        }

        if self.exit {
            event_loop.exit();
        }
    }

    pub fn recv(&mut self) {
        for msg in self.server.recv() {
            match msg {
                ClientMessage::SpawnVoxelGrid(entity, grid) => {
                    let id = self.world.spawn(grid).id();
                    self.server_id_map.insert(entity, id);
                }
                ClientMessage::LoadChunk(entity, chunk) => {
                    let id = self.world.spawn(chunk).id();
                    self.server_id_map.insert(entity, id);
                }
                ClientMessage::PosUpdate(e, pos) => {
                    if let Some(id) = self.server_id_map.get(&e) {
                        self.world.entity_mut(*id).insert(pos);
                    }
                }
            }
        }
    }

    pub fn window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::Resized(size) => self.renderer.resize(size),
            // WindowEvent::RedrawRequested => self.renderer.draw(),
            WindowEvent::CursorLeft { .. } => {
                self.input.clear();
            }
            _ => self.input.update_window(event),
        }
    }
}
