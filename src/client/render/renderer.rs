use std::sync::Arc;

use super::voxel::VoxelPipeline;
use crate::client::{rsc::CLEAR_COLOR, ClientState};
use winit::{
    dpi::PhysicalSize,
    window::{Fullscreen, Window},
};

pub struct Renderer<'a> {
    size: PhysicalSize<u32>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    encoder: Option<wgpu::CommandEncoder>,
    staging_belt: wgpu::util::StagingBelt,
    voxel_pipeline: VoxelPipeline,
}

impl<'a> Renderer<'a> {
    pub fn new(window: Arc<Window>, fullscreen: bool) -> Self {
        if fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

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

        Self {
            size,
            voxel_pipeline: VoxelPipeline::new(&device, &config.format),
            encoder: None,
            staging_belt,
            surface,
            device,
            adapter,
            config,
            queue,
        }
    }

    fn create_encoder(&mut self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    pub fn draw(&mut self) {
        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.encoder.take().unwrap_or(self.create_encoder());
        {
            let render_pass = &mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            self.voxel_pipeline.draw(render_pass);
        }

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();
    }

    pub fn update(&mut self, state: &ClientState) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        self.voxel_pipeline.update(
            &self.device,
            &mut encoder,
            &mut self.staging_belt,
            &RenderUpdateData {
                state,
                size: &self.size,
            },
        );
        self.encoder = Some(encoder);
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn size(&self) -> &PhysicalSize<u32> {
        &self.size
    }
}

pub struct RenderUpdateData<'a> {
    pub state: &'a ClientState,
    pub size: &'a PhysicalSize<u32>,
}
