use bevy_ecs::{
    query::{Changed, Or},
    system::{Query, Res},
};
use nalgebra::Vector3;
use ndarray::Axis;

use crate::{
    client::{
        component::RenderResource,
        render::{CreateVoxelGrid, RenderMessage},
    },
    world::component::{Orientation, Pos, VoxelGrid},
};

pub fn update_renderer(
    query: Query<
        (&Pos, &Orientation, &VoxelGrid),
        Or<(Changed<Pos>, Changed<Orientation>, Changed<VoxelGrid>)>,
    >,
    renderer: Res<RenderResource>,
) {
    for (pos, orientation, grid) in query.iter() {
        println!("YAY");
        renderer.send(RenderMessage::CreateVoxelGrid(CreateVoxelGrid {
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
