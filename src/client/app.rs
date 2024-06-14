use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ControlFlow};

use super::Client;

pub struct ClientApp<'a> {
    client: Option<Client<'a>>,
}

impl <'a> ClientApp<'a> {
    fn client(&mut self) -> &mut Client<'a> {
        self.client.as_mut().expect("bruh")
    }

    pub fn new() -> Self {
        Self { client: None }
    }
}

impl ApplicationHandler for ClientApp<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.client.is_none() {
            self.client = Some(Client::new(event_loop));
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.client().window_event(event);
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.client().input.update_device(event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.client().update(event_loop);
    }
}
