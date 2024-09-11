use std::marker::PhantomData;

use wgpu::util::DeviceExt;

pub struct Uniform<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    binding: u32,
    ty: PhantomData<T>,
}

impl<T: Default + bytemuck::Pod> Uniform<T> {
    pub fn init(device: &wgpu::Device, name: &str, binding: u32) -> Self {
        Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&(name.to_owned() + " Uniform Buf")),
                contents: bytemuck::cast_slice(&[T::default()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            binding,
            ty: PhantomData,
        }
    }
}

impl<T: PartialEq + bytemuck::Pod> Uniform<T> {
    pub fn init_with(device: &wgpu::Device, name: &str, binding: u32, data: &[T]) -> Self {
        Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&(name.to_owned() + " Uniform Buf")),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            binding,
            ty: PhantomData,
        }
    }
    pub fn bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
    pub fn bind_group_entry(&self) -> wgpu::BindGroupEntry {
        return wgpu::BindGroupEntry {
            binding: self.binding,
            resource: self.buffer.as_entire_binding(),
        };
    }
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        data: T,
    ) {
        let slice = &[data];
        let mut view = belt.write_buffer(
            encoder,
            &self.buffer,
            0,
            unsafe {
                std::num::NonZeroU64::new_unchecked((slice.len() * std::mem::size_of::<T>()) as u64)
            },
            device,
        );
        view.copy_from_slice(bytemuck::cast_slice(slice));
    }
}
