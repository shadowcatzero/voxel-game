use std::time::Duration;

use nalgebra::{Rotation3, Vector3};
use winit::{keyboard::KeyCode as Key, window::CursorGrabMode};

use super::Client;

impl Client<'_> {
    pub fn handle_input(&mut self, dt: &Duration) {
        let dt = dt.as_secs_f32();
        let Client { input, state, .. } = self;
        if input.just_pressed(Key::Escape) {
            if let Some(window) = &self.window {
                self.grabbed_cursor = !self.grabbed_cursor;
                let mode = if self.grabbed_cursor {
                    window.set_cursor_visible(false);
                    CursorGrabMode::Locked
                } else {
                    window.set_cursor_visible(true);
                    CursorGrabMode::None
                };
                window.set_cursor_grab(mode).expect("wah");
            }
            return;
        }
        if self.grabbed_cursor {
            let delta = input.mouse_delta;
            if delta.x != 0.0 {
                state.camera.orientation =
                    Rotation3::from_axis_angle(&state.camera.up(), delta.x * 0.003)
                        * state.camera.orientation;
            }
            if delta.y != 0.0 {
                state.camera.orientation =
                    Rotation3::from_axis_angle(&state.camera.right(), delta.y * 0.003)
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
        if input.pressed(Key::KeyZ) {
            state.camera_scroll += dt * 10.0;
            state.camera.scale = (state.camera_scroll * 0.1).exp();
        }
        if input.pressed(Key::KeyX) {
            state.camera_scroll -= dt * 10.0;
            state.camera.scale = (state.camera_scroll * 0.1).exp();
        }
    }
}
