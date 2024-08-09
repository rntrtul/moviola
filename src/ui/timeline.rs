use std::cell::RefCell;
use std::rc::Rc;

use gst::ClockTime;
use gtk4::gio;
use gtk4::prelude::{BoxExt, EventControllerExt, GestureDragExt, WidgetExt};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::ui::handle::HANDLE_WIDTH;
use crate::video::export::TimelineExportSettings;
use crate::video::player::Player;
use crate::video::thumbnail::Thumbnail;

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

// fixme: ugly handnling of frameinfo
#[derive(Debug)]
pub enum TimelineCmdMsg {
    ThumbnailsGenerated,
}

#[derive(Debug)]
pub enum TimelineOutput {
    SeekToPercent(f64),
}

#[relm4::component(pub)]
impl Component for TimelineModel {
    type CommandOutput = TimelineCmdMsg;
    type Input = TimelineMsg;
    type Output = TimelineOutput;
    type Init = ();

    view! {
        gtk::Overlay {
            connect_get_child_position => move |_, _| {
                // fixme: adjust handlebar position on resize of timeline
                return None;
            },

            #[wrap(Some)]
            set_child: timeline = &gtk::Box {
                set_hexpand: true,
                set_margin_start: HANDLE_WIDTH as i32,
                set_margin_end: HANDLE_WIDTH as i32,
            },

            add_overlay: seek_bar = &super::HandleWidget::default() {
                add_controller = gtk::GestureDrag {
                    connect_drag_begin[sender] => move |_,x,y| {
                        sender.input(TimelineMsg::DragBegin(x, y))
                    },

                    connect_drag_update[sender] => move |drag,offset_x,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let targ_x = start_x + offset_x;
                        sender.input(TimelineMsg::DragUpdate(targ_x))
                    },

                    connect_drag_end[sender] => move |_, _,_| {
                        sender.input(TimelineMsg::DragEnd);
                    },
                },

                 add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |click,_,x,_| {
                        let width = click.widget().unwrap().width() as f64;
                        let percent = x / width;
                        sender.input(TimelineMsg::SeekToPercent(percent));
                    }
                },
            },
        },
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TimelineModel {
            thumbnails_available: false,
            start: 0.,
            end: 1.,
            prev_drag_target: -1.,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TimelineMsg::GenerateThumbnails(uri) => {
                Self::remove_timeline_thumbnails(&widgets.timeline);
                self.thumbnails_available = false;

                sender.oneshot_command(async move {
                    Thumbnail::generate_thumbnails(uri).await;
                    TimelineCmdMsg::ThumbnailsGenerated
                });
            }
            TimelineMsg::PopulateTimeline => {
                let timeline = &widgets.timeline;
                TimelineModel::populate_timeline(timeline);
            }
            TimelineMsg::SeekToPercent(percent) => {
                widgets.seek_bar.set_seek_x(percent as f32);
                widgets.seek_bar.queue_draw();
                sender
                    .output(TimelineOutput::SeekToPercent(percent))
                    .unwrap();
            }
            TimelineMsg::UpdateSeekBarPos(percent) => {
                // todo: make smoother update. increase poll rate or use animation
                widgets.seek_bar.set_seek_x(percent as f32);
                widgets.seek_bar.queue_draw();
            }
            TimelineMsg::DragBegin(x, y) => {
                widgets.seek_bar.drag_start(x, y);
            }
            TimelineMsg::DragUpdate(target_x) => {
                widgets.seek_bar.drag_update(target_x as f32);
                widgets.seek_bar.queue_draw();
                //
                if target_x != self.prev_drag_target {
                    sender.input(TimelineMsg::SeekToPercent(widgets.seek_bar.seek_x() as f64));
                    self.prev_drag_target = target_x;
                }
            }
            TimelineMsg::DragEnd => {
                widgets.seek_bar.drag_end();
                self.start = widgets.seek_bar.start_x();
                self.end = widgets.seek_bar.end_x();
                self.prev_drag_target = -1.;
            }
            TimelineMsg::Reset => {
                self.start = 0f32;
                self.end = 1f32;
                self.prev_drag_target = -1.;
                widgets.seek_bar.reset();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TimelineCmdMsg::ThumbnailsGenerated => {
                self.thumbnails_available = true;
                sender.input(TimelineMsg::PopulateTimeline);
            }
        }
    }
}

impl TimelineModel {
    pub fn get_export_settings(&self, player: Rc<RefCell<Player>>) -> TimelineExportSettings {
        let duration_mseconds = player.borrow().info.duration.mseconds() as f32;

        let start = ClockTime::from_mseconds((duration_mseconds * self.start) as u64);
        let end = ClockTime::from_mseconds((duration_mseconds * self.end) as u64);
        let duration = end - start;

        TimelineExportSettings { start, duration }
    }

    fn remove_timeline_thumbnails(timeline: &gtk::Box) {
        if timeline.first_child().is_some() {
            for _ in 0..Thumbnail::number_of_thumbnails() {
                let child = timeline.first_child().unwrap();
                timeline.remove(&child);
            }
        }
    }

    fn populate_timeline(timeline: &gtk::Box) {
        // todo: see if can reuse picture widget instead of discarding. without storing ref to all of them
        // Self::remove_timeline_thumbnails(timeline);

        for path in Thumbnail::thumbnail_paths() {
            let file = gio::File::for_path(path.as_path());
            let image = gtk::Picture::for_file(&file);

            image.set_hexpand(true);
            image.set_valign(gtk::Align::Fill);
            image.set_halign(gtk::Align::Fill);
            timeline.append(&image);
        }
    }
}
