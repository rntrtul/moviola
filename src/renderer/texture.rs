use anyhow::*;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn new_for_size(
        width: u32,
        height: u32,
        device: &wgpu::Device,
        label: &str,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn write_from_bytes(&self, queue: &wgpu::Queue, bytes: &[u8]) {
        let img = image::load_from_memory(bytes).unwrap();
        self.write_from_image(queue, &img);
    }

    pub fn write_from_image(&self, queue: &wgpu::Queue, img: &image::DynamicImage) {
        let rgba = img.to_rgba8();
        self.write_from_pixel_buffer(queue, &rgba);
    }
    pub fn write_from_sample(&self, queue: &wgpu::Queue, sample: &gst::Sample) {
        let buffer = sample.buffer().unwrap();
        self.write_from_pixel_buffer(queue, &buffer.map_readable().unwrap().as_slice());
    }

    pub fn write_from_pixel_buffer(&self, queue: &wgpu::Queue, buffer: &[u8]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.texture.width()),
                rows_per_image: None,
            },
            self.texture.size(),
        );
    }
}
