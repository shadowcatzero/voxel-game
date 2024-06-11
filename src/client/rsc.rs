use std::time::Duration;

pub const FPS: u32 = 60;
pub const FRAME_TIME: Duration = Duration::from_millis(1000 / FPS as u64);

pub const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.1,
    b: 0.1,
    a: 1.0,
};

