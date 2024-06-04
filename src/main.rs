use client::Client;
use winit::event_loop::EventLoop;

mod client;
mod util;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop
        .run_app(&mut Client::new())
        .expect("Failed to run event loop");
}
