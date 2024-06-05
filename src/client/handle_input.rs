use std::time::Duration;

use nalgebra::Rotation3;
use winit::{dpi::PhysicalPosition, keyboard::KeyCode as Key, window::CursorGrabMode};

use super::Client;

impl Client<'_> {
    pub fn handle_input(&mut self, dt: &Duration) {
        let dt = dt.as_secs_f32();
        let Client { input, state, .. } = self;
        if input.just_pressed(Key::Escape) {
            if let Some(window) = &self.window {
                self.grabbed_cursor = !self.grabbed_cursor;
                if self.grabbed_cursor {
                    window.set_cursor_visible(false);
                    window
                        .set_cursor_grab(CursorGrabMode::Locked)
                        .map(|_| {
                            self.keep_cursor = false;
                        })
                        .or_else(|_| {
                            self.keep_cursor = true;
                            window.set_cursor_grab(CursorGrabMode::Confined)
                        })
                        .expect("cursor lock");
                } else {
                    self.keep_cursor = false;
                    window.set_cursor_visible(true);
                    window
                        .set_cursor_grab(CursorGrabMode::None)
                        .expect("cursor unlock");
                };
            }
            return;
        }
        if self.grabbed_cursor {
            if let Some(window) = &self.window {
                if self.keep_cursor {
                    let size = window.inner_size();
                    window
                        .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
                        .expect("cursor move");
                }
            }
            let delta = input.mouse_delta * 0.003;
            if delta.x != 0.0 {
                state.camera.orientation = Rotation3::from_axis_angle(&state.camera.up(), delta.x)
                    * state.camera.orientation;
            }
            if delta.y != 0.0 {
                state.camera.orientation =
                    Rotation3::from_axis_angle(&state.camera.right(), delta.y)
                        * state.camera.orientation;
            }
        }
        let rot_dist = 1.0 * dt;
        if input.pressed(Key::KeyQ) {
            state.camera.orientation =
                Rotation3::from_axis_angle(&state.camera.forward(), rot_dist)
                    * state.camera.orientation;
        }
        if input.pressed(Key::KeyE) {
            state.camera.orientation =
                Rotation3::from_axis_angle(&state.camera.forward(), -rot_dist)
                    * state.camera.orientation;
        }
        state.camera.orientation.renormalize();
        if input.scroll_delta != 0.0 {
            state.camera_scroll += input.scroll_delta;
            state.camera.scale = (state.camera_scroll * 0.2).exp();
        }
        let move_dist = 10.0 * dt;

        if input.pressed(Key::KeyW) {
            state.camera.pos += *state.camera.forward() * move_dist;
        }
        if input.pressed(Key::KeyA) {
            state.camera.pos += *state.camera.left() * move_dist;
        }
        if input.pressed(Key::KeyS) {
            state.camera.pos += *state.camera.backward() * move_dist;
        }
        if input.pressed(Key::KeyD) {
            state.camera.pos += *state.camera.right() * move_dist;
        }
        if input.pressed(Key::Space) {
            state.camera.pos += *state.camera.up() * move_dist;
        }
        if input.pressed(Key::ShiftLeft) {
            state.camera.pos += *state.camera.down() * move_dist;
        }
    }
}
