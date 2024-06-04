use crate::util::math::Vec2us;
use bevy_ecs::component::Component;
use nalgebra::{DMatrix, DimRange, Dyn};
use std::ops::{Deref, Range};

pub type GridRegion = (Range<usize>, Range<usize>);
pub type GridView<'a, T> = nalgebra::Matrix<
    T,
    Dyn,
    Dyn,
    nalgebra::ViewStorageMut<'a, T, Dyn, Dyn, nalgebra::Const<1>, Dyn>,
>;

#[derive(Clone, Component)]
pub struct TrackedGrid<T> {
    data: DMatrix<T>,
    changes: Vec<GridRegion>,
}

impl<T> TrackedGrid<T> {
    pub fn new(data: DMatrix<T>) -> Self {
        Self {
            data,
            changes: Vec::new(),
        }
    }
    pub fn width(&self) -> usize {
        self.data.ncols()
    }
    pub fn height(&self) -> usize {
        self.data.nrows()
    }
    pub fn view_range_mut<RowRange: DimRange<Dyn>, ColRange: DimRange<Dyn>>(
        &mut self,
        x_range: ColRange,
        y_range: RowRange,
    ) -> GridView<'_, T> {
        let shape = self.data.shape();
        let r = Dyn(shape.0);
        let rows = y_range.begin(r)..y_range.end(r);
        let c = Dyn(shape.1);
        let cols = x_range.begin(c)..x_range.end(c);
        self.changes.push((rows.clone(), cols.clone()));
        self.data.view_range_mut(rows, cols)
    }
    pub fn take_changes(&mut self) -> Vec<GridRegion> {
        std::mem::replace(&mut self.changes, Vec::new())
    }
    pub fn change(&mut self, index: Vec2us) -> Option<&mut T> {
        if let Some(d) = self.data.get_mut((index.y, index.x)) {
            self.changes
                .push((index.y..index.y + 1, index.x..index.x + 1));
            Some(d)
        } else {
            None
        }
    }
}

impl<T> Deref for TrackedGrid<T> {
    type Target = DMatrix<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

// pub fn tile_pos(&self, pos: Pos2f) -> Option<Vec2us> {
//     let mut pos = self.orientation.inverse() * pos;
//     pos += Vec2f::new(
//         (self.size.x / 2) as f32 + 0.5,
//         (self.size.y / 2) as f32 + 0.5,
//     );
//     if pos.x < 0.0 || pos.y < 0.0 {
//         return None;
//     }
//     let truncated = Vec2us::new(pos.x as usize, pos.y as usize);
//     if truncated.x > self.size.x - 1 || truncated.y > self.size.y - 1 {
//         return None;
//     }
//     Some(truncated)
// }
