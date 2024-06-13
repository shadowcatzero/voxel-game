use crate::world::component::VoxelGrid;
use bevy_ecs::world::World;
use nalgebra::{Rotation3, UnitVector3, Vector3};
use ndarray::Array3;

use crate::client::render::voxel::VoxelColor;

use super::component::VoxelGridBundle;

pub fn generate(world: &mut World) {
    let dim = (15, 10, 10);
    world.spawn(VoxelGridBundle {
        pos: Vector3::new(0.0, 0.0, 20.0).into(),
        orientation: Rotation3::from_axis_angle(&Vector3::y_axis(), 0.5).into(),
        grid: VoxelGrid::new(Array3::from_shape_fn(dim, |(x, y, z)| {
            if x == z && x == y {
                VoxelColor::white()
            } else if z == 3 {
                VoxelColor {
                    r: (x as f32 / dim.0 as f32 * 255.0) as u8,
                    g: (y as f32 / dim.1 as f32 * 255.0) as u8,
                    b: 100,
                    a: 255,
                }
            } else if z == 0 {
                VoxelColor {
                    r: (x as f32 / dim.0 as f32 * 255.0) as u8,
                    g: (y as f32 / dim.1 as f32 * 255.0) as u8,
                    b: 0,
                    a: 100,
                }
            } else {
                VoxelColor::none()
            }
        })),
    });

    let dim = (1000, 2, 1000);
    world.spawn(VoxelGridBundle {
        pos: Vector3::new(0.0, -2.1, 0.0).into(),
        orientation: Rotation3::identity().into(),
        grid: VoxelGrid::new(Array3::from_shape_fn(dim, |(x, y, z)| {
            if y == 0 {
                VoxelColor::random()
            } else if (y == dim.1 - 1) && (x == 0 || x == dim.0 - 1 || z == 0 || z == dim.2 - 1) {
                VoxelColor {
                    r: 255,
                    g: 0,
                    b: 255,
                    a: 255,
                }
            } else {
                VoxelColor::none()
            }
        })),
    });

    let dim = (3, 3, 3);
    world.spawn(VoxelGridBundle {
        pos: Vector3::new(0.0, 0.0, 16.5).into(),
        orientation: (Rotation3::from_axis_angle(&Vector3::y_axis(), std::f32::consts::PI / 4.0)
            * Rotation3::from_axis_angle(
                &UnitVector3::new_normalize(Vector3::new(1.0, 0.0, 1.0)),
                std::f32::consts::PI / 4.0,
            ))
        .into(),
        grid: VoxelGrid::new(Array3::from_shape_fn(dim, |(..)| VoxelColor {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        })),
    });
}
