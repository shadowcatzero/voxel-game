use wgpu::{BufferUsages, VertexAttribute};

use super::buf::{ArrBuf, ArrBufUpdate, BufMove};

pub struct Instances<T: bytemuck::Pod> {
    buf: ArrBuf<T>,
    location: u32,
    attrs: [VertexAttribute; 1],
}

impl<T: bytemuck::Pod> Instances<T> {
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

    pub fn init(
        device: &wgpu::Device,
        label: &str,
        location: u32,
        format: wgpu::VertexFormat,
    ) -> Self {
        Self {
            buf: ArrBuf::init(
                device,
                &(label.to_owned() + " Instance"),
                BufferUsages::VERTEX,
            ),
            location,
            attrs: [wgpu::VertexAttribute {
                format,
                offset: 0,
                shader_location: location,
            }],
        }
    }

    pub fn set_in<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(self.location, self.buf.buffer().slice(..));
    }

    pub fn desc(&self) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<T>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &self.attrs,
        }
    }

    pub fn mov(&mut self, mov: BufMove) {
        self.buf.mov(mov);
    }
}
