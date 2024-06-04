use std::marker::PhantomData;
use wgpu::{BufferAddress, BufferUsages};

pub struct ArrBuf<T: bytemuck::Pod> {
    len: usize,
    buffer: wgpu::Buffer,
    label: String,
    typ: PhantomData<T>,
    usage: BufferUsages,
    moves: Vec<BufMove>,
}

impl<T: bytemuck::Pod> ArrBuf<T> {
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
                    std::num::NonZeroU64::new_unchecked(
                        (update.data.len() * std::mem::size_of::<T>()) as u64,
                    )
                },
                device,
            );
            view.copy_from_slice(bytemuck::cast_slice(&update.data));
        }
        resized
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

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn mov(&mut self, mov: BufMove) {
        self.moves.push(mov);
    }
}

pub struct ArrBufUpdate<T> {
    pub offset: usize,
    pub data: Vec<T>,
}

#[derive(Clone, Copy, Debug)]
pub struct BufMove {
    pub source: usize,
    pub dest: usize,
    pub size: usize,
}
