use super::camera::Camera;

pub struct ClientState {
    pub camera: Camera,
    pub camera_scroll: f32,
    pub speed: f32,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            camera_scroll: 0.0,
            speed: 0.0,
        }
    }
}
