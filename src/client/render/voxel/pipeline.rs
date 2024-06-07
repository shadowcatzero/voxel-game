use nalgebra::{Projective3, Rotation3, Transform3, Translation3, UnitVector3, Vector3};

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
            let mut data = vec![VoxelColor::none(); lx * ly * lz];
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

            let lx2 = 1000;
            let ly2 = 2;
            let lz2 = 1000;
            let offset2 = data.len();
            let mut data2 = vec![VoxelColor::none(); lx2 * ly2 * lz2];
            let paint = VoxelColor {
                r: 255,
                g: 0,
                b: 255,
                a: 255,
            };
            for x in 0..lx2 {
                data2[x + (ly2 - 1) * lx2] = paint;
                data2[x + (ly2 - 1) * lx2 + (lz2 - 1) * lx2 * ly2] = paint;
            }
            for z in 0..lz2 {
                data2[(ly2 - 1) * lx2 + z * lx2 * ly2] = paint;
                data2[lx2 - 1 + (ly2 - 1) * lx2 + z * lx2 * ly2] = paint;
            }
            for x in 0..lx2 {
                for z in 0..lz2 {
                    data2[x + z * lx2 * ly2] = VoxelColor::random();
                }
            }
            data.append(&mut data2);
            let lx3 = 3;
            let ly3 = 3;
            let lz3 = 3;
            let offset3 = data.len();
            data.append(&mut vec![
                VoxelColor {
                    r: 255,
                    g: 0,
                    b: 255,
                    a: 255,
                };
                lx3 * ly3 * lz3
            ]);
            self.voxels.update(
                device,
                encoder,
                belt,
                data.len(),
                &[ArrBufUpdate { offset: 0, data }],
            );
            let proj = Projective3::identity()
                * Translation3::new(0.0, 0.0, 20.0)
                * Rotation3::from_axis_angle(&Vector3::y_axis(), 0.5)
                * Translation3::new(-(lx as f32 / 2.0), -(ly as f32 / 2.0), -(lz as f32 / 2.0));
            let group = VoxelGroup {
                transform: proj,
                transform_inv: proj.inverse(),
                dimensions: Vector3::new(lx as u32, ly as u32, lz as u32),
                offset: 0,
            };
            let proj2 = Projective3::identity()
                * Translation3::new(0.0, -2.1, 20.0)
                * Translation3::new(
                    -(lx2 as f32 / 2.0),
                    -(ly2 as f32 / 2.0),
                    -(lz2 as f32 / 2.0),
                );
            let group2 = VoxelGroup {
                transform: proj2,
                transform_inv: proj2.inverse(),
                dimensions: Vector3::new(lx2 as u32, ly2 as u32, lz2 as u32),
                offset: offset2 as u32,
            };
            let proj3 = Projective3::identity()
                * Translation3::new(0.0, 0.0, 10.0)
                * Rotation3::from_axis_angle(&Vector3::y_axis(), std::f32::consts::PI / 4.0)
                * Rotation3::from_axis_angle(
                    &UnitVector3::new_normalize(Vector3::new(1.0, 0.0, 1.0)),
                    std::f32::consts::PI / 4.0,
                )
                * Translation3::new(
                    -(lx3 as f32 / 2.0),
                    -(ly3 as f32 / 2.0),
                    -(lz3 as f32 / 2.0),
                );
            let group3 = VoxelGroup {
                transform: proj3,
                transform_inv: proj3.inverse(),
                dimensions: Vector3::new(lx3 as u32, ly3 as u32, lz3 as u32),
                offset: offset3 as u32,
            };
            let groups = vec![group, group2, group3];
            self.voxel_groups.update(
                device,
                encoder,
                belt,
                groups.len(),
                &[ArrBufUpdate {
                    offset: 0,
                    data: groups,
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
