mod command;
mod util;
pub mod voxel;
use std::sync::Arc;

pub use command::*;

use super::camera::Camera;
use crate::client::rsc::CLEAR_COLOR;
use nalgebra::Vector2;
use util::GPUTimer;
use voxel::VoxelPipeline;
use winit::{dpi::PhysicalSize, window::Window};

pub struct Renderer<'a> {
    size: Vector2<u32>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    config: wgpu::SurfaceConfiguration,
    staging_belt: wgpu::util::StagingBelt,
    voxel_pipeline: VoxelPipeline,
    timer: GPUTimer,
    camera: Camera,
}

impl<'a> Renderer<'a> {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("Could not create window surface!");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Could not get adapter!");

        let buf_size = (10f32.powi(9) * 1.5) as u32;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
                required_limits: wgpu::Limits {
                    max_storage_buffer_binding_size: buf_size,
                    max_buffer_size: buf_size as u64,
                    ..Default::default()
                },
                memory_hints: wgpu::MemoryHints::default(),
            },
            None, // Trace path
        ))
        .expect("Could not get device!");

        // TODO: use a logger
        let info = adapter.get_info();
        println!("Adapter: {}", info.name);
        println!("Backend: {:?}", info.backend);

        let surface_caps = surface.get_capabilities(&adapter);
        // Set surface format to srbg
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        // create surface config
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        // not exactly sure what this number should be,
        // doesn't affect performance much and depends on "normal" zoom
        let staging_belt = wgpu::util::StagingBelt::new(4096 * 4);
        let timer = GPUTimer::new(&device, queue.get_timestamp_period(), 1);

        Self {
            camera: Camera::default(),
            size: Vector2::new(size.width, size.height),
            voxel_pipeline: VoxelPipeline::new(&device, &config),
            staging_belt,
            surface,
            encoder: Self::create_encoder(&device),
            timer,
            device,
            config,
            queue,
        }
    }

    pub fn reset_shader(&mut self) {
        self.voxel_pipeline.reset_shader(&self.device);
    }

    pub fn update_shader(&mut self) {
        self.voxel_pipeline.update_shader(&self.device);
    }

    fn create_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        })
    }

    pub fn draw(&mut self) {
        let mut encoder = std::mem::replace(&mut self.encoder, Self::create_encoder(&self.device));
        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        self.timer.start_compute(&mut compute_pass, 0);
        self.voxel_pipeline.compute(&mut compute_pass);
        self.timer.stop_compute(&mut compute_pass, 0);
        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.voxel_pipeline.draw(&mut render_pass);
        drop(render_pass);

        self.timer.resolve(&mut encoder);

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();

        self.timer.finish(&self.device);
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = Vector2::new(size.width, size.height);
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.voxel_pipeline.resize(&self.device, self.size);
        self.voxel_pipeline.update_view(
            &self.device,
            &mut self.encoder,
            &mut self.staging_belt,
            &self.camera,
        );
    }

    pub fn timer(&self) -> &GPUTimer {
        &self.timer
    }
}
