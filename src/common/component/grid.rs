use crate::client::render::voxel::VoxelColor;
use bevy_ecs::{bundle::Bundle, component::Component};
use ndarray::{Array3, ArrayBase, Dim, SliceArg};
use std::ops::Range;

use super::{Orientation, Pos};

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

impl<T> std::ops::Deref for TrackedGrid<T> {
    type Target = Array3<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Bundle, Clone)]
pub struct VoxelGridBundle {
    pub pos: Pos,
    pub orientation: Orientation,
    pub grid: VoxelGrid,
}
