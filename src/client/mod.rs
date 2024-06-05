mod camera;
mod handle_input;
mod input;
mod render;
mod rsc;
mod state;
mod window;

pub use state::*;

use self::{input::Input, render::Renderer, rsc::FRAME_TIME, ClientState};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::window::Window;

pub struct Client<'a> {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer<'a>>,
    frame_time: Duration,
    state: ClientState,
    exit: bool,
    input: Input,
    target: Instant,
    prev_frame: Instant,
    prev_update: Instant,
    grabbed_cursor: bool,
    keep_cursor: bool,
}

impl Client<'_> {
    pub fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            exit: false,
            frame_time: FRAME_TIME,
            state: ClientState::new(),
            input: Input::new(),
            prev_frame: Instant::now(),
            prev_update: Instant::now(),
            target: Instant::now(),
            grabbed_cursor: false,
            keep_cursor: false,
        }
    }

    pub fn start(&mut self) {}

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = now - self.prev_update;
        self.prev_update = now;

        self.handle_input(&dt);
        self.input.end();

        if now >= self.target {
            self.target += self.frame_time;
            self.prev_frame = now;

            let renderer = self.renderer.as_mut().unwrap();
            renderer.update(&self.state);
            renderer.draw();
        }

        self.exit
    }
}
