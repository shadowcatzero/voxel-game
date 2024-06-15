use crate::client::camera::Camera;

use super::{voxel::VoxelColor, Renderer};
use bevy_ecs::entity::Entity;
use nalgebra::{Rotation3, Vector3};
use ndarray::Array3;
use std::sync::Arc;
use winit::window::Window;

#[derive(Debug, Clone)]
pub enum RenderCommand {
    CreateVoxelGrid(CreateVoxelGrid),
    UpdateGridTransform(UpdateGridTransform),
    ViewUpdate(Camera),
}

#[derive(Debug, Clone)]
pub struct CreateVoxelGrid {
    pub id: Entity,
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
    pub dimensions: Vector3<usize>,
    pub grid: Array3<VoxelColor>,
}

#[derive(Debug, Clone)]
pub struct UpdateGridTransform {
    pub id: Entity,
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
}

impl<'a> Renderer<'a> {
    pub fn spawn(window: Arc<Window>) -> Renderer<'a> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("Could not create window surface!");
        Self::new(instance, surface, size)
    }

    pub fn handle_commands(&mut self, commands: Vec<RenderCommand>) {
        let mut new_camera = false;
        for cmd in commands {
            match cmd {
                RenderCommand::CreateVoxelGrid(desc) => {
                    self.voxel_pipeline.add_group(
                        &self.device,
                        &mut self.encoder,
                        &mut self.staging_belt,
                        desc,
                    );
                }
                RenderCommand::ViewUpdate(camera) => {
                    new_camera = true;
                    self.camera = camera;
                }
                RenderCommand::UpdateGridTransform(update) => {
                    self.voxel_pipeline.update_transform(
                        &self.device,
                        &mut self.encoder,
                        &mut self.staging_belt,
                        update,
                    );
                }
            }
        }
        if new_camera {
            self.voxel_pipeline.update_view(
                &self.device,
                &mut self.encoder,
                &mut self.staging_belt,
                self.size,
                &self.camera,
            );
        }
    }
}
