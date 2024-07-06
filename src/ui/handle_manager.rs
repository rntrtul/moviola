use std::rc::Rc;

use gtk4::prelude::WidgetExt;

use crate::ui::HandleWidget;

#[derive(Debug)]
pub struct HandleManager {
    pub start_handle: Rc<HandleWidget>,
    pub end_handle: Rc<HandleWidget>,
}

impl HandleManager {
    pub fn set_start_pos(&self, pos: f64) {
        self.start_handle.set_percent_pos(pos);
    }

    pub fn set_end_pos(&self, pos: f64) {
        self.end_handle.set_percent_pos(pos);
    }

    pub fn try_set_start_rel_x(&self, offset: i32, container_width: i32) -> bool {
        let end_handle_pos = container_width - self.end_handle.x();
        let target_position = self.start_handle.x() + offset;

        let bound_by_end_handle = target_position < end_handle_pos;
        let bound_by_container = target_position >= 0;

        let rel_container_end = -self.start_handle.x();
        let at_container_end = self.start_handle.rel_x() == rel_container_end;

        let mut rel_x_changed = false;
        self.start_handle.set_target_x(target_position);

        if bound_by_end_handle {
            if bound_by_container {
                self.start_handle.set_rel_x(offset);
                rel_x_changed = true;
            } else if !bound_by_container && !at_container_end {
                self.start_handle.set_rel_x(rel_container_end);
                rel_x_changed = true;
            }
        }

        if rel_x_changed {
            self.start_handle.queue_draw();
        }

        return rel_x_changed;
    }

    pub fn try_set_end_rel_x(&self, offset: i32, container_width: i32) -> bool {
        let start_handle_pos = self.start_handle.x();
        let offset_from_container_end = -self.end_handle.x() + offset;
        let target_position = container_width + offset_from_container_end;

        let bound_by_start_handle = target_position > start_handle_pos;
        let bound_by_container = offset_from_container_end <= 0;

        let rel_container_end = self.end_handle.x();
        let at_container_end = self.end_handle.rel_x() == rel_container_end;

        let mut rel_x_changed = false;
        self.end_handle.set_target_x(target_position);

        if bound_by_start_handle {
            if bound_by_container {
                self.end_handle.set_rel_x(offset);
                rel_x_changed = true;
            } else if !bound_by_container && !at_container_end {
                self.end_handle.set_rel_x(rel_container_end);
                rel_x_changed = true;
            }
        }

        if rel_x_changed {
            self.end_handle.queue_draw();
        }

        return rel_x_changed;
    }

    pub fn set_start_margin(&self) {
        let curr_margin = self.start_handle.x();
        let new_margin = curr_margin + self.start_handle.rel_x();

        self.start_handle.set_x(new_margin);
        self.start_handle.set_margin_start(new_margin);
        self.start_handle.set_rel_x(0);
    }

    pub fn set_end_margin(&self) {
        let curr_maragin = self.end_handle.x();
        let new_margin = (-curr_maragin + self.end_handle.rel_x()).abs();

        self.end_handle.set_x(new_margin);
        self.end_handle.set_margin_end(new_margin);
        self.end_handle.set_rel_x(0);
    }
}
