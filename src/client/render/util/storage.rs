use super::buf::{ArrBuf, ArrBufUpdate, BufMove};
use wgpu::BufferUsages;

pub struct Storage<T: bytemuck::Pod + PartialEq> {
    binding: u32,
    buf: ArrBuf<T>,
}

impl<T: PartialEq + bytemuck::Pod> Storage<T> {
    pub fn init(device: &wgpu::Device, label: &str, binding: u32) -> Self {
        Self {
            buf: ArrBuf::init(
                device,
                &(label.to_owned() + " Storage"),
                BufferUsages::STORAGE,
            ),
            binding,
        }
    }
    pub fn init_with(device: &wgpu::Device, label: &str, binding: u32, data: &[T]) -> Self {
        Self {
            buf: ArrBuf::init_with(
                device,
                &(label.to_owned() + " Storage"),
                BufferUsages::STORAGE,
                data
            ),
            binding,
        }
    }
}

impl<T: PartialEq + bytemuck::Pod> Storage<T> {
    pub fn bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
    pub fn bind_group_entry(&self) -> wgpu::BindGroupEntry {
        return wgpu::BindGroupEntry {
            binding: self.binding,
            resource: self.buf.buffer().as_entire_binding(),
        };
    }
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        size: usize,
        updates: &[ArrBufUpdate<T>],
    ) -> bool {
        self.buf.update(device, encoder, belt, size, updates)
    }

    pub fn mov(&mut self, mov: BufMove) {
        self.buf.mov(mov);
    }

    pub fn len(&mut self) -> usize {
        self.buf.len()
    }
}
