use wgpu::TextureFormat;

use super::{group::VoxelGroup, light::GlobalLight, view::View};
use crate::{
    client::render::util::{Storage, StorageTexture, Uniform},
    util::oct_tree::OctNode,
};
use nalgebra::Vector3;

pub struct Layout {
    pub texture: StorageTexture,
    pub view: Uniform<View>,
    pub voxel_groups: Storage<VoxelGroup>,
    pub voxels: Storage<OctNode>,
    pub global_lights: Storage<GlobalLight>,
    render_bind_layout: wgpu::BindGroupLayout,
    compute_bind_layout: wgpu::BindGroupLayout,
    render_pipeline_layout: wgpu::PipelineLayout,
    compute_pipeline_layout: wgpu::PipelineLayout,
    format: TextureFormat,
}

impl Layout {
    pub fn init(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let view = Uniform::init(device, "view", 0);
        let voxels = Storage::init(device, wgpu::ShaderStages::COMPUTE, "voxels", 1);
        let voxel_groups = Storage::init(device, wgpu::ShaderStages::COMPUTE, "voxel groups", 2);
        let global_lights = Storage::init_with(
            device,
            wgpu::ShaderStages::COMPUTE,
            "global lights",
            3,
            &[GlobalLight {
                direction: Vector3::new(-0.5, -4.0, 2.0).normalize(),
            }],
        );
        let texture = StorageTexture::init(
            device,
            wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            "compute output",
            wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            4,
        );
        let render_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    view.bind_group_layout_entry(),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
                    voxels.bind_group_layout_entry(),
                    voxel_groups.bind_group_layout_entry(),
                    global_lights.bind_group_layout_entry(),
                    texture.bind_group_layout_entry(),
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
            voxels,
            voxel_groups,
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
                self.view.bind_group_entry(),
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.texture.sampler),
                },
            ],
            label: Some("tile_bind_group"),
        })
    }
    pub fn compute_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.compute_bind_layout,
            entries: &[
                self.view.bind_group_entry(),
                self.voxels.bind_group_entry(),
                self.voxel_groups.bind_group_entry(),
                self.global_lights.bind_group_entry(),
                self.texture.bind_group_entry(),
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
