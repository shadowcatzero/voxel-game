use evenio::{
    event::{EventMut, GlobalEvent, Insert, ReceiverMut, Sender, Spawn},
    fetch::Single,
};
use nalgebra::{Rotation3, Vector3};
use ndarray::Axis;

use crate::{
    client::{
        component::RenderComponent,
        render::{CreateVoxelGrid, RenderMessage},
    },
    world::component::{Orientation, Pos, VoxelGrid},
};

#[derive(GlobalEvent)]
pub struct SpawnVoxelGrid {
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
    pub grid: VoxelGrid,
}

pub fn handle_create_grid(
    r: ReceiverMut<SpawnVoxelGrid>,
    renderer: Single<&RenderComponent>,
    mut s: Sender<(Spawn, Insert<Pos>, Insert<Orientation>, Insert<VoxelGrid>)>,
) {
    let SpawnVoxelGrid {
        pos,
        orientation,
        grid,
    } = EventMut::take(r.event);
    renderer
        .send(RenderMessage::CreateVoxelGrid(CreateVoxelGrid {
            pos,
            orientation,
            dimensions: Vector3::new(
                grid.len_of(Axis(0)),
                grid.len_of(Axis(1)),
                grid.len_of(Axis(2)),
            ),
            grid: grid.iter().cloned().collect(),
        }))
        .expect("render broke");
    let e = s.spawn();
    s.insert(e, Pos(pos));
    s.insert(e, Orientation(orientation));
    s.insert(e, grid);
}
