mod color;
mod grid;
mod group;
mod light;
mod view;

pub use color::*;

use super::super::UpdateGridTransform;
use crate::client::{
    camera::Camera,
    render::{
        util::{ArrBufUpdate, Storage, Uniform},
        CreateVoxelGrid,
    },
};
use bevy_ecs::entity::Entity;
use light::GlobalLight;
use nalgebra::{Projective3, Transform3, Translation3, Vector2, Vector3};
use std::collections::HashMap;

use {group::VoxelGroup, view::View};

pub struct VoxelPipeline {
    pipeline: wgpu::RenderPipeline,
    view: Uniform<View>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    voxel_groups: Storage<VoxelGroup>,
    voxels: Storage<VoxelColor>,
    global_lights: Storage<GlobalLight>,
    id_map: HashMap<Entity, (usize, VoxelGroup)>,
}

impl VoxelPipeline {
    pub fn new(device: &wgpu::Device, format: &wgpu::TextureFormat) -> Self {
        // shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tile Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let view = Uniform::init(device, "view", 0);
        let voxels = Storage::init(device, "voxels", 1);
        let voxel_groups = Storage::init(device, "voxel groups", 2);
        let global_lights = Storage::init_with(
            device,
            "global lights",
            3,
            &[GlobalLight {
                direction: Vector3::new(-0.5, -4.0, 2.0).normalize(),
            }],
        );

        // bind groups
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                view.bind_group_layout_entry(),
                voxels.bind_group_layout_entry(),
                voxel_groups.bind_group_layout_entry(),
                global_lights.bind_group_layout_entry(),
            ],
            label: Some("tile_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                view.bind_group_entry(),
                voxels.bind_group_entry(),
                voxel_groups.bind_group_entry(),
                global_lights.bind_group_entry(),
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
            global_lights,
            id_map: HashMap::new(),
        }
    }

    pub fn add_group(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        CreateVoxelGrid {
            id,
            pos,
            orientation,
            dimensions,
            grid,
        }: CreateVoxelGrid,
    ) {
        let offset = self.voxels.len();

        let updates = [ArrBufUpdate {
            offset,
            data: &grid.as_slice().unwrap(),
        }];
        let size = offset + grid.len();
        self.voxels.update(device, encoder, belt, size, &updates);

        let proj = Projective3::identity()
            * Translation3::from(pos)
            * orientation
            * Translation3::from(-dimensions.cast() / 2.0);
        let group = VoxelGroup {
            transform: proj,
            transform_inv: proj.inverse(),
            dimensions: dimensions.cast(),
            offset: offset as u32,
        };
        let updates = [ArrBufUpdate {
            offset: self.voxel_groups.len(),
            data: &[group],
        }];
        let i = self.voxel_groups.len();
        let size = i + 1;
        self.voxel_groups
            .update(device, encoder, belt, size, &updates);

        self.id_map.insert(id, (i, group));

        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                self.view.bind_group_entry(),
                self.voxels.bind_group_entry(),
                self.voxel_groups.bind_group_entry(),
                self.global_lights.bind_group_entry(),
            ],
            label: Some("tile_bind_group"),
        });
    }

    pub fn update_transform(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        update: UpdateGridTransform,
    ) {
        if let Some((i, group)) = self.id_map.get_mut(&update.id) {
            let proj = Projective3::identity()
                * Translation3::from(update.pos)
                * update.orientation
                * Translation3::from(-group.dimensions.cast() / 2.0);
            group.transform = proj;
            group.transform_inv = proj.inverse();
            let updates = [ArrBufUpdate {
                offset: *i,
                data: &[*group],
            }];
            let size = self.voxel_groups.len();
            self.voxel_groups
                .update(device, encoder, belt, size, &updates);
        }
    }

    pub fn update_view(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        size: Vector2<u32>,
        camera: &Camera,
    ) {
        let transform =
            Transform3::identity() * Translation3::from(camera.pos) * camera.orientation;
        let data = View {
            width: size.x,
            height: size.y,
            zoom: camera.scale,
            transform,
        };
        self.view.update(device, encoder, belt, data)
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}
