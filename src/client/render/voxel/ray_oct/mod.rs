mod color;
mod grid;
mod group;
mod layout;
mod light;
mod view;

use super::super::UpdateGridTransform;
use crate::{
    client::{
        camera::Camera,
        render::{
            util::{ArrBufUpdate, StorageTexture},
            AddChunk, CreateVoxelGrid,
        },
    },
    common::component::chunk,
};
use bevy_ecs::entity::Entity;
pub use color::*;
use layout::Layout;
use nalgebra::{Projective3, Transform3, Translation3, Vector2, Vector3};
use std::{collections::HashMap, ops::Deref};
use wgpu::include_wgsl;
use {group::VoxelGroup, view::View};

pub struct VoxelPipeline {
    layout: Layout,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    id_map: HashMap<Entity, (usize, VoxelGroup)>,
}

const RENDER_SHADER: wgpu::ShaderModuleDescriptor<'_> = include_wgsl!("shader/render.wgsl");
const COMPUTE_SHADER: wgpu::ShaderModuleDescriptor<'_> = include_wgsl!("shader/compute_working.wgsl");

impl VoxelPipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        // shaders

        let layout = Layout::init(device, config);

        let render_bind_group = layout.render_bind_group(device);
        let shader = device.create_shader_module(RENDER_SHADER);
        let render_pipeline = layout.render_pipeline(device, shader);

        let compute_bind_group = layout.compute_bind_group(device);
        let shader = device.create_shader_module(COMPUTE_SHADER);
        let compute_pipeline = layout.compute_pipeline(device, &shader);

        Self {
            layout,
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            id_map: HashMap::new(),
        }
    }

    pub fn reset_shader(&mut self, device: &wgpu::Device) {
        let shader = device.create_shader_module(COMPUTE_SHADER);
        self.compute_pipeline = self.layout.compute_pipeline(device, &shader);
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
    }

    pub fn update_shader(&mut self, device: &wgpu::Device) {
        let Ok(shader) = std::fs::read_to_string(
            env!("CARGO_MANIFEST_DIR").to_owned() + "/src/client/render/voxel/ray_oct/shader/compute_working.wgsl",
        ) else {
            println!("Failed to reload shader!");
            return;
        };
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        if pollster::block_on(device.pop_error_scope()).is_some() {
            let comp_info = pollster::block_on(shader.get_compilation_info());
            println!("Failed to compile shaders:");
            for msg in comp_info.messages {
                println!("{}", msg.message);
            }
        } else {
            self.compute_pipeline = self.layout.compute_pipeline(device, &shader);
        }
    }

    pub fn add_chunk(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        AddChunk { id, pos, tree, .. }: AddChunk,
    ) {
        let offset = self.layout.voxels.len();

        let data = tree.raw();
        let updates = [ArrBufUpdate { offset, data }];
        let size = offset + data.len();
        self.layout
            .voxels
            .update(device, encoder, belt, size, &updates);

        let proj = Projective3::identity()
            * Translation3::from((pos.deref() * chunk::SIDE_LENGTH as i32).cast())
            * Translation3::from(-chunk::DIMENSIONS.cast() / 2.0);
        let group = VoxelGroup {
            transform: proj,
            transform_inv: proj.inverse(),
            scale: chunk::SCALE,
            offset: offset as u32,
        };
        let updates = [ArrBufUpdate {
            offset: self.layout.voxel_groups.len(),
            data: &[group],
        }];
        let i = self.layout.voxel_groups.len();
        let size = i + 1;
        self.layout
            .voxel_groups
            .update(device, encoder, belt, size, &updates);

        self.id_map.insert(id, (i, group));
        self.compute_bind_group = self.layout.compute_bind_group(device);
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Vector2<u32>) {
        self.layout.texture = StorageTexture::init(
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
        self.compute_bind_group = self.layout.compute_bind_group(device);
        self.render_bind_group = self.layout.render_bind_group(device);
    }

    pub fn update_transform(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        update: UpdateGridTransform,
    ) {
        if let Some((i, group)) = self.id_map.get_mut(&update.id) {
            let offset = Vector3::from_element(-(2u32.pow(group.scale) as f32) / 2.0);
            let proj = Projective3::identity()
                * Translation3::from(update.pos)
                * update.orientation
                * Translation3::from(offset);
            group.transform = proj;
            group.transform_inv = proj.inverse();
            let updates = [ArrBufUpdate {
                offset: *i,
                data: &[*group],
            }];
            let size = self.layout.voxel_groups.len();
            self.layout
                .voxel_groups
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
            zoom: camera.scale,
            transform,
        };
        self.layout.view.update(device, encoder, belt, data);
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }

    pub const WORKGROUP_SIZE: u32 = 8;

    pub fn compute(&self, pass: &mut wgpu::ComputePass) {
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.compute_bind_group, &[]);
        let buf = &self.layout.texture.buf;
        let x = (buf.width() - 1) / Self::WORKGROUP_SIZE + 1;
        let y = (buf.height() - 1) / Self::WORKGROUP_SIZE + 1;
        pass.dispatch_workgroups(x, y, 1);
    }
}
