use super::camera::Camera;

pub struct ClientState {
    pub camera: Camera,
    pub camera_scroll: f32,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            camera_scroll: 0.0,
        }
    }
}
