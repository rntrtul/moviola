use crate::ui::video_controls::handle::{HANDLE_HEIGHT, HANDLE_WIDTH};
use crate::video::export::TimelineExportSettings;
use crate::video::player::Player;
use crate::video::thumbnail::Thumbnail;
use gst::ClockTime;
use gtk4::prelude::{BoxExt, ButtonExt, EventControllerExt, GestureDragExt, WidgetExt};
use gtk4::{gio, ContentFit};
use relm4::{adw, gtk, Component, ComponentParts, ComponentSender};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug)]
pub struct VideoControlModel {
    thumbnails_available: bool,
    video_is_playing: bool,
    video_is_mute: bool,
    start: f32,
    end: f32,
    prev_drag_target: f64,
    player: Rc<RefCell<Player>>,
}

#[derive(Debug)]
pub enum VideoControlMsg {
    VideoLoaded,
    GenerateThumbnails(String),
    PopulateTimeline,
    DragBegin(f64, f64),
    DragUpdate(f64),
    DragEnd,
    UpdateSeekBarPos(f64),
    SeekToPercent(f64),
    TogglePlayPause,
    ToggleMute,
    Reset,
}

#[derive(Debug)]
pub enum VideoControlOutput {
    Seek(ClockTime),
    TogglePlayPause,
    ToggleMute,
}

#[derive(Debug)]
pub enum VideoControlCmdMsg {
    ThumbnailsGenerated,
    AnimateSeekBar,
    UpdateCurrentTime,
}

#[relm4::component(pub)]
impl Component for VideoControlModel {
    type CommandOutput = VideoControlCmdMsg;
    type Input = VideoControlMsg;
    type Output = VideoControlOutput;
    type Init = Rc<RefCell<Player>>;

    view! {
        adw::Clamp {
            set_maximum_size: 600,

            gtk::Box{
                set_spacing: 10,

                #[name = "position_label"]
                gtk::Label {
                    add_css_class: "monospace"
                },

                gtk::Button {
                    add_css_class: "raised",
                    #[watch]
                    set_icon_name: if model.video_is_playing {
                        "pause"
                    } else {
                        "play"
                    },
                    connect_clicked => VideoControlMsg::TogglePlayPause,
                },

                gtk::Overlay {
                    #[wrap(Some)]
                    set_child: timeline = &gtk::Box {
                        set_hexpand: true,
                        set_margin_start: HANDLE_WIDTH as i32,
                        set_margin_end: HANDLE_WIDTH as i32,
                        set_margin_top: HANDLE_HEIGHT as i32,
                        set_margin_bottom: HANDLE_HEIGHT as i32,
                    },

                    add_overlay: seek_bar = &super::HandleWidget::default() {
                        add_controller = gtk::GestureDrag {
                            connect_drag_begin[sender] => move |_,x,y| {
                                sender.input(VideoControlMsg::DragBegin(x, y))
                            },

                            connect_drag_update[sender] => move |drag,offset_x,_| {
                                let (start_x, _) = drag.start_point().unwrap();
                                let targ_x = start_x + offset_x;
                                sender.input(VideoControlMsg::DragUpdate(targ_x))
                            },

                            connect_drag_end[sender] => move |_, _,_| {
                                sender.input(VideoControlMsg::DragEnd);
                            },
                        },

                         add_controller = gtk::GestureClick {
                            connect_pressed[sender] => move |click,_,x,_| {
                                let width = click.widget().unwrap().width() as f64;
                                let percent = x / width;
                                sender.input(VideoControlMsg::SeekToPercent(percent));
                            }
                        },
                    },
                },

                gtk::Button {
                    add_css_class: "raised",
                    #[watch]
                    set_icon_name: if model.video_is_mute {
                        "audio-volume-muted"
                    } else {
                        "audio-volume-high"
                    },
                    connect_clicked => VideoControlMsg::ToggleMute,
                },

                #[name = "duration_label"]
                gtk::Label {
                    set_css_classes: &["monospace", "dim-label"]
                },
            }
        }
    }

    fn init(
        player: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = VideoControlModel {
            thumbnails_available: false,
            video_is_mute: false,
            video_is_playing: true,
            start: 0.,
            end: 1.,
            prev_drag_target: -1.,
            player,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        tokio::time::sleep(Duration::from_millis(60)).await;
                        out.send(VideoControlCmdMsg::AnimateSeekBar).unwrap();
                    }
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        out.send(VideoControlCmdMsg::UpdateCurrentTime).unwrap();
                    }
                })
                .drop_on_shutdown()
        });

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
            VideoControlMsg::TogglePlayPause => {
                self.video_is_playing = !self.video_is_playing;
                sender.output(VideoControlOutput::TogglePlayPause).unwrap()
            }
            VideoControlMsg::ToggleMute => {
                self.video_is_mute = !self.video_is_mute;
                sender.output(VideoControlOutput::ToggleMute).unwrap();
            }
            VideoControlMsg::GenerateThumbnails(uri) => {
                Self::remove_timeline_thumbnails(&widgets.timeline);
                self.thumbnails_available = false;

                sender.oneshot_command(async move {
                    Thumbnail::generate_thumbnails(uri).await;
                    VideoControlCmdMsg::ThumbnailsGenerated
                });
            }
            VideoControlMsg::PopulateTimeline => {
                let timeline = &widgets.timeline;
                VideoControlModel::populate_timeline(timeline);
            }
            VideoControlMsg::SeekToPercent(percent) => {
                widgets.seek_bar.set_seek_x(percent as f32);

                let timestamp = ClockTime::from_nseconds(
                    (self.player.borrow().info.duration.nseconds() as f64 * percent) as u64,
                );
                Self::update_label_timestamp(timestamp, &widgets.position_label);

                widgets.seek_bar.queue_draw();
                sender.output(VideoControlOutput::Seek(timestamp)).unwrap();
            }
            VideoControlMsg::UpdateSeekBarPos(percent) => {
                widgets.seek_bar.set_seek_x(percent as f32);
                widgets.seek_bar.queue_draw();
            }
            VideoControlMsg::DragBegin(x, y) => {
                widgets.seek_bar.drag_start(x, y);
            }
            VideoControlMsg::DragUpdate(target_x) => {
                widgets.seek_bar.drag_update(target_x as f32);
                widgets.seek_bar.queue_draw();

                if target_x != self.prev_drag_target {
                    sender.input(VideoControlMsg::SeekToPercent(
                        widgets.seek_bar.seek_x() as f64
                    ));
                    self.prev_drag_target = target_x;
                }
            }
            VideoControlMsg::DragEnd => {
                widgets.seek_bar.drag_end();
                self.start = widgets.seek_bar.start_x();
                self.end = widgets.seek_bar.end_x();
                self.prev_drag_target = -1.;
            }
            VideoControlMsg::VideoLoaded => {
                self.video_is_playing = true;
                Self::update_label_timestamp(
                    self.player.borrow().info.duration,
                    &widgets.duration_label,
                );
            }
            VideoControlMsg::Reset => {
                self.start = 0f32;
                self.end = 1f32;
                self.prev_drag_target = -1.;
                widgets.seek_bar.reset();
            }
        }
        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            VideoControlCmdMsg::ThumbnailsGenerated => {
                self.thumbnails_available = true;
                sender.input(VideoControlMsg::PopulateTimeline);
            }
            VideoControlCmdMsg::AnimateSeekBar => {
                let player = self.player.borrow();
                let Ok(curr_position) = player.position() else {
                    return;
                };

                let percent =
                    curr_position.mseconds() as f64 / player.info().duration.mseconds() as f64;
                sender.input(VideoControlMsg::UpdateSeekBarPos(percent));
            }
            VideoControlCmdMsg::UpdateCurrentTime => {
                let Ok(curr_position) = self.player.borrow().position() else {
                    return;
                };

                Self::update_label_timestamp(curr_position, &widgets.position_label);
            }
        }
        self.update_view(widgets, sender);
    }
}

impl VideoControlModel {
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
        for path in Thumbnail::thumbnail_paths() {
            let file = gio::File::for_path(path.as_path());
            let image = gtk::Picture::for_file(&file);

            image.set_content_fit(ContentFit::Cover);
            image.set_hexpand(true);
            image.set_valign(gtk::Align::Fill);
            image.set_halign(gtk::Align::Fill);
            timeline.append(&image);
        }
    }

    fn clock_time_display_text(time: ClockTime) -> String {
        let seconds = time.seconds() % 60;
        let minutes = time.minutes() % 60;
        let hours = time.hours();

        if hours > 0 {
            format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
        } else {
            format!("{:0>2}:{:0>2}", minutes, seconds)
        }
    }

    fn update_label_timestamp(timestamp: ClockTime, label: &gtk::Label) {
        let display_time = Self::clock_time_display_text(timestamp);
        label.set_label(&*display_time);
    }
}
