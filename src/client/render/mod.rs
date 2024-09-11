mod command;
mod util;
pub mod voxel;
pub use command::*;

use super::camera::Camera;
use crate::client::rsc::CLEAR_COLOR;
use nalgebra::Vector2;
use util::DepthTexture;
use voxel::VoxelPipeline;
use winit::dpi::PhysicalSize;

pub struct Renderer<'a> {
    size: Vector2<u32>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    config: wgpu::SurfaceConfiguration,
    staging_belt: wgpu::util::StagingBelt,
    voxel_pipeline: VoxelPipeline,
    camera: Camera,
    depth_texture: DepthTexture,
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

        let depth_texture = DepthTexture::init(&device, &config, "depth_texture");

        Self {
            camera: Camera::default(),
            size: Vector2::new(size.width, size.height),
            voxel_pipeline: VoxelPipeline::new(&device, &config),
            staging_belt,
            surface,
            encoder: Self::create_encoder(&device),
            device,
            config,
            queue,
            depth_texture,
        }
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
        self.voxel_pipeline.compute(&mut compute_pass, self.config.width, self.config.height);
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.voxel_pipeline.draw(&mut render_pass);
        drop(render_pass);

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = Vector2::new(size.width, size.height);
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.voxel_pipeline.resize(&self.device, self.size);

        self.depth_texture =
            DepthTexture::init(&self.device, &self.config, "depth_texture");
        self.voxel_pipeline.update_view(
            &self.device,
            &mut self.encoder,
            &mut self.staging_belt,
            self.size,
            &self.camera,
        );
    }
}
