use bevy_ecs::bundle::Bundle;

use crate::world::{component::{Position, Rotation}, grid::TileGrid};

#[derive(Bundle)]
pub struct ClientGrid {
    pub grid: TileGrid,
    pub position: Position,
    pub rotation: Rotation,
}
