pub struct Texture {
    texture_desc: wgpu::TextureDescriptor<'static>,
    view_desc: wgpu::TextureViewDescriptor<'static>,
    sampler_desc: wgpu::SamplerDescriptor<'static>,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn init_depth(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &'static str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width + 1,
            height: config.height + 1,
            depth_or_array_layers: 1,
        };
        let texture_desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        Self::init(
            device,
            texture_desc,
            wgpu::TextureViewDescriptor::default(),
            wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            },
        )
    }

    pub fn init(
        device: &wgpu::Device,
        texture_desc: wgpu::TextureDescriptor<'static>,
        view_desc: wgpu::TextureViewDescriptor<'static>,
        sampler_desc: wgpu::SamplerDescriptor<'static>,
    ) -> Self {
        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&view_desc);
        let sampler = device.create_sampler(&sampler_desc);
        Self {
            texture_desc,
            view_desc,
            sampler_desc,
            texture,
            view,
            sampler,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) {
        self.texture_desc.size = size;
        self.texture = device.create_texture(&self.texture_desc);
        self.view = self.texture.create_view(&self.view_desc);
    }
    pub fn view_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
    pub fn sampler_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        }
    }
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture_desc.format
    }
}
