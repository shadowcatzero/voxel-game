mod color;
mod grid;
mod group;
mod light;
mod view;

pub use color::*;
use wgpu::include_wgsl;

use super::super::UpdateGridTransform;
use crate::{
    client::{
        camera::Camera,
        render::{
            util::{ArrBufUpdate, DepthTexture, Storage, StorageTexture, Uniform},
            AddChunk, CreateVoxelGrid,
        },
    },
    common::component::chunk,
    util::oct_tree::OctNode,
};
use bevy_ecs::entity::Entity;
use light::GlobalLight;
use nalgebra::{Projective3, Transform3, Translation3, Vector2, Vector3};
use std::{collections::HashMap, ops::Deref};

use {group::VoxelGroup, view::View};

pub struct VoxelPipeline {
    compute_pipeline: wgpu::ComputePipeline,
    texture: StorageTexture,
    cbind_group_layout: wgpu::BindGroupLayout,
    cbind_group: wgpu::BindGroup,

    pipeline: wgpu::RenderPipeline,
    view: Uniform<View>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    voxel_groups: Storage<VoxelGroup>,
    voxels: Storage<OctNode>,
    global_lights: Storage<GlobalLight>,
    id_map: HashMap<Entity, (usize, VoxelGroup)>,
}

impl VoxelPipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        // shaders
        let shader = device.create_shader_module(include_wgsl!("render.wgsl"));

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
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            "idk man im tired",
            wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            4,
        );

        // bind groups
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("tile_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                view.bind_group_entry(),
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
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
                    format: config.format,
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
        });

        let cbind_group_layout =
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

        let cbind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &cbind_group_layout,
            entries: &[
                view.bind_group_entry(),
                voxels.bind_group_entry(),
                voxel_groups.bind_group_entry(),
                global_lights.bind_group_entry(),
                texture.bind_group_entry(),
            ],
            label: Some("voxel compute"),
        });

        let cpipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("voxel compute"),
            bind_group_layouts: &[&cbind_group_layout],
            push_constant_ranges: &[],
        });

        let cs_module = device.create_shader_module(include_wgsl!("compute.wgsl"));
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("voxel"),
            layout: Some(&cpipeline_layout),
            module: &cs_module,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            compute_pipeline,
            texture,
            cbind_group_layout,
            cbind_group,
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
        // let offset = self.voxels.len();
        //
        // let updates = [ArrBufUpdate {
        //     offset,
        //     data: &grid.as_slice().unwrap(),
        // }];
        // let size = offset + grid.len();
        // self.voxels.update(device, encoder, belt, size, &updates);
        //
        // let proj = Projective3::identity()
        //     * Translation3::from(pos)
        //     * orientation
        //     * Translation3::from(-dimensions.cast() / 2.0);
        // let group = VoxelGroup {
        //     transform: proj,
        //     transform_inv: proj.inverse(),
        //     dimensions: dimensions.cast(),
        //     offset: offset as u32,
        // };
        // let updates = [ArrBufUpdate {
        //     offset: self.voxel_groups.len(),
        //     data: &[group],
        // }];
        // let i = self.voxel_groups.len();
        // let size = i + 1;
        // self.voxel_groups
        //     .update(device, encoder, belt, size, &updates);
        //
        // self.id_map.insert(id, (i, group));
        //
        // self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &self.bind_group_layout,
        //     entries: &[
        //         self.view.bind_group_entry(),
        //         self.voxels.bind_group_entry(),
        //         self.voxel_groups.bind_group_entry(),
        //         self.global_lights.bind_group_entry(),
        //     ],
        //     label: Some("tile_bind_group"),
        // });
    }

    pub fn add_chunk(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        AddChunk { id, pos, tree, .. }: AddChunk,
    ) {
        let offset = self.voxels.len();

        let data = tree.raw();
        let updates = [ArrBufUpdate { offset, data }];
        let size = offset + data.len();
        self.voxels.update(device, encoder, belt, size, &updates);

        let proj = Projective3::identity()
            * Translation3::from((pos.deref() * chunk::SIDE_LENGTH as i32).cast())
            * Translation3::from(-chunk::DIMENSIONS.cast() / 2.0);
        let group = VoxelGroup {
            transform: proj,
            transform_inv: proj.inverse(),
            dimensions: chunk::DIMENSIONS.cast(),
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
        self.update_cbind_group(device);
    }

    pub fn update_cbind_group(&mut self, device: &wgpu::Device) {
        self.cbind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.cbind_group_layout,
            entries: &[
                self.view.bind_group_entry(),
                self.voxels.bind_group_entry(),
                self.voxel_groups.bind_group_entry(),
                self.global_lights.bind_group_entry(),
                self.texture.bind_group_entry(),
            ],
            label: Some("tile_bind_group"),
        });
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Vector2<u32>) {
        self.texture = StorageTexture::init(
            device,
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            "idk man im tired",
            wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            4,
        );
        self.update_cbind_group(device);
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
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

    pub const WORKGROUP_SIZE: u32 = 8;

    pub fn compute(&self, pass: &mut wgpu::ComputePass) {
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.cbind_group, &[]);
        let buf = &self.texture.buf;
        let x = (buf.width() - 1) / Self::WORKGROUP_SIZE + 1;
        let y = (buf.height() - 1) / Self::WORKGROUP_SIZE + 1;
        pass.dispatch_workgroups(x, y, 1);
    }
}
