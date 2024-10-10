use crate::ui::preview;
use crate::ui::preview::preview::Preview;
use crate::ui::preview::Orientation;
use ges::subclass::prelude::ObjectSubclassExt;
use gst_video::VideoOrientationMethod;
use gtk4::prelude::{SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl Preview {
    pub(crate) fn orient_snapshot(&self, snapshot: &gtk4::Snapshot) {
        let orientation = self.orientation.get();

        let x_center = self.obj().width() as f32 / 2.0;
        let y_center = self.obj().height() as f32 / 2.0;

        if orientation.angle != 0.0 || orientation.mirrored {
            snapshot.translate(&gtk4::graphene::Point::new(x_center, y_center));
        }

        if orientation.mirrored {
            match orientation.angle {
                0.0 => snapshot.scale(-1.0, 1.0),
                90.0 => {
                    snapshot.rotate(90.0);
                    snapshot.scale(-1.0, 1.0);
                }
                270.0 => {
                    snapshot.rotate(270.0);
                    snapshot.scale(-1.0, 1.0);
                }
                180.0 => snapshot.scale(1.0, -1.0),
                _ => {}
            }
        } else {
            match orientation.angle {
                90.0 => snapshot.rotate(90.0),
                270.0 => snapshot.rotate(270.0),
                180.0 => snapshot.scale(-1.0, -1.0),
                _ => {}
            }
        }

        if orientation.angle != 0.0 || orientation.mirrored {
            if orientation.is_vertical() {
                snapshot.translate(&gtk4::graphene::Point::new(-y_center, -x_center));
            } else {
                snapshot.translate(&gtk4::graphene::Point::new(-x_center, -y_center));
            }
        }
    }
}

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
        self.queue_resize();
    }
}
