mod thread;
mod util;
pub mod voxel;

pub use thread::*;

use super::camera::Camera;
use crate::client::rsc::{CLEAR_COLOR, FRAME_TIME};
use nalgebra::Vector2;
use smaa::{SmaaMode, SmaaTarget};
use std::time::{Duration, Instant};
use voxel::VoxelPipeline;
use winit::dpi::PhysicalSize;

pub struct Renderer<'a> {
    size: Vector2<u32>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    staging_belt: wgpu::util::StagingBelt,
    voxel_pipeline: VoxelPipeline,
    smaa_target: SmaaTarget,
    camera: Camera,

    frame_time: Duration,
    target: Instant,
}

impl<'a> Renderer<'a> {
    pub fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'a>,
        size: PhysicalSize<u32>,
    ) -> Self {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Could not get adapter!");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
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
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        // not exactly sure what this number should be,
        // doesn't affect performance much and depends on "normal" zoom
        let staging_belt = wgpu::util::StagingBelt::new(4096 * 4);

        let smaa_target = SmaaTarget::new(
            &device,
            &queue,
            size.width,
            size.height,
            surface_format,
            SmaaMode::Smaa1X,
        );

        Self {
            camera: Camera::default(),
            size: Vector2::new(size.width, size.height),
            voxel_pipeline: VoxelPipeline::new(&device, &config.format),
            staging_belt,
            surface,
            device,
            config,
            queue,
            smaa_target,
            frame_time: FRAME_TIME,
            target: Instant::now(),
        }
    }

    fn create_encoder(&mut self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut encoder = std::mem::replace(encoder, self.create_encoder());
        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let smaa_frame = self
            .smaa_target
            .start_frame(&self.device, &self.queue, &view);
        {
            let render_pass = &mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &smaa_frame,
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
            self.voxel_pipeline.draw(render_pass);
        }
        smaa_frame.resolve();

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>, encoder: &mut wgpu::CommandEncoder) {
        self.size = Vector2::new(size.width, size.height);
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.smaa_target
            .resize(&self.device, size.width, size.height);

        self.voxel_pipeline.update_view(
            &self.device,
            encoder,
            &mut self.staging_belt,
            self.size,
            &self.camera,
        );
    }
}
