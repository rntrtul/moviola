use crate::ui::preview::Orientation;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// todo: send a matrix for rotation and translate
pub struct FramePositionUniform {
    rotation: [f32; 2],
    crop_start: [u32; 2],
    crop_size: [u32; 2],
    translate: [u32; 2],
    orientation: f32,
    mirrored: u32,
}

impl FramePositionUniform {
    pub fn new() -> Self {
        Self {
            orientation: 0.0,
            rotation: [1.0, 0.0],
            crop_start: [0, 0],
            crop_size: [0, 0],
            translate: [0, 0],
            mirrored: 0,
        }
    }

    pub fn update_rotation(&mut self, radians: f32) {
        self.rotation = [radians.cos(), radians.sin()];
    }

    pub fn orient(&mut self, orientation: Orientation) {
        self.orientation = orientation.absolute_angle();
        self.mirrored = if orientation.mirrored { 1 } else { 0 };
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Position Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

impl FrameSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Size Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
