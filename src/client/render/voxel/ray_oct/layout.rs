use wgpu::{BufferUsages, ShaderStages, TextureFormat};

use super::{chunk::Chunk, light::GlobalLight, view::View};
use crate::{
    client::render::util::{ArrayBuffer, Texture, Uniform},
    util::oct_tree::OctNode,
};
use nalgebra::Vector3;

pub struct Layout {
    pub view: Uniform<View>,
    pub chunks: ArrayBuffer<Chunk>,
    pub voxel_data: ArrayBuffer<OctNode>,
    pub global_lights: ArrayBuffer<GlobalLight>,
    pub texture: Texture,
    render_bind_layout: wgpu::BindGroupLayout,
    compute_bind_layout: wgpu::BindGroupLayout,
    render_pipeline_layout: wgpu::PipelineLayout,
    compute_pipeline_layout: wgpu::PipelineLayout,
    format: TextureFormat,
}

impl Layout {
    pub fn init(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let view = Uniform::init(device, "view", 0);
        let chunks = ArrayBuffer::init(device, "chunks", BufferUsages::STORAGE);
        let voxel_data = ArrayBuffer::init_with(
            device,
            "voxel data",
            BufferUsages::STORAGE,
            &[OctNode::new_leaf(0)],
        );
        let global_lights = ArrayBuffer::init_with(
            device,
            "global lights",
            BufferUsages::STORAGE,
            &[GlobalLight {
                direction: Vector3::new(-1.0, -2.3, 2.0).normalize(),
            }],
        );
        let desc = wgpu::TextureDescriptor {
            label: Some("compute output"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = Texture::init(
            device,
            desc,
            wgpu::TextureViewDescriptor::default(),
            wgpu::SamplerDescriptor::default(),
        );
        let render_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("voxel render"),
            });
        let compute_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    view.bind_group_layout_entry(),
                    chunks.bind_group_layout_entry(
                        1,
                        ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: true },
                    ),
                    voxel_data.bind_group_layout_entry(
                        2,
                        ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: true },
                    ),
                    global_lights.bind_group_layout_entry(
                        3,
                        ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: true },
                    ),
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: texture.format(),
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
                label: Some("voxel compute"),
            });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Tile Pipeline Layout"),
                bind_group_layouts: &[&render_bind_layout],
                push_constant_ranges: &[],
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("voxel compute"),
                bind_group_layouts: &[&compute_bind_layout],
                push_constant_ranges: &[],
            });
        Self {
            view,
            voxel_data,
            chunks,
            global_lights,
            texture,
            render_bind_layout,
            compute_bind_layout,
            render_pipeline_layout,
            compute_pipeline_layout,
            format: config.format,
        }
    }

    pub fn render_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.render_bind_layout,
            entries: &[
                self.texture.view_bind_group_entry(0),
                self.texture.sampler_bind_group_entry(1),
            ],
            label: Some("voxel render"),
        })
    }

    pub fn compute_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.compute_bind_layout,
            entries: &[
                self.view.bind_group_entry(),
                self.chunks.bind_group_entry(1),
                self.voxel_data.bind_group_entry(2),
                self.global_lights.bind_group_entry(3),
                self.texture.view_bind_group_entry(4),
            ],
            label: Some("voxel compute"),
        })
    }

    pub fn render_pipeline(
        &self,
        device: &wgpu::Device,
        shader: wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Voxel Pipeline"),
            layout: Some(&self.render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: true,
            },
            multiview: None,
            cache: None,
        })
    }

    pub fn compute_pipeline(
        &self,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
    ) -> wgpu::ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("voxel"),
            layout: Some(&self.compute_pipeline_layout),
            module: shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        })
    }
}
