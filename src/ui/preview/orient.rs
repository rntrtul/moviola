#[derive(Debug, Clone, Copy)]
pub struct Orientation {
    pub(crate) base_angle: f32,
    pub(crate) angle: f32,
    pub(crate) mirrored: bool,
}

impl Orientation {
    pub fn new_with_base(base_angle: f32) -> Self {
        Self {
            base_angle,
            angle: 0.0f32,
            mirrored: false,
        }
    }

    pub fn reset(&mut self) {
        self.mirrored = false;
        self.angle = 0f32;
    }

    pub fn absolute_angle(&self) -> f32 {
        (self.base_angle + self.angle) % 360.0
    }

    pub fn is_width_flipped(&self) -> bool {
        self.absolute_angle() == 90.0 || self.absolute_angle() == 270.0
    }

    pub fn is_base_width_flipped(&self) -> bool {
        self.base_angle == 90.0 || self.base_angle == 270.0
    }

    fn flip_mirrored(&mut self) {
        self.mirrored = !self.mirrored;
    }

    pub fn oriented_size(&self, width: u32, height: u32) -> (u32, u32) {
        if self.is_width_flipped() {
            (height, width)
        } else {
            (width, height)
        }
    }

    pub fn rotate_90_clockwise(&mut self) {
        self.angle = (self.angle + 90.0) % 360.0;
    }

    fn flip_horizontally(&mut self) {
        self.flip_mirrored();
    }

    fn flip_vertical(&mut self) {
        self.angle = (self.angle + 180.0) % 360.0;
        self.flip_mirrored();
    }

    pub fn mirror_horizontally(&mut self) {
        if self.is_base_width_flipped() {
            self.flip_vertical();
        } else {
            self.flip_horizontally();
        }
    }

    pub fn mirror_vertically(&mut self) {
        if self.is_base_width_flipped() {
            self.flip_horizontally();
        } else {
            self.flip_mirrored();
        }
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self {
            base_angle: 0.0,
            angle: 0.0,
            mirrored: false,
        }
    }
}
