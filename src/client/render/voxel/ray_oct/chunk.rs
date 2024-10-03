#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Zeroable)]
pub struct Chunk {
    pub offset: u32,
}

unsafe impl bytemuck::Pod for Chunk {}
