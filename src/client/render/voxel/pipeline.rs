use nalgebra::{Rotation3, Transform3, Translation3, Vector3};

use super::{color::VoxelColor, group::VoxelGroup, view::View};
use crate::client::render::{
    buf::ArrBufUpdate, storage::Storage, uniform::Uniform, RenderUpdateData,
};

pub struct VoxelPipeline {
    pipeline: wgpu::RenderPipeline,
    view: Uniform<View>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    voxel_groups: Storage<VoxelGroup>,
    voxels: Storage<VoxelColor>,
    arst: bool,
}

const WIDTH: u32 = 300;
const HEIGHT: u32 = 300;

impl VoxelPipeline {
    pub fn new(device: &wgpu::Device, format: &wgpu::TextureFormat) -> Self {
        // shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tile Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let view = Uniform::<View>::init(device, "view", 0);
        let voxels = Storage::init(device, "voxels", 1);
        let voxel_groups = Storage::init(device, "voxel groups", 2);

        // bind groups
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                view.bind_group_layout_entry(),
                voxels.bind_group_layout_entry(),
                voxel_groups.bind_group_layout_entry(),
            ],
            label: Some("tile_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                view.bind_group_entry(),
                voxels.bind_group_entry(),
                voxel_groups.bind_group_entry(),
            ],
            label: Some("tile_bind_group"),
        });

        // pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Tile Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Voxel Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                    format: *format,
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
        });

        Self {
            pipeline: render_pipeline,
            view,
            bind_group,
            bind_group_layout,
            voxels,
            voxel_groups,
            arst: false,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        update_data: &RenderUpdateData,
    ) {
        if !self.arst {
            let lx = 15;
            let ly = 10;
            let lz = 10;
            let size = lx * ly * lz;
            let mut data = vec![VoxelColor::none(); size];
            for x in 0..lx {
                for y in 0..ly {
                    data[x + y * lx] = VoxelColor {
                        r: (x as f32 / lx as f32 * 255.0) as u8,
                        g: (y as f32 / ly as f32 * 255.0) as u8,
                        b: 0,
                        a: 100,
                    };
                }
            }
            for x in 0..lx {
                for y in 0..ly {
                    data[x + y * lx + 3 * lx * ly] = VoxelColor {
                        r: (x as f32 / lx as f32 * 255.0) as u8,
                        g: (y as f32 / ly as f32 * 255.0) as u8,
                        b: 100,
                        a: 255,
                    };
                }
            }
            for i in 0..lx.min(ly.min(lz)) {
                data[i + i * lx + i * lx * ly] = VoxelColor::white();
            }
            self.voxels.update(
                device,
                encoder,
                belt,
                data.len(),
                &[ArrBufUpdate { offset: 0, data }],
            );
            let thing = Translation3::new(0.0, 0.0, 20.0)
                * Rotation3::from_axis_angle(&Vector3::y_axis(), 0.5)
                * Translation3::new(-(lx as f32 / 2.0), -(ly as f32 / 2.0), -(lz as f32 / 2.0));
            let group = VoxelGroup {
                transform: Transform3::identity() * thing.inverse(),
                dimensions: Vector3::new(lx as u32, ly as u32, lz as u32),
            };
            self.voxel_groups.update(
                device,
                encoder,
                belt,
                1,
                &[ArrBufUpdate {
                    offset: 0,
                    data: vec![group],
                }],
            );
            self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    self.view.bind_group_entry(),
                    self.voxels.bind_group_entry(),
                    self.voxel_groups.bind_group_entry(),
                ],
                label: Some("tile_bind_group"),
            });

            self.arst = true;
        }
        self.view.update(device, encoder, belt, update_data);
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}
