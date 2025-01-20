use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FramePositionUniform {
    // todo: send a matrix for rotation and translate
    rotation: [f32; 2],
    crop_start: [u32; 2],
    crop_size: [u32; 2],
    translate: [u32; 2],
}

impl FramePositionUniform {
    pub fn new() -> Self {
        Self {
            rotation: [1.0, 0.0],
            crop_start: [0, 0],
            crop_size: [0, 0],
            translate: [0, 0],
        }
    }

    pub fn update_rotation(&mut self, radians: f32) {
        self.rotation = [radians.cos(), radians.sin()];
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Position Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
