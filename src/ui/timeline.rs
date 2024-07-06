use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::{BoxExt, EventControllerExt, GestureDragExt, WidgetExt};
use relm4::*;
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::ui::handle_manager::HandleManager;
use crate::ui::thumbnail_manager::ThumbnailManager;

#[derive(Debug)]
pub struct TimelineModel {
    thumbnails_available: bool,
    handle_manager: Option<HandleManager>,
}

#[derive(Debug)]
pub enum TimelineMsg {
    GenerateThumbnails(String),
    PopulateTimeline,
    MoveStartTo(i32),
    MoveStartEnd,
    MoveEndTo(i32),
    MoveEndEnd,
    UpdateSeekBarPos(f64),
    SeekToPercent(f64),
}

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
                inline_css: "background-color: grey",
                set_margin_start: 5,
                set_margin_end: 5,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |click,_,x,_| {
                        let width = click.widget().width() as f64;
                        let percent = x / width;
                        sender.input(TimelineMsg::SeekToPercent(percent));
                    }
                },

                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,x_offset,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let width = drag.widget().width() as f64;
                        let percent_dragged = (start_x + x_offset) / width;

                        sender.input(TimelineMsg::SeekToPercent(percent_dragged));
                    },
                }
            },

            add_overlay: start_handle = &super::HandleWidget::default() {
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Center,

                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,offset_x,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let targ_x = (start_x + offset_x) as i32;
                        sender.input(TimelineMsg::MoveStartTo(targ_x))
                    },

                    connect_drag_end[sender] => move |_, _,_| {
                        sender.input(TimelineMsg::MoveStartEnd);
                    },
                }
            },

            add_overlay: end_handle = &super::HandleWidget::new(0, true, false) {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,

                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,offset_x,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let targ_x = (start_x + offset_x) as i32;
                        sender.input(TimelineMsg::MoveEndTo(targ_x))
                    },

                    connect_drag_end[sender] => move |_, _,_| {
                        sender.input(TimelineMsg::MoveEndEnd);
                    },
                }
            },

            add_overlay: seek_bar = &super::HandleWidget::new(0, false, false) {
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Center,
            },
        },
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = TimelineModel {
            thumbnails_available: false,
            handle_manager: None,
        };

        let widgets = view_output!();

        let handle_manager = HandleManager {
            start_handle: Rc::new(widgets.start_handle.clone()),
            end_handle: Rc::new(widgets.end_handle.clone()),
        };

        handle_manager.set_end_pos(1.0);

        model.handle_manager = Some(handle_manager);

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
                    ThumbnailManager::generate_thumbnails(uri).await;
                    TimelineCmdMsg::ThumbnailsGenerated
                });
            }
            TimelineMsg::PopulateTimeline => {
                let timeline = &widgets.timeline;
                TimelineModel::populate_timeline(timeline);
            }
            TimelineMsg::SeekToPercent(percent) => {
                let seek_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if seek_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(seek_bar_pos);
                    sender
                        .output(TimelineOutput::SeekToPercent(percent))
                        .unwrap();
                }
            }
            TimelineMsg::UpdateSeekBarPos(percent) => {
                let target_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if target_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(target_bar_pos);
                }
            }
            TimelineMsg::MoveStartTo(pos) => {
                if self
                    .handle_manager
                    .as_ref()
                    .unwrap()
                    .try_set_start_rel_x(pos, widgets.timeline.width())
                {
                    let seek_percent =
                        widgets.start_handle.target_x() as f64 / widgets.timeline.width() as f64;
                    self.handle_manager
                        .as_ref()
                        .unwrap()
                        .set_start_pos(seek_percent);
                    sender.input(TimelineMsg::SeekToPercent(seek_percent));
                }
            }
            TimelineMsg::MoveEndTo(pos) => {
                if self
                    .handle_manager
                    .as_ref()
                    .unwrap()
                    .try_set_end_rel_x(pos, widgets.timeline.width())
                {
                    let seek_percent =
                        widgets.end_handle.target_x() as f64 / widgets.timeline.width() as f64;
                    self.handle_manager
                        .as_ref()
                        .unwrap()
                        .set_end_pos(seek_percent);
                    sender.input(TimelineMsg::SeekToPercent(seek_percent));
                }
            }
            TimelineMsg::MoveStartEnd => self.handle_manager.as_ref().unwrap().set_start_margin(),
            TimelineMsg::MoveEndEnd => self.handle_manager.as_ref().unwrap().set_end_margin(),
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
    pub fn get_target_start_percent(&self) -> f64 {
        self.handle_manager
            .as_ref()
            .unwrap()
            .start_handle
            .percent_pos()
    }

    pub fn get_target_end_percent(&self) -> f64 {
        self.handle_manager
            .as_ref()
            .unwrap()
            .end_handle
            .percent_pos()
    }

    fn remove_timeline_thumbnails(timeline: &gtk::Box) {
        if timeline.first_child().is_some() {
            for _ in 0..ThumbnailManager::get_number_of_thumbnails() {
                let child = timeline.first_child().unwrap();
                timeline.remove(&child);
            }
        }
    }

    fn populate_timeline(timeline: &gtk::Box) {
        // todo: see if can reuse picture widget instead of discarding. without storing ref to all of them
        // Self::remove_timeline_thumbnails(timeline);

        for path in ThumbnailManager::get_thumbnail_paths() {
            let file = gio::File::for_parse_name(path.as_str());
            let image = gtk::Picture::for_file(&file);

            image.set_hexpand(true);
            image.set_valign(gtk::Align::Fill);
            image.set_halign(gtk::Align::Fill);
            timeline.append(&image);
        }
    }
}
