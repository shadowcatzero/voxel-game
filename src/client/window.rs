use std::sync::Arc;

use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ControlFlow,
    window::WindowAttributes,
};

use super::{render::Renderer, Client};

impl ApplicationHandler for Client<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default())
                    .expect("Failed to create window"),
            );
            self.renderer = Some(Renderer::new(window.clone(), false));
            self.window = Some(window);
            self.start();
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::Resized(size) => renderer.resize(size),
            WindowEvent::RedrawRequested => renderer.draw(),
            _ => self.input.update_window(event),
        }
    }

    fn device_event(
            &mut self,
            _event_loop: &winit::event_loop::ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
        self.input.update_device(event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.update() {
            event_loop.exit();
        }
    }
}
