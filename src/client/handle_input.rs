use std::time::Duration;

use nalgebra::Rotation3;
use ndarray::Array3;
use winit::{dpi::PhysicalPosition, keyboard::KeyCode as Key, window::CursorGrabMode};

use crate::{
    common::ServerMessage,
    common::component::{VoxelGrid, VoxelGridBundle},
};

use super::{render::voxel::VoxelColor, Client};

impl Client<'_> {
    pub fn handle_input(&mut self, dt: &Duration) {
        let dt = dt.as_secs_f32();
        let Client {
            input,
            state,
            window,
            ..
        } = self;

        // cursor lock
        if input.just_pressed(Key::Escape) {
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
            return;
        }
        if self.keep_cursor {
            let size = window.inner_size();
            window
                .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
                .expect("cursor move");
        }

        // camera orientation
        let old_camera = state.camera;
        if self.grabbed_cursor {
            let delta = input.mouse_delta * 0.003 / state.camera.scale;
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

        // camera position
        let move_dist = 64.0 * dt;
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
        if state.camera != old_camera {
            self.render_commands
                .push(super::render::RenderCommand::ViewUpdate(state.camera));
        }

        // fun
        if input.just_pressed(Key::KeyF) {
            self.server
                .send(ServerMessage::SpawnVoxelGrid(VoxelGridBundle {
                    pos: (state.camera.pos + 135.0 * 2.0 * *state.camera.forward()).into(),
                    orientation: state.camera.orientation.into(),
                    grid: VoxelGrid::new(Array3::from_shape_fn((135, 135, 135), |(x, y, z)| {
                        if x == 0 || y == 0 || z == 0 || x == 134 || y == 134 || z == 134 {
                            VoxelColor::white()
                        } else {
                            VoxelColor::none()
                        }
                    })),
                }));
        }

        if input.just_pressed(Key::KeyR) {
            self.renderer.update_shader();
        }
        if input.just_pressed(Key::KeyT) {
            self.renderer.reset_shader();
        }
    }
}
