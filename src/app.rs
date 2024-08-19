use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gst::ClockTime;
use gst_plugin_gtk4::Orientation;
use gtk::glib;
use gtk::prelude::{ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk4::gio;
use gtk4::prelude::{BoxExt, ButtonExt, FileExt, GestureDragExt, GtkApplicationExt};
use relm4::{
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, RelmWidgetExt,
};

use crate::ui::controls_sidebar::{ControlsModel, ControlsMsg, ControlsOutput};
use crate::ui::crop_box::CropMode;
use crate::ui::timeline::{TimelineModel, TimelineMsg, TimelineOutput};
use crate::ui::CropBoxWidget;
use crate::video::metadata::VideoInfo;
use crate::video::player::Player;

use super::ui::video_player::{VideoPlayerModel, VideoPlayerMsg, VideoPlayerOutput};

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
    controls_panel: Controller<ControlsModel>,
    timeline: Controller<TimelineModel>,
    video_selected: bool,
    video_is_loaded: bool,
    video_is_playing: bool,
    video_is_exporting: bool,
    video_is_mute: bool,
    show_crop_box: bool,
    player: Rc<RefCell<Player>>,
    uri: Option<String>,
    frame_info: Option<VideoInfo>,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    AudioMute,
    AudioPlaying,
    ExportFrame,
    ExportVideo(String),
    ExportDone,
    OpenFile,
    SaveFile,
    SetVideo(String),
    Orient(Orientation),
    ShowCropBox,
    HideCropBox,
    Rotate,
    SetCropMode(CropMode),
    CropBoxDetectHandle((f32, f32)),
    CropBoxDragUpdate((f64, f64)),
    CropBoxDragEnd,
    SeekToPercent(f64),
    TogglePlayPause,
    ToggleMute,
    VideoLoaded,
    VideoPaused,
    VideoPlaying,
    Quit,
}

#[derive(Debug)]
pub enum AppCommandMsg {
    VideoLoaded,
    AnimateSeekBar,
}

impl App {
    fn build_file_dialog() -> gtk::FileDialog {
        let filters = gio::ListStore::new::<gtk::FileFilter>();

        let video_filter = gtk::FileFilter::new();
        video_filter.add_mime_type("video/*");
        video_filter.set_name(Some("Video"));
        filters.append(&video_filter);

        gtk::FileDialog::builder()
            .title("Open Video")
            .accept_label("Open")
            .modal(true)
            .filters(&filters)
            .build()
    }

    fn launch_file_opener(sender: &ComponentSender<Self>) {
        let file_dialog = Self::build_file_dialog();

        let cancelable = gio::Cancellable::new();
        let window = relm4::main_adw_application().active_window().unwrap();

        let sender = sender.clone();
        file_dialog.open(Some(&window), Some(&cancelable), move |result| {
            let file = match result {
                Ok(f) => f,
                Err(_) => return,
            };
            sender.input(AppMsg::SetVideo(file.uri().to_string()));
        });
    }

    fn launch_file_save(sender: &ComponentSender<Self>) {
        // todo: set inital file_name with appropiate file extension
        let file_dialog = Self::build_file_dialog();
        file_dialog.set_accept_label(Some("Save"));

        let cancelable = gio::Cancellable::new();
        let window = relm4::main_adw_application().active_window().unwrap();

        let sender = sender.clone();
        file_dialog.save(Some(&window), Some(&cancelable), move |result| {
            let file = match result {
                Ok(f) => f,
                Err(_) => return,
            };
            sender.input(AppMsg::ExportVideo(file.uri().to_string()));
        });
    }

    fn display_text(time: ClockTime) -> String {
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
        let display_time = Self::display_text(timestamp);
        label.set_label(&*display_time);
    }
}

#[relm4::component(pub)]
impl Component for App {
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCommandMsg;
    type Init = u8;
    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_default_width: 1160,
            set_default_height: 600,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

             adw::OverlaySplitView{
                set_pin_sidebar: true,
                #[watch]
                set_show_sidebar: model.video_selected,
                set_sidebar_position: gtk::PackType::Start,
                set_min_sidebar_width: 280.,

                #[wrap(Some)]
                set_sidebar = model.controls_panel.widget(),

                #[wrap(Some)]
                set_content = &adw::ToolbarView{
                    add_top_bar = &adw::HeaderBar {
                      pack_end = &gtk::Button {
                            set_icon_name: "document-open-symbolic",
                            #[watch]
                            set_visible: model.video_selected && !model.video_is_exporting,
                            connect_clicked => AppMsg::OpenFile,
                        }
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box{
                        set_margin_all: 10,
                        set_orientation: gtk::Orientation::Vertical,

                        adw::StatusPage {
                            set_title: "Select Video",
                            set_description: Some("select a video file to edit"),
                            #[watch]
                            set_visible: !model.video_selected,

                            #[name = "open_file_btn"]
                            gtk::Button {
                                set_label: "Open File",
                                set_hexpand: false,
                                add_css_class: "suggested-action",
                                add_css_class: "pill",
                            },
                        },

                        gtk::Spinner {
                            set_height_request: 360,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_hexpand: true,
                            #[watch]
                            set_spinning: (model.video_selected && !model.video_is_loaded) || model.video_is_exporting,
                            #[watch]
                            set_visible: (model.video_selected && !model.video_is_loaded) || model.video_is_exporting,
                        },

                        gtk::Overlay{
                            #[watch]
                            set_visible: model.video_is_loaded,
                            #[wrap(Some)]
                            set_child = model.video_player.widget(),

                            add_overlay: crop_box = &CropBoxWidget::default(){
                                #[watch]
                                set_visible: model.show_crop_box,
                                add_controller = gtk::GestureDrag {
                                    connect_drag_begin[sender] => move |_,x,y| {
                                        sender.input(AppMsg::CropBoxDetectHandle((x as f32,y as f32)));
                                    },
                                    connect_drag_update[sender] => move |drag, x_offset, y_offset| {
                                        let (start_x, start_y) = drag.start_point().unwrap();

                                        let x = start_x + x_offset;
                                        let y = start_y + y_offset;

                                        sender.input(AppMsg::CropBoxDragUpdate((x,y)));
                                    },
                                    connect_drag_end[sender] => move |_,_,_| {
                                        sender.input(AppMsg::CropBoxDragEnd);
                                    },
                                },
                            },
                        },

                        gtk::Box{
                            set_orientation: gtk::Orientation::Vertical,
                            #[watch]
                            set_visible: model.video_is_loaded && !model.video_is_exporting,

                            gtk::Box{
                                #[watch]
                                set_spacing: 10,
                                add_css_class: "toolbar",

                                gtk::Button {
                                    #[watch]
                                    set_icon_name: if model.video_is_playing {
                                        "pause"
                                    } else {
                                        "play"
                                    },

                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::TogglePlayPause)
                                    }
                                },

                                model.timeline.widget() {},

                                gtk::Button {
                                    #[watch]
                                     set_icon_name: if model.video_is_mute {
                                        "audio-volume-muted"
                                    } else {
                                        "audio-volume-high"
                                    },
                                    connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::ToggleMute)
                                    }
                                },
                            },

                            gtk::Box{
                                set_halign: gtk::Align::Center,
                                #[name = "position_label"]
                                gtk::Label {
                                    add_css_class: "monospace"
                                },
                                gtk::Label {
                                    add_css_class: "dim-label",
                                    set_label: " / "
                                },
                                #[name = "duration_label"]
                                gtk::Label {
                                    set_css_classes: &["monospace", "dim-label"]
                                },
                            },
                        },
                    },
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let video_player: Controller<VideoPlayerModel> = VideoPlayerModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                VideoPlayerOutput::ToggleVideoPlay => AppMsg::TogglePlayPause,
            });

        let controls_panel: Controller<ControlsModel> = ControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ControlsOutput::OrientVideo(orientation) => AppMsg::Orient(orientation),
                ControlsOutput::SetCropMode(mode) => AppMsg::SetCropMode(mode),
                ControlsOutput::ExportFrame => AppMsg::ExportFrame,
                ControlsOutput::ShowCropBox => AppMsg::ShowCropBox,
                ControlsOutput::HideCropBox => AppMsg::HideCropBox,
                ControlsOutput::SaveFile => AppMsg::SaveFile,
            });

        let timeline: Controller<TimelineModel> =
            TimelineModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    TimelineOutput::SeekToPercent(percent) => AppMsg::SeekToPercent(percent),
                });

        let player = Rc::new(RefCell::new(Player::new(video_player.model().sink())));

        let model = Self {
            video_player,
            controls_panel,
            timeline,
            video_selected: false,
            video_is_loaded: false,
            video_is_playing: false,
            video_is_exporting: false,
            video_is_mute: false,
            show_crop_box: false,
            player,
            uri: None,
            frame_info: None,
        };

        let widgets = view_output!();

        widgets.open_file_btn.connect_clicked(move |_| {
            sender.input(AppMsg::OpenFile);
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
            AppMsg::Quit => {
                println!("QUIT");
                main_application().quit()
            }
            AppMsg::OpenFile => {
                self.video_selected = false;
                App::launch_file_opener(&sender)
            }
            AppMsg::SetVideo(uri) => {
                self.video_selected = true;

                self.video_player.widget().set_visible(false);

                self.video_is_playing = false;
                if self.player.borrow_mut().is_playing() {
                    self.player.borrow_mut().set_is_playing(false);
                }
                self.uri.replace(uri.clone());
                widgets.crop_box.reset_box();

                self.timeline
                    .emit(TimelineMsg::GenerateThumbnails(uri.clone()));

                self.player
                    .borrow_mut()
                    .play_uri(self.uri.as_ref().unwrap().clone());

                let bus = self.player.borrow_mut().pipeline_bus();
                sender.oneshot_command(async move {
                    Player::wait_for_pipeline_init(bus);
                    AppCommandMsg::VideoLoaded
                });
            }
            AppMsg::ExportFrame => {
                self.player.borrow_mut().export_frame();
            }
            AppMsg::SaveFile => {
                Self::launch_file_save(&sender);
            }
            AppMsg::ExportVideo(save_uri) => {
                self.video_player.widget().set_visible(false);

                self.video_is_exporting = true;
                let timeline_export_settings = self
                    .timeline
                    .model()
                    .get_export_settings(self.player.clone());

                let export_settings = self.controls_panel.model().export_settings();

                self.player.borrow_mut().export_video(
                    self.uri.as_ref().unwrap().clone(),
                    save_uri,
                    timeline_export_settings,
                    export_settings,
                    sender.clone(),
                );
            }
            AppMsg::ExportDone => {
                self.video_is_exporting = false;
                self.video_selected = false;
                self.video_is_loaded = false;

                self.player.borrow_mut().reset_pipeline();
                self.timeline.emit(TimelineMsg::Reset);
                widgets.crop_box.reset_box();
            }
            AppMsg::Orient(orientation) => {
                let is_vertical = matches!(
                    orientation,
                    Orientation::Rotate90
                        | Orientation::Rotate270
                        | Orientation::FlipRotate90
                        | Orientation::FlipRotate270
                );
                widgets.crop_box.set_is_preview_rotated(is_vertical);
                widgets.crop_box.queue_draw();

                self.player.borrow_mut().set_video_orientation(orientation)
            }
            AppMsg::ShowCropBox => {
                self.show_crop_box = true;
                self.player.borrow_mut().remove_crop();
            }
            AppMsg::HideCropBox => {
                let crop_box = &widgets.crop_box;

                // todo: make function crop_box.has_changed()
                if crop_box.left_x() == 0f32
                    && crop_box.top_y() == 0f32
                    && crop_box.bottom_y() == 1f32
                    && crop_box.right_x() == 1f32
                {
                    println!("Can skip adding effect");
                } else {
                    let width = self.frame_info.as_ref().unwrap().width as f32;
                    let height = self.frame_info.as_ref().unwrap().height as f32;

                    let left = (width * crop_box.left_x()) as i32;
                    let top = (height * crop_box.top_y()) as i32;
                    let right = (width - (width * crop_box.right_x())) as i32;
                    let bottom = (height - (height * crop_box.bottom_y())) as i32;

                    self.player
                        .borrow_mut()
                        .set_video_crop(left, top, right, bottom);
                }

                self.show_crop_box = false
            }
            AppMsg::SetCropMode(mode) => {
                widgets.crop_box.set_crop_mode(mode);
                widgets.crop_box.maintain_aspect_ratio();
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDetectHandle(pos) => {
                widgets.crop_box.is_point_in_handle(pos.0, pos.1);
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDragUpdate(target) => {
                widgets.crop_box.update_drag_pos(target);
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDragEnd => {
                widgets.crop_box.set_drag_active(false);
                widgets.crop_box.queue_draw();
            }
            AppMsg::SeekToPercent(percent) => {
                let timestamp = ClockTime::from_nseconds(
                    (self.player.borrow().info.duration.nseconds() as f64 * percent) as u64,
                );
                Self::update_label_timestamp(timestamp, &widgets.position_label);
                self.player.borrow_mut().seek(timestamp);
            }
            AppMsg::TogglePlayPause => {
                let mut player = self.player.borrow_mut();
                player.toggle_play_plause();
                self.video_is_playing = player.is_playing();
            }
            AppMsg::ToggleMute => {
                let mut player = self.player.borrow_mut();
                player.toggle_mute();
                self.video_is_mute = player.is_mute();
            }
            AppMsg::VideoLoaded => {
                self.timeline
                    .emit(TimelineMsg::GenerateThumbnails(self.uri.clone().unwrap()));
            }
            AppMsg::VideoPaused => self.video_is_playing = false,
            AppMsg::VideoPlaying => self.video_is_playing = true,
            AppMsg::AudioMute => self.video_is_mute = true,
            AppMsg::AudioPlaying => self.video_is_mute = false,
            AppMsg::Rotate => self.controls_panel.emit(ControlsMsg::Rotate),
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
            AppCommandMsg::AnimateSeekBar => {
                let player = self.player.borrow();
                let curr_position = player.position();

                if !self.video_is_playing
                    || !player.is_playing()
                    || curr_position == ClockTime::ZERO
                {
                    return;
                }

                Self::update_label_timestamp(curr_position, &widgets.position_label);

                let percent =
                    curr_position.mseconds() as f64 / player.info().duration.mseconds() as f64;
                self.timeline.emit(TimelineMsg::UpdateSeekBarPos(percent));
            }
            AppCommandMsg::VideoLoaded => {
                self.video_is_loaded = true;
                self.video_is_playing = true;

                let mut player = self.player.borrow_mut();

                player.discover_metadata();

                // todo: probably don't need this in self?
                self.frame_info = Some(player.info());
                widgets
                    .crop_box
                    .set_asepct_ratio(player.info().aspect_ratio);
                self.controls_panel.emit(ControlsMsg::DefaultCodec(
                    player.info.container_info.clone(),
                ));

                Self::update_label_timestamp(player.info.duration, &widgets.duration_label);

                self.video_player.widget().set_visible(true);
                player.set_is_playing(true);

                self.video_player.emit(VideoPlayerMsg::VideoLoaded);

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            loop {
                                tokio::time::sleep(Duration::from_millis(125)).await;
                                out.send(AppCommandMsg::AnimateSeekBar).unwrap();
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }

        self.update_view(widgets, sender);
    }
}
