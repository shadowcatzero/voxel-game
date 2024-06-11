mod app;
mod camera;
mod component;
mod handle_input;
mod init;
mod input;
pub mod render;
mod rsc;
mod state;
mod system;

pub use app::*;
use component::RenderComponent;
use evenio::world::World;
use init::init_world;
use render::{RenderMessage, RendererChannel};
pub use state::*;

use self::{input::Input, render::Renderer, ClientState};
use std::{sync::Arc, thread::JoinHandle, time::Instant};
use winit::{event::WindowEvent, window::Window};

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
}

impl Client {
    pub fn new(window: Arc<Window>) -> Self {
        let mut world = World::new();

        let (render_channel, render_handle) = Renderer::spawn(window.clone());
        let e = world.spawn();
        world.insert(e, RenderComponent(render_channel.clone()));
        world.add_handler(system::voxel_grid::handle_create_grid);

        init_world(&mut world);
        let state = ClientState::new();
        // render_channel.send(RenderMessage::ViewUpdate(state.camera)).expect("GRRRR");

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
        }
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = now - self.prev_update;
        self.prev_update = now;

        self.handle_input(&dt);
        self.input.end();

        if self.exit {
            self.renderer.send(RenderMessage::Exit).expect("AAAA");
            self.render_handle
                .take()
                .expect("uh oh")
                .join()
                .expect("bruh");
        }
        self.exit
    }

    pub fn window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::Resized(size) => self
                .renderer
                .send(RenderMessage::Resize(size))
                .expect("render broke"),
            WindowEvent::RedrawRequested => self
                .renderer
                .send(RenderMessage::Draw)
                .expect("render broke"),
            WindowEvent::CursorLeft { .. } => {
                self.input.clear();
            }
            _ => self.input.update_window(event),
        }
    }
}
