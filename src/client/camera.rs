use nalgebra::{Point3, Rotation3, UnitVector3, Vector3};

const DEFAULT_ASPECT_RATIO: f32 = 16. / 9.;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pos: Point3<f32>,
    pub orientation: Rotation3<f32>,
    pub aspect: f32,
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point3::origin(),
            orientation: Rotation3::identity(),
            aspect: DEFAULT_ASPECT_RATIO,
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

