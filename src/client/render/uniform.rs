use wgpu::util::DeviceExt;

use super::RenderUpdateData;

pub trait UniformData {
    fn update(&mut self, data: &RenderUpdateData) -> bool;
}

pub struct Uniform<T: bytemuck::Pod + PartialEq + UniformData> {
    data: T,
    buffer: wgpu::Buffer,
    binding: u32,
}

impl<T: Default + PartialEq + bytemuck::Pod + UniformData> Uniform<T> {
    pub fn init(device: &wgpu::Device, name: &str, binding: u32) -> Self {
        let data = T::default();
        Self {
            data,
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&(name.to_owned() + " Uniform Buf")),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            binding,
        }
    }
}

impl<T: PartialEq + bytemuck::Pod + UniformData> Uniform<T> {
    pub fn bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
        update_data: &RenderUpdateData,
    ) {
        if self.data.update(update_data) {
            let slice = &[self.data];
            let mut view = belt.write_buffer(
                encoder,
                &self.buffer,
                0,
                unsafe {
                    std::num::NonZeroU64::new_unchecked(
                        (slice.len() * std::mem::size_of::<T>()) as u64,
                    )
                },
                device,
            );
            view.copy_from_slice(bytemuck::cast_slice(slice));
        }
    }
}
