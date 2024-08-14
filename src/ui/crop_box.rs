use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::{
    DerivedObjectProperties, ObjectImpl, ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt,
};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, glib, graphene, gsk, Snapshot};
use relm4::gtk;

use crate::ui::IGNORE_OVERLAY_COLOUR;

pub static MARGIN: f32 = 5.;
static HANDLE_FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
static BOX_COLOUR: gdk::RGBA = gdk::RGBA::WHITE;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "ActiveHandleType")]
pub enum ActiveHandleType {
    #[default]
    None,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "CropMode")]
pub enum CropMode {
    #[default]
    Free,
    Original,
    Square,
    _16To9,
    _4To5,
    _5To7,
    _4To3,
    _3To5,
    _3To2,
}

impl CropMode {
    fn value(&self) -> f32 {
        match *self {
            CropMode::Free => 0.,
            CropMode::Original => 0.,
            CropMode::Square => 1.,
            CropMode::_16To9 => 16. / 9.,
            CropMode::_4To3 => 4. / 3.,
            CropMode::_3To2 => 2. / 3.,
            CropMode::_4To5 => 4. / 5.,
            CropMode::_5To7 => 5. / 7.,
            CropMode::_3To5 => 3. / 5.,
        }
    }
}

// properties are represented in percentages since preview is not 1:1 to output
#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::CropBoxWidget)]
pub struct CropBoxWidget {
    #[property(get, set)]
    pub left_x: Cell<f32>,
    #[property(get, set)]
    pub top_y: Cell<f32>,
    #[property(get, set)]
    pub right_x: Cell<f32>,
    #[property(get, set)]
    pub bottom_y: Cell<f32>,
    #[property(get, set)]
    pub prev_drag_x: Cell<f32>,
    #[property(get, set)]
    pub prev_drag_y: Cell<f32>,
    #[property(get, set = Self::set_drag_active)]
    pub drag_active: Cell<bool>,
    #[property(get, set = Self::set_aspect_ratio)]
    pub asepct_ratio: Cell<f64>,
    #[property(get, set = Self::set_is_preview_rotated)]
    pub is_preview_rotated: Cell<bool>,
    #[property(get, set, builder(ActiveHandleType::None))]
    active_handle: Cell<ActiveHandleType>,
    #[property(get, set, builder(CropMode::Free))]
    crop_mode: Cell<CropMode>,
}

#[glib::object_subclass]
impl ObjectSubclass for CropBoxWidget {
    const NAME: &'static str = "CropBoxWidget";
    type Type = super::CropBoxWidget;
    type ParentType = gtk::Widget;
}

#[glib::derived_properties]
impl ObjectImpl for CropBoxWidget {}

impl WidgetImpl for CropBoxWidget {
    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();

        let border_rect = self.crop_rect(widget.width() as f32, widget.height() as f32);
        let preview = self.preview_rect(widget.width() as f32, widget.height() as f32);

        let non_cropped_area = gsk::ColorNode::new(&IGNORE_OVERLAY_COLOUR, &preview);
        let cropped_area = gsk::ColorNode::new(&BOX_COLOUR, &border_rect);
        let mask_node =
            gsk::MaskNode::new(non_cropped_area, cropped_area, gsk::MaskMode::InvertedAlpha);

        snapshot.append_node(&mask_node);

        let right_x = border_rect.x() + border_rect.width();
        let bottom_y = border_rect.y() + border_rect.height();

        if self.drag_active.get() {
            let horizontal_step = border_rect.width() / 3.;
            Self::draw_evenly_spaced_line(
                snapshot,
                true,
                horizontal_step,
                border_rect.x(),
                border_rect.y(),
                bottom_y,
            );

            let vertical_step = border_rect.height() / 3.;
            Self::draw_evenly_spaced_line(
                snapshot,
                false,
                vertical_step,
                border_rect.y(),
                border_rect.x(),
                right_x,
            );
        }

        let handle_center = self.handle_centers(widget.width() as f32, widget.height() as f32);

        for center in handle_center {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&center, MARGIN);
            let handle = path_builder.to_path();
            snapshot.append_fill(&handle, HANDLE_FILL_RULE, &BOX_COLOUR);
        }

        let border = gsk::RoundedRect::from_rect(border_rect, 0.);
        let border_widths = [1.; 4];
        let border_colours = [BOX_COLOUR; 4];

        snapshot.append_border(&border, &border_widths, &border_colours);
    }
}

impl Default for crate::ui::CropBoxWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("left_x", 0f32)
            .property("top_y", 0f32)
            .property("right_x", 1f32)
            .property("bottom_y", 1f32)
            .property("drag_active", false)
            .build()
    }
}

impl CropBoxWidget {
    fn draw_evenly_spaced_line(
        snapshot: &Snapshot,
        is_horizontal: bool,
        step_size: f32,
        step_start: f32,
        start: f32,
        end: f32,
    ) {
        let thirds_box_stroke = gsk::Stroke::builder(1.).build();

        for step in 1..3 {
            let pos = step_start + (step_size * step as f32);
            let path_builder = gsk::PathBuilder::new();

            if is_horizontal {
                path_builder.move_to(pos, start);
                path_builder.line_to(pos, end);
            } else {
                path_builder.move_to(start, pos);
                path_builder.line_to(end, pos);
            }

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &BOX_COLOUR);
        }
    }
    // returns (x, y, width, height)
    fn preview_rect(&self, widget_width: f32, widget_height: f32) -> graphene::Rect {
        let marginless_width = widget_width - (MARGIN * 2f32);
        let marginless_height = widget_height - (MARGIN * 2f32);

        let height_constrained_width = (marginless_height as f64 * self.asepct_ratio.get()) as f32;
        let width_constrained_height = (marginless_width as f64 / self.asepct_ratio.get()) as f32;

        let preview_width = marginless_width.min(height_constrained_width).ceil();
        let preview_height = marginless_height.min(width_constrained_height).ceil();

        let x = (widget_width - preview_width) / 2f32;
        // picture does not center vertically so do not need to have y_instep, besides marin
        // let y = (widget_height - preview_height) / 2f32;

        graphene::Rect::new(x, MARGIN, preview_width, preview_height)
    }

    fn crop_rect(&self, widget_width: f32, widget_height: f32) -> graphene::Rect {
        let preview = self.preview_rect(widget_width, widget_height);

        let left_x = (preview.width() * self.left_x.get()) + preview.x();
        let top_y = (preview.height() * self.top_y.get()) + preview.y();

        let right_x = ((preview.width()) * self.right_x.get()) + preview.x();
        let bottom_y = ((preview.height()) * self.bottom_y.get()) + preview.y();

        graphene::Rect::new(left_x, top_y, right_x - left_x, bottom_y - top_y)
    }

    fn handle_centers(&self, widget_width: f32, widget_height: f32) -> [graphene::Point; 4] {
        let rect = self.crop_rect(widget_width, widget_height);

        [
            graphene::Point::new(rect.x(), rect.y()),
            graphene::Point::new(rect.x(), rect.y() + rect.height()),
            graphene::Point::new(rect.x() + rect.width(), rect.y()),
            graphene::Point::new(rect.x() + rect.width(), rect.y() + rect.height()),
        ]
    }

    pub fn set_aspect_ratio(&self, aspect_ratio: f64) {
        self.asepct_ratio.set(aspect_ratio);
    }

    pub fn set_drag_active(&self, active: bool) {
        // todo: have drag end function
        self.drag_active.set(active);
        if !active {
            self.active_handle.set(ActiveHandleType::None);
            self.prev_drag_x.set(0f32);
            self.prev_drag_y.set(0f32);
        }
    }

    pub fn set_is_preview_rotated(&self, is_preview_rotated: bool) {
        if self.is_preview_rotated.get() != is_preview_rotated {
            self.set_aspect_ratio(1. / self.asepct_ratio.get());
        }
        self.is_preview_rotated.set(is_preview_rotated);
    }
}

impl crate::ui::CropBoxWidget {
    // fixme: should not have to manually call after setting crop mode
    pub fn maintain_aspect_ratio(&self) {
        if self.crop_mode() == CropMode::Free {
            return;
        }

        let target_aspect_ratio = if self.crop_mode() == CropMode::Original {
            self.asepct_ratio() as f32
        } else {
            self.crop_mode().value()
        };
        let widget_width = self.width() as f32;
        let widget_height = self.height() as f32;

        let crop_rect = if self.drag_active() {
            self.imp().crop_rect(widget_width, widget_height)
        } else {
            self.imp().preview_rect(widget_width, widget_height)
        };

        let right_x = crop_rect.x() + crop_rect.width();
        let bottom_y = crop_rect.y() + crop_rect.height();

        let is_width_constrained = crop_rect.width() < (crop_rect.height() * target_aspect_ratio);

        let (new_width, new_height) = if is_width_constrained {
            let new_height = crop_rect.width() / target_aspect_ratio;
            (crop_rect.width(), new_height)
        } else {
            let new_width = crop_rect.height() * target_aspect_ratio;
            (new_width, crop_rect.height())
        };

        let preview = self.imp().preview_rect(widget_width, widget_height);

        // todo: combine this and get_cordinate_percent_from_drag logic into point_in_percent_preview_relative
        let adjusted_left_x =
            (right_x - new_width - preview.x()).clamp(0., preview.width()) / preview.width();
        let adjusted_right_x =
            (crop_rect.x() + new_width - preview.x()).clamp(0., preview.width()) / preview.width();
        let adjusted_top_y =
            (bottom_y - new_height - preview.y()).clamp(0., preview.height()) / preview.height();
        let adjusted_bottom_y = (crop_rect.y() + new_height - preview.y())
            .clamp(0., preview.height())
            / preview.height();

        match self.active_handle() {
            ActiveHandleType::TopLeft => {
                self.set_left_x(adjusted_left_x);
                self.set_top_y(adjusted_top_y);
            }
            ActiveHandleType::TopRight => {
                self.set_right_x(adjusted_right_x);
                self.set_top_y(adjusted_top_y);
            }
            ActiveHandleType::BottomLeft => {
                self.set_left_x(adjusted_left_x);
                self.set_bottom_y(adjusted_bottom_y);
            }
            ActiveHandleType::BottomRight => {
                self.set_right_x(adjusted_right_x);
                self.set_bottom_y(adjusted_bottom_y);
            }
            ActiveHandleType::None => {
                self.set_right_x(adjusted_right_x);
                self.set_bottom_y(adjusted_bottom_y);
            }
        }
    }

    fn update_box(&self, x: f32, y: f32, changing_left_x: bool, changing_top_y: bool) {
        if changing_left_x && x < self.right_x() {
            self.set_left_x(x);
        } else if !changing_left_x && x > self.left_x() {
            self.set_right_x(x);
        }

        if changing_top_y && y < self.bottom_y() {
            self.set_top_y(y);
        } else if !changing_top_y && y > self.top_y() {
            self.set_bottom_y(y);
        }

        self.maintain_aspect_ratio();
    }

    pub fn get_cordinate_percent_from_drag(&self, x: f64, y: f64) -> (f64, f64) {
        let preview = self
            .imp()
            .preview_rect(self.width() as f32, self.height() as f32);

        let x_adj = (x - preview.x() as f64).clamp(0., preview.width() as f64);
        let y_adj = (y - preview.y() as f64).clamp(0., preview.height() as f64);

        (
            x_adj / preview.width() as f64,
            y_adj / preview.height() as f64,
        )
    }

    pub fn is_point_in_handle(&self, x: f32, y: f32) {
        let target_point = graphene::Point::new(x, y);

        let handle_centers = self
            .imp()
            .handle_centers(self.width() as f32, self.height() as f32);

        let mut point_in_circle = false;

        for (idx, point) in handle_centers.iter().enumerate() {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&point, MARGIN);
            let circle = path_builder.to_path();

            if circle.in_fill(&target_point, HANDLE_FILL_RULE) {
                let handle = match idx {
                    0 => ActiveHandleType::TopLeft,
                    1 => ActiveHandleType::BottomLeft,
                    2 => ActiveHandleType::TopRight,
                    3 => ActiveHandleType::BottomRight,
                    _ => panic!("too many handle indicies"),
                };
                self.set_active_handle(handle);
                point_in_circle = true;
                break;
            }
        }

        self.set_drag_active(point_in_circle);
    }

    pub fn update_drag_pos(&self, target: (f64, f64)) {
        let (x_percent, y_percent) = self.get_cordinate_percent_from_drag(target.0, target.1);
        let x = x_percent as f32;
        let y = y_percent as f32;

        if self.prev_drag_x() == 0. && self.prev_drag_y() == 0. {
            self.set_prev_drag_x(x);
            self.set_prev_drag_y(y);
        }

        match self.active_handle() {
            ActiveHandleType::TopLeft => {
                self.update_box(x, y, true, true);
            }
            ActiveHandleType::BottomLeft => {
                self.update_box(x, y, true, false);
            }
            ActiveHandleType::TopRight => {
                self.update_box(x, y, false, true);
            }
            ActiveHandleType::BottomRight => {
                self.update_box(x, y, false, false);
            }
            ActiveHandleType::None => {
                let offset_x = x - self.prev_drag_x();
                let offset_y = y - self.prev_drag_y();

                if offset_x == 0. && offset_y == 0. {
                    return;
                }

                // make sure step is only as big as space available to prevent box warping.
                let step_x = if offset_x < 0. && (offset_x * -1.) > self.left_x() {
                    self.left_x() * -1.
                } else if offset_x > 0. && (1. - self.right_x()) < offset_x {
                    1. - self.right_x()
                } else {
                    offset_x
                };
                let step_y = if offset_y < 0. && (offset_y * -1.) > self.top_y() {
                    self.top_y() * -1.
                } else if offset_y > 0. && (1. - self.bottom_y()) < offset_y {
                    1. - self.bottom_y()
                } else {
                    offset_y
                };

                if (step_x < 0. && self.left_x() > 0.) || (step_x > 0. && self.right_x() < 1.) {
                    let left_x = (self.left_x() + step_x).clamp(0., self.right_x());
                    let right_x = (self.right_x() + step_x).clamp(self.left_x(), 1.);

                    self.set_left_x(left_x);
                    self.set_right_x(right_x);
                }

                if (step_y < 0. && self.top_y() > 0.) || (step_y > 0. && self.bottom_y() < 1.) {
                    let top_y = (self.top_y() + step_y).clamp(0., self.bottom_y());
                    let bottom_y = (self.bottom_y() + step_y).clamp(self.top_y(), 1.);

                    self.set_top_y(top_y);
                    self.set_bottom_y(bottom_y);
                }

                self.set_prev_drag_x(x);
                self.set_prev_drag_y(y);
            }
        }
    }

    pub fn reset_box(&self) {
        self.set_top_y(0f32);
        self.set_left_x(0f32);
        self.set_bottom_y(1f32);
        self.set_right_x(1f32);

        self.set_asepct_ratio(0f64);

        self.set_active_handle(ActiveHandleType::None);
        self.set_prev_drag_x(0f32);
        self.set_prev_drag_y(0f32);
    }
}
