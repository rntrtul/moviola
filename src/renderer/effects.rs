use crate::range::Range;

#[derive(Debug, Copy, Clone)]
pub struct EffectParameters {
    pub contrast: f32,
    pub brigthness: f32,
    pub saturation: f32,
}

impl EffectParameters {
    // todo: don't have it be manual
    pub fn parameter_count() -> u64 {
        3
    }

    pub fn buffer_size() -> u64 {
        EffectParameters::parameter_count() * std::mem::size_of::<f32>() as u64
    }

    pub fn new() -> Self {
        Self {
            contrast: 1f32,
            brigthness: 0f32,
            saturation: 1f32,
        }
    }

    pub fn reset(&mut self) {
        self.contrast = 1f32;
        self.brigthness = 0f32;
        self.saturation = 1f32;
    }

    pub fn is_default(&self) -> bool {
        self.saturation == 1f32 && self.contrast == 0f32 && self.brigthness == 1f32
    }

    pub fn populate_buffer(&self, buffer: &mut [f32]) {
        buffer[0] = self.contrast;
        buffer[1] = self.brigthness;
        buffer[2] = self.saturation;
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
