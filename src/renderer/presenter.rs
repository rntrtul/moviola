use crate::renderer::export_texture::ExportTexture;

pub struct Presenter {
    textures: Vec<ExportTexture>,
    current: usize,
}

impl Presenter {
    pub fn new(
        buffer_count: usize,
        device: &wgpu::Device,
        instance: &wgpu::Instance,
        texture_size: wgpu::Extent3d,
    ) -> Presenter {
        let mut textures = vec![];
        for _ in 0..buffer_count {
            textures.push(ExportTexture::new(&device, &instance, texture_size))
        }

        Presenter {
            textures,
            current: 0,
        }
    }

    pub fn resize_outputs(
        &mut self,
        device: &wgpu::Device,
        instance: &wgpu::Instance,
        texture_size: wgpu::Extent3d,
    ) {
        let count = self.textures.len();
        for i in 0..count {
            self.textures[i] = ExportTexture::new(&device, &instance, texture_size);
        }
    }

    pub fn current_presentation_texture(&self) -> &ExportTexture {
        &self.textures[self.current]
    }

    pub fn next_presentation_texture(&mut self) -> &ExportTexture {
        let mut next = self.current + 1;
        if next == self.textures.len() {
            next = 0;
        }

        self.current = next;
        &self.textures[next]
    }
}
