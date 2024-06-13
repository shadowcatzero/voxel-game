use crate::client::camera::Camera;

use super::{voxel::VoxelColor, Renderer};
use nalgebra::{Rotation3, Vector3};
use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};
use winit::{dpi::PhysicalSize, window::Window};

#[derive(Debug, Clone)]
pub enum RenderMessage {
    Resize(PhysicalSize<u32>),
    Draw,
    CreateVoxelGrid(CreateVoxelGrid),
    ViewUpdate(Camera),
    Exit,
}

#[derive(Debug, Clone)]
pub struct CreateVoxelGrid {
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
    pub dimensions: Vector3<usize>,
    pub grid: Vec<VoxelColor>,
}

impl Renderer<'_> {
    pub fn spawn(window: Arc<Window>) -> (RendererChannel, JoinHandle<()>) {
        let (s, mut r) = channel();

        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("Could not create window surface!");
        (
            RendererChannel(s),
            std::thread::spawn(move || {
                Self::new(instance, surface, size).start(&mut r);
            }),
        )
    }

    pub fn start(&mut self, reciever: &mut Receiver<RenderMessage>) {
        let mut new_camera = false;
        'main: loop {
            let now = Instant::now();
            for msg in reciever.try_iter() {
                match msg {
                    RenderMessage::CreateVoxelGrid(desc) => {
                        self.voxel_pipeline.add_group(
                            &self.device,
                            &mut self.encoder,
                            &mut self.staging_belt,
                            desc,
                        );
                    }
                    RenderMessage::Draw => {
                        self.draw();
                    }
                    RenderMessage::Resize(size) => {
                        self.resize(size);
                    }
                    RenderMessage::Exit => {
                        break 'main;
                    }
                    RenderMessage::ViewUpdate(camera) => {
                        new_camera = true;
                        self.camera = camera;
                    }
                }
            }
            if now >= self.target {
                self.target = now + self.frame_time;
                if new_camera {
                    self.voxel_pipeline.update_view(
                        &self.device,
                        &mut self.encoder,
                        &mut self.staging_belt,
                        self.size,
                        &self.camera,
                    );
                    new_camera = false;
                }
                self.draw();
            }
        }
    }
}

#[derive(Clone)]
pub struct RendererChannel(Sender<RenderMessage>);
impl RendererChannel {
    pub fn send(&self, msg: RenderMessage) {
        // TODO: handle this properly
        self.0.send(msg).expect("Failed to send renderer message");
    }
}
