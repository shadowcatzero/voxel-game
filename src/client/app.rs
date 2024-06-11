use std::sync::Arc;

use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ControlFlow,
    window::WindowAttributes,
};

use super::Client;

pub struct ClientApp {
    client: Option<Client>,
}

impl ClientApp {
    fn client(&mut self) -> &mut Client {
        self.client.as_mut().expect("bruh")
    }

    pub fn new() -> Self {
        Self { client: None }
    }
}

impl ApplicationHandler for ClientApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.client.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default())
                    .expect("Failed to create window"),
            );
            let client = Client::new(window);
            self.client = Some(client);
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
        if self.client().update() {
            event_loop.exit();
        }
    }
}
