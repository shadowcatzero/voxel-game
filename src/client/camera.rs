use nalgebra::{Rotation3, UnitVector3, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub pos: Vector3<f32>,
    pub orientation: Rotation3<f32>,
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vector3::zeros(),
            orientation: Rotation3::identity(),
            scale: 1.0,
        }
    }
}

impl Camera {
    pub fn left(&self) -> UnitVector3<f32> {
        self.orientation * -Vector3::x_axis()
    }
    pub fn right(&self) -> UnitVector3<f32> {
        self.orientation * Vector3::x_axis()
    }
    pub fn down(&self) -> UnitVector3<f32> {
        self.orientation * -Vector3::y_axis()
    }
    pub fn up(&self) -> UnitVector3<f32> {
        self.orientation * Vector3::y_axis()
    }
    pub fn backward(&self) -> UnitVector3<f32> {
        self.orientation * -Vector3::z_axis()
    }
    pub fn forward(&self) -> UnitVector3<f32> {
        self.orientation * Vector3::z_axis()
    }
}
