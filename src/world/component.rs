use std::ops::Range;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{bundle::Bundle, component::Component};
use nalgebra::{Rotation3, Vector3};
use ndarray::{Array3, ArrayBase, Dim, SliceArg};

use crate::client::render::voxel::VoxelColor;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct Synced;

#[derive(Debug, Clone, Copy, Component, Default, Deref, DerefMut)]
pub struct Pos(pub Vector3<f32>);
#[derive(Debug, Clone, Copy, Component, Default, Deref, DerefMut)]
pub struct Orientation(pub Rotation3<f32>);

pub type VoxelGrid = TrackedGrid<VoxelColor>;
pub type GridRegion = (Range<usize>, Range<usize>, Range<usize>);
#[derive(Debug, Clone, Component)]
pub struct TrackedGrid<T> {
    data: Array3<T>,
    changes: Vec<GridRegion>,
}

impl<T> TrackedGrid<T> {
    pub fn new(data: Array3<T>) -> Self {
        Self {
            data,
            changes: Vec::new(),
        }
    }
    pub fn view_slice_mut<I: SliceArg<Dim<[usize; 3]>>>(
        &mut self,
        slice: I,
    ) -> ArrayBase<ndarray::ViewRepr<&mut T>, <I as SliceArg<Dim<[usize; 3]>>>::OutDim> {
        self.data.slice_mut(slice)
    }
    pub fn take_changes(&mut self) -> Vec<GridRegion> {
        std::mem::take(&mut self.changes)
    }
}

impl Pos {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vector3::new(x, y, z))
    }
}
impl Orientation {
    pub fn from_axis_angle<SB: nalgebra::Storage<f32, nalgebra::Const<3>>>(
        axis: &nalgebra::Unit<nalgebra::Matrix<f32, nalgebra::Const<3>, nalgebra::Const<1>, SB>>,
        angle: f32,
    ) -> Self {
        Self(Rotation3::from_axis_angle(axis, angle))
    }
}
impl From<Vector3<f32>> for Pos {
    fn from(val: Vector3<f32>) -> Self {
        Pos(val)
    }
}
impl From<Rotation3<f32>> for Orientation {
    fn from(val: Rotation3<f32>) -> Self {
        Orientation(val)
    }
}

#[derive(Bundle, Clone)]
pub struct VoxelGridBundle {
    pub pos: Pos,
    pub orientation: Orientation,
    pub grid: VoxelGrid,
}

impl<T> std::ops::Deref for TrackedGrid<T> {
    type Target = Array3<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
