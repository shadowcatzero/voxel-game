use nalgebra::Vector3;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct GlobalLight {
    pub direction: Vector3<f32>,
}

unsafe impl bytemuck::Pod for GlobalLight {}
