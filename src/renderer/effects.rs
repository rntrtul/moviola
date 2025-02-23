use crate::range::Range;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EffectParameters {
    pub contrast: f32,
    pub brigthness: f32,
    pub saturation: f32,
}

impl Default for EffectParameters {
    fn default() -> Self {
        Self {
            contrast: 1f32,
            brigthness: 0f32,
            saturation: 1f32,
        }
    }
}

impl EffectParameters {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn reset(&mut self) {
        self.contrast = 1f32;
        self.brigthness = 0f32;
        self.saturation = 1f32;
    }

    pub fn is_default(&self) -> bool {
        self == &Default::default()
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Effects Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn set_contrast(&mut self, value: f32) {
        self.contrast = value;
    }

    pub fn contrast_range() -> (Range, Range) {
        (Range::new(0.5, 1.5), Range::new(-100.0, 100.0))
    }

    pub fn set_brightness(&mut self, value: f32) {
        self.brigthness = value;
    }

    pub fn brigntess_range() -> (Range, Range) {
        (Range::new(-0.25, 0.25), Range::new(-100.0, 100.0))
    }

    pub fn set_saturation(&mut self, value: f32) {
        self.saturation = value;
    }

    pub fn saturation_range() -> (Range, Range) {
        (Range::new(0.0, 2.0), Range::new(-100.0, 100.0))
    }
}
