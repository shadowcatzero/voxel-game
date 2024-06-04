use nalgebra::{Matrix4x3, Point2, Projective2, Transform2, Vector2, Vector3};

pub type Vec2f = Vector2<f32>;
pub type Vec3f = Vector3<f32>;
pub type Pos2f = Point2<f32>;
pub type Vec2us = Vector2<usize>;

// hahaha.. HAHAHAAAAAAAAAAA... it's over now, surely I'll remember this lesson
pub trait Bruh<T> {
    fn gpu_mat3(&self) -> Matrix4x3<T>;
}

impl Bruh<f32> for Transform2<f32> {
    fn gpu_mat3(&self) -> Matrix4x3<f32> {
        let mut a = Matrix4x3::identity();
        // I LOVE GPU DATA STRUCTURE ALIGNMENT (it makes sense tho)
        a.view_mut((0,0), (3,3)).copy_from(self.matrix());
        a
    }
}

impl Bruh<f32> for Projective2<f32> {
    fn gpu_mat3(&self) -> Matrix4x3<f32> {
        let mut a = Matrix4x3::identity();
        // I LOVE GPU DATA STRUCTURE ALIGNMENT (it makes sense tho)
        a.view_mut((0,0), (3,3)).copy_from(self.matrix());
        a
    }
}
