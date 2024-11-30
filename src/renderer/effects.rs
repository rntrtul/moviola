use crate::ui::Range;

#[derive(Debug, Copy, Clone)]
pub struct EffectParameters {
    contrast: f32,
    brigthness: f32,
    hue: f32,
    saturation: f32,
}

impl EffectParameters {
    // todo: don't have it be manual
    pub fn parameter_count() -> u64 {
        4
    }

    pub fn buffer_size() -> u64 {
        EffectParameters::parameter_count() * std::mem::size_of::<f32>() as u64
    }

    pub fn new() -> Self {
        Self {
            contrast: 1f32,
            brigthness: 0f32,
            hue: 0f32,
            saturation: 0f32,
        }
    }

    pub fn populate_buffer(&self, buffer: &mut [f32]) {
        buffer[0] = self.contrast;
        buffer[1] = self.brigthness;
        buffer[2] = self.hue;
        buffer[3] = self.saturation;
    }

    pub fn set_contrast(&mut self, value: f32) {
        self.contrast = value;
    }

    pub fn contrast_range() -> (Range, Range) {
        (Range::new(0.5, 1.5), Range::new(-100.0, 100.0))
    }
}
