#![allow(clippy::type_complexity)]

use client::ClientApp;
use winit::event_loop::EventLoop;

mod client;
mod common;
mod server;
mod util;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop
        .run_app(&mut ClientApp::new())
        .expect("Failed to run event loop");
}
