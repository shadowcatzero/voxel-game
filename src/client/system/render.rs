use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed, Or},
    system::{Query, ResMut},
};
use nalgebra::Vector3;
use ndarray::Axis;

use crate::{
    client::{
        component::RenderCommands,
        render::{CreateVoxelGrid, RenderCommand, UpdateGridTransform},
    },
    world::component::{Orientation, Pos, VoxelGrid},
};

pub fn add_grid(
    query: Query<
        (Entity, &Pos, &Orientation, &VoxelGrid),
        Or<(Added<Pos>, Added<Orientation>, Added<VoxelGrid>)>,
    >,
    mut renderer: ResMut<RenderCommands>,
) {
    for (id, pos, orientation, grid) in query.iter() {
        renderer.push(RenderCommand::CreateVoxelGrid(CreateVoxelGrid {
            id,
            pos: **pos,
            orientation: **orientation,
            dimensions: Vector3::new(
                grid.len_of(Axis(0)),
                grid.len_of(Axis(1)),
                grid.len_of(Axis(2)),
            ),
            grid: grid.iter().cloned().collect(),
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
