use crate::ui::preview;
use crate::ui::preview::Orientation;
use gst_video::VideoOrientationMethod;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl Orientation {
    pub fn is_vertical(&self) -> bool {
        self.angle == 90.0 || self.angle == 270.0
    }

    pub fn to_direction(&self) -> VideoOrientationMethod {
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
}

// glib version, called from outside
impl preview::Preview {
    pub fn set_orientation(&self, orintation: Orientation) {
        self.imp().orientation.set(orintation);
        self.imp().renderer.borrow_mut().orient(orintation);
        self.queue_resize();
    }
}
