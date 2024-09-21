use std::sync::Arc;

use vulkano::{buffer::{Buffer, BufferCreateInfo, BufferUsage}, memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter}};

pub struct VoxelPipeline {

}

impl VoxelPipeline {
    pub fn init(memory_allocator: Arc<impl MemoryAllocator>) {
        let buffer = Buffer::from_data(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        );
    }
}
