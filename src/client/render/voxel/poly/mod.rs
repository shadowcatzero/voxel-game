mod color;
mod face;
mod group;
mod light;
mod view;

use block_mesh::{ndshape::RuntimeShape, UnitQuadBuffer, RIGHT_HANDED_Y_UP_CONFIG};
pub use color::*;
pub use face::*;

use group::FaceGroup;
use light::GlobalLight;
use nalgebra::{Perspective3, Transform3, Translation3, Vector2, Vector3};
use view::View;
use wgpu::{SurfaceConfiguration, VertexAttribute, VertexFormat};

use crate::{
    client::{camera::Camera, render::AddChunk},
    common::component::{chunk, ChunkData},
};

use super::super::{
    util::{Instances, Storage, Texture, Uniform},
    CreateVoxelGrid, UpdateGridTransform,
};

pub struct VoxelPipeline {
    pipeline: wgpu::RenderPipeline,
    view: Uniform<View>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_groups: Vec<wgpu::BindGroup>,
    vertices: Vec<Instances<VoxelFace>>,
    global_lights: Storage<GlobalLight>,
}

const INSTANCE_ATTRS: [wgpu::VertexAttribute; 2] = [
    VertexAttribute {
        format: VertexFormat::Uint32,
        offset: 0,
        shader_location: 0,
    },
    VertexAttribute {
        format: VertexFormat::Uint32,
        offset: 4,
        shader_location: 1,
    },
];

impl VoxelPipeline {
    pub fn new(device: &wgpu::Device, config: &SurfaceConfiguration) -> Self {
        // shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tile Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let view = Uniform::<View>::init(device, "view", 0);
        let example_faces =
            Instances::<VoxelFace>::init(device, "voxel groups", 0, &INSTANCE_ATTRS);
        let example_group = Uniform::<FaceGroup>::init(device, "voxel group", 1);
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
                example_group.bind_group_layout_entry(),
                global_lights.bind_group_layout_entry(),
            ],
            label: Some("tile_bind_group_layout"),
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
                buffers: &[example_faces.desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
            bind_group_layout,
            bind_groups: Vec::new(),
            vertices: Vec::new(),
            global_lights,
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
        let mut transform = (Translation3::from(camera.pos) * camera.orientation)
            .inverse()
            .to_matrix();
        transform = transform.append_nonuniform_scaling(&Vector3::new(1.0, 1.0, -1.0));
        let projection = Perspective3::new(
            size.x as f32 / size.y as f32,
            std::f32::consts::PI / 2.0,
            0.1,
            10000.0,
        );
        transform = projection.as_matrix() * transform;
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
        for i in 0..self.bind_groups.len() {
            render_pass.set_bind_group(0, &self.bind_groups[i], &[]);
            let vertices = &self.vertices[i];
            vertices.set_in(render_pass);
            render_pass.draw(0..4, 0..vertices.len() as u32);
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
        let proj = Transform3::identity()
            * Translation3::from(pos)
            * orientation
            * Translation3::from(-dimensions.cast() / 2.0);

        let mut buffer = UnitQuadBuffer::new();
        let dim: Vector3<u32> = dimensions.cast();
        let dim = Vector3::new(dim.z, dim.y, dim.x);
        let shape = RuntimeShape::<u32, 3>::new(dim.into());
        let slice = grid.as_slice().unwrap();
        block_mesh::visible_block_faces(
            slice,
            &shape,
            [0; 3],
            (dim - Vector3::new(1, 1, 1)).into(),
            &RIGHT_HANDED_Y_UP_CONFIG.faces,
            &mut buffer,
        );
        for (face, group) in buffer.groups.iter().enumerate() {
            let data: Vec<VoxelFace> = group
                .iter()
                .map(|a| {
                    let i = a.minimum[0] + a.minimum[1] * dim.y + a.minimum[2] * dim.y * dim.x;
                    VoxelFace {
                        index: i,
                        color: slice[i as usize],
                    }
                })
                .collect();
            self.vertices.push(Instances::init_with(
                device,
                "vvvvv",
                0,
                &INSTANCE_ATTRS,
                &data,
            ));
            let group = FaceGroup {
                dimensions: dimensions.cast(),
                transform: proj,
                face: ((8 - face) % 6) as u32,
            };
            let uniform = Uniform::init_with(device, "voxel group", 1, &[group]);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    self.view.bind_group_entry(),
                    uniform.bind_group_entry(),
                    self.global_lights.bind_group_entry(),
                ],
                label: Some("voxel bind group"),
            });
            self.bind_groups.push(bind_group);
        }
    }

    pub fn add_chunk(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        AddChunk { id, pos, mesh }: AddChunk,
    ) {
        if mesh.faces.iter().all(|f| f.is_empty()) {
            return;
        }

        let proj = Transform3::identity()
            * Translation3::from(pos.cast() * crate::common::component::chunk::SIDE_LENGTH as f32)
            * Translation3::from(-chunk::DIMENSIONS.cast() / 2.0);

        for (face, meshes) in mesh.faces.iter().enumerate() {
            self.vertices.push(Instances::init_with(
                device,
                "vvvvv",
                0,
                &INSTANCE_ATTRS,
                meshes,
            ));
            let group = FaceGroup {
                dimensions: chunk::DIMENSIONS.cast(),
                transform: proj,
                face: face as u32,
            };
            let uniform = Uniform::init_with(device, "voxel group", 1, &[group]);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    self.view.bind_group_entry(),
                    uniform.bind_group_entry(),
                    self.global_lights.bind_group_entry(),
                ],
                label: Some("voxel bind group"),
            });
            self.bind_groups.push(bind_group);
        }
    }

    pub fn update_transform(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut wgpu::util::StagingBelt,
        update: UpdateGridTransform,
    ) {
    }
}
