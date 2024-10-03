use std::marker::PhantomData;
use wgpu::{util::DeviceExt, BufferAddress, BufferUsages};

pub struct ArrayBuffer<T: bytemuck::Pod> {
    len: usize,
    buffer: wgpu::Buffer,
    label: String,
    typ: PhantomData<T>,
    usage: BufferUsages,
    moves: Vec<BufMove>,
}

impl<T: bytemuck::Pod> ArrayBuffer<T> {
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        size: usize,
        updates: &[ArrBufUpdate<T>],
    ) -> bool {
        let mut resized = false;
        if size != self.len || !self.moves.is_empty() {
            let new = Self::init_buf(device, &self.label, size, self.usage);
            let cpy_len = self.len.min(size);
            encoder.copy_buffer_to_buffer(
                &self.buffer,
                0,
                &new,
                0,
                (cpy_len * std::mem::size_of::<T>()) as u64,
            );
            for m in &self.moves {
                encoder.copy_buffer_to_buffer(
                    &self.buffer,
                    (m.source * std::mem::size_of::<T>()) as BufferAddress,
                    &new,
                    (m.dest * std::mem::size_of::<T>()) as BufferAddress,
                    (m.size * std::mem::size_of::<T>()) as BufferAddress,
                );
            }
            resized = true;
            self.moves.clear();
            self.len = size;
            self.buffer = new;
        }
        if self.len == 0 {
            return resized;
        }
        for update in updates {
            let mut view = belt.write_buffer(
                encoder,
                &self.buffer,
                (update.offset * std::mem::size_of::<T>()) as BufferAddress,
                unsafe {
                    std::num::NonZeroU64::new_unchecked(std::mem::size_of_val(update.data) as u64)
                },
                device,
            );
            view.copy_from_slice(bytemuck::cast_slice(update.data));
        }
        resized
    }

    pub fn add(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        data: &[T],
    ) {
        self.update(
            device,
            encoder,
            belt,
            self.len + data.len(),
            &[ArrBufUpdate {
                offset: self.len,
                data,
            }],
        );
    }

    pub fn set(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        offset: usize,
        data: &[T],
    ) {
        self.update(
            device,
            encoder,
            belt,
            self.len,
            &[ArrBufUpdate { offset, data }],
        );
    }

    pub fn init(device: &wgpu::Device, label: &str, usage: BufferUsages) -> Self {
        let label = &(label.to_owned() + " Buffer");
        Self {
            len: 0,
            buffer: Self::init_buf(device, label, 0, usage),
            label: label.to_string(),
            typ: PhantomData,
            usage,
            moves: Vec::new(),
        }
    }

    pub fn init_with(device: &wgpu::Device, label: &str, usage: BufferUsages, data: &[T]) -> Self {
        let label = &(label.to_owned() + " Buffer");
        Self {
            len: data.len(),
            buffer: Self::init_buf_with(device, label, usage, data),
            label: label.to_string(),
            typ: PhantomData,
            usage,
            moves: Vec::new(),
        }
    }

    fn init_buf(
        device: &wgpu::Device,
        label: &str,
        mut size: usize,
        usage: BufferUsages,
    ) -> wgpu::Buffer {
        if usage.contains(BufferUsages::STORAGE) && size == 0 {
            size = 1;
        }
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            usage: usage | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            size: (size * std::mem::size_of::<T>()) as u64,
            mapped_at_creation: false,
        })
    }

    fn init_buf_with(
        device: &wgpu::Device,
        label: &str,
        usage: BufferUsages,
        data: &[T],
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            usage: usage | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            contents: bytemuck::cast_slice(data),
        })
    }

    pub fn mov(&mut self, mov: BufMove) {
        self.moves.push(mov);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry {
        return wgpu::BindGroupEntry {
            binding,
            resource: self.buffer.as_entire_binding(),
        };
    }
}

pub struct ArrBufUpdate<'a, T> {
    pub offset: usize,
    pub data: &'a [T],
}

#[derive(Clone, Copy, Debug)]
pub struct BufMove {
    pub source: usize,
    pub dest: usize,
    pub size: usize,
}
