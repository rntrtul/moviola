use gtk4::glib;

mod handle;
mod timeline;

#[derive(Debug)]
pub struct TimelineModel {
    thumbnails_available: bool,
    start: f32,
    end: f32,
    prev_drag_target: f64,
}

#[derive(Debug)]
pub enum TimelineMsg {
    GenerateThumbnails(String),
    PopulateTimeline,
    DragBegin(f64, f64),
    DragUpdate(f64),
    DragEnd,
    UpdateSeekBarPos(f64),
    SeekToPercent(f64),
    Reset,
}

#[derive(Debug)]
pub enum TimelineOutput {
    SeekToPercent(f64),
}

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk4::Widget;
}
