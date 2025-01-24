use crate::ui::preview;
use gst_video::VideoOrientationMethod;
use relm4::gtk::prelude::WidgetExt;
use relm4::gtk::subclass::prelude::ObjectSubclassIsExt;

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

    pub fn absolute_angle(&self) -> f32 {
        (self.base_angle + self.angle) % 360.0
    }

    pub fn is_width_flipped(&self) -> bool {
        self.absolute_angle() == 90.0 || self.absolute_angle() == 270.0
    }

    pub fn to_gst_video_orientation(&self) -> VideoOrientationMethod {
        // only using angle since base angle encoded in video metadata
        if self.mirrored {
            match self.angle {
                0.0 => VideoOrientationMethod::Horiz,
                90.0 => VideoOrientationMethod::UrLl,
                180.0 => VideoOrientationMethod::Vert,
                270.0 => VideoOrientationMethod::UlLr,
                _ => VideoOrientationMethod::Auto,
            }
        } else {
            match self.angle {
                0.0 => VideoOrientationMethod::Identity,
                90.0 => VideoOrientationMethod::_90r,
                180.0 => VideoOrientationMethod::_180,
                270.0 => VideoOrientationMethod::_90r,
                _ => VideoOrientationMethod::Identity,
            }
        }
    }

    pub fn flip_mirrored(&mut self) {
        self.mirrored = !self.mirrored;
    }

    pub fn oriented_size(&self, width: u32, height: u32) -> (u32, u32) {
        if self.absolute_angle() == 90.0 || self.absolute_angle() == 180.0 {
            (height, width)
        } else {
            (width, height)
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

// glib version, called from outside
impl preview::Preview {
    pub fn set_orientation(&self, orintation: Orientation) {
        self.imp().orientation.set(orintation);
        self.queue_resize();
    }
}
