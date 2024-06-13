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
use bevy_ecs::{system::SystemId, world::World};
use component::RenderResource;
use render::{RenderMessage, RendererChannel};
pub use state::*;
use system::voxel_grid::update_renderer;

use crate::{server::Server, sync::ServerHandle, world::generation::generate};

use self::{input::Input, render::Renderer, ClientState};
use std::{sync::Arc, thread::JoinHandle, time::Instant};
use winit::{
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub struct Client {
    window: Arc<Window>,
    state: ClientState,
    render_handle: Option<JoinHandle<()>>,
    renderer: RendererChannel,
    exit: bool,
    input: Input,
    prev_update: Instant,
    grabbed_cursor: bool,
    keep_cursor: bool,
    world: World,
    server: ServerHandle,
    bruh: SystemId,
}

impl Client {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let mut world = World::new();
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .expect("Failed to create window"),
        );

        let (render_channel, render_handle) = Renderer::spawn(window.clone());
        world.insert_resource(RenderResource(render_channel.clone()));
        let bruh = world.register_system(update_renderer);

        let state = ClientState::new();
        let server = Server::spawn();
        generate(&mut world);

        Self {
            window,
            exit: false,
            render_handle: Some(render_handle),
            renderer: render_channel,
            state,
            input: Input::new(),
            prev_update: Instant::now(),
            grabbed_cursor: false,
            keep_cursor: false,
            world,
            server,
            bruh,
        }
    }

    pub fn update(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.prev_update;
        self.prev_update = now;

        self.handle_input(&dt);
        self.input.end();

        self.recv();
        self.world.run_system(self.bruh).expect("WHAT");
        self.world.clear_trackers();

        if self.exit {
            self.renderer.send(RenderMessage::Exit);
            // you know I'd like to do a timeout here...
            // only because I have an NVIDIA GPU HELP
            self.render_handle
                .take()
                .expect("uh oh")
                .join()
                .expect("bruh");
            event_loop.exit();
        }
    }

    pub fn recv(&mut self) {
        for msg in self.server.recv() {}
    }

    pub fn window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::Resized(size) => self.renderer.send(RenderMessage::Resize(size)),
            WindowEvent::RedrawRequested => self.renderer.send(RenderMessage::Draw),
            WindowEvent::CursorLeft { .. } => {
                self.input.clear();
            }
            _ => self.input.update_window(event),
        }
    }
}
