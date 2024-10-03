use std::time::Duration;

pub struct GPUTimer {
    resolve_buf: wgpu::Buffer,
    map_buf: wgpu::Buffer,
    query_set: wgpu::QuerySet,
    timestamps: Vec<u64>,
}

impl GPUTimer {
    pub fn new(device: &wgpu::Device, count: u32) -> Self {
        let count = count * 2;
        let timestamp_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            count,
            label: Some("voxel timestamp"),
            ty: wgpu::QueryType::Timestamp,
        });
        let timestamp_resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel timestamp"),
            mapped_at_creation: false,
            size: 8 * count as u64,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        });

        let timestamp_mapped_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel timestamp"),
            mapped_at_creation: false,
            size: 8 * count as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        });

        Self {
            query_set: timestamp_set,
            resolve_buf: timestamp_resolve_buf,
            map_buf: timestamp_mapped_buf,
            timestamps: vec![0; count as usize],
        }
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(&self.query_set, 0..2, &self.resolve_buf, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buf, 0, &self.map_buf, 0, self.map_buf.size());
    }

    pub fn duration(&self, i: u32) -> Duration {
        let i = i as usize * 2;
        Duration::from_nanos(self.timestamps[i + 1] - self.timestamps[i])
    }

    pub fn finish(&mut self, device: &wgpu::Device) {
        let (s, r) = std::sync::mpsc::channel();
        self.map_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |v| {
                s.send(v).expect("what");
            });
        device.poll(wgpu::Maintain::wait()).panic_on_timeout();
        if let Ok(Ok(())) = r.recv() {
            let data = self.map_buf.slice(..).get_mapped_range();
            self.timestamps.copy_from_slice(bytemuck::cast_slice(&data));
            drop(data);
            self.map_buf.unmap();
        }
    }

    #[allow(dead_code)]
    pub fn start(&self, encoder: &mut wgpu::CommandEncoder, i: u32) {
        encoder.write_timestamp(&self.query_set, i * 2);
    }

    #[allow(dead_code)]
    pub fn stop(&self, encoder: &mut wgpu::CommandEncoder, i: u32) {
        encoder.write_timestamp(&self.query_set, i * 2 + 1);
    }

    #[allow(dead_code)]
    pub fn start_compute(&self, pass: &mut wgpu::ComputePass, i: u32) {
        pass.write_timestamp(&self.query_set, i * 2);
    }

    #[allow(dead_code)]
    pub fn stop_compute(&self, pass: &mut wgpu::ComputePass, i: u32) {
        pass.write_timestamp(&self.query_set, i * 2 + 1);
    }
}
