use std::ops::Deref;

use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed, Or},
    system::{Query, ResMut},
};
use nalgebra::{AbstractRotation, Rotation3, Vector3};
use ndarray::{Array3, Axis};

use crate::{
    client::{
        component::RenderCommands,
        render::{voxel::VoxelColor, AddChunk, CreateVoxelGrid, RenderCommand, UpdateGridTransform},
    },
    common::component::{ChunkData, ChunkMesh, ChunkPos, Orientation, Pos, VoxelGrid},
};

pub fn add_grid(
    query: Query<
        (Entity, &Pos, &Orientation, &VoxelGrid),
        Or<(Added<Pos>, Added<Orientation>, Added<VoxelGrid>)>,
    >,
    mut renderer: ResMut<RenderCommands>,
) {
    for (id, pos, orientation, grid) in query.iter() {
        let dims = Vector3::new(
            grid.len_of(Axis(0)) + 2,
            grid.len_of(Axis(1)) + 2,
            grid.len_of(Axis(2)) + 2,
        );
        let mut padded = Array3::from_elem((dims.x, dims.y, dims.z), VoxelColor::none());
        padded
            .slice_mut(ndarray::s![1..dims.x - 1, 1..dims.y - 1, 1..dims.z - 1])
            .assign(grid);
        renderer.push(RenderCommand::CreateVoxelGrid(CreateVoxelGrid {
            id,
            pos: **pos,
            orientation: **orientation,
            dimensions: dims,
            grid: padded,
        }));
    }
}

pub fn update_transform(
    query: Query<(Entity, &Pos, &Orientation), Or<(Changed<Pos>, Changed<Orientation>)>>,
    mut renderer: ResMut<RenderCommands>,
) {
    for (id, pos, orientation) in query.iter() {
        renderer.push(RenderCommand::UpdateGridTransform(UpdateGridTransform {
            id,
            pos: **pos,
            orientation: **orientation,
        }));
    }
}

pub fn add_chunk(
    query: Query<(Entity, &ChunkPos, &ChunkMesh, &ChunkData), Or<(Added<ChunkPos>, Added<ChunkMesh>, Added<ChunkData>)>>,
    mut renderer: ResMut<RenderCommands>,
) {
    for (id, pos, mesh, data) in query.iter() {
        renderer.push(RenderCommand::AddChunk(AddChunk {
            id,
            pos: *pos,
            mesh: mesh.clone(),
            tree: data.deref().clone(),
        }));
    }
}
