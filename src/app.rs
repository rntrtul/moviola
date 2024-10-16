use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gst::ClockTime;
use gtk::glib;
use gtk::prelude::{ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk4::gio;
use gtk4::prelude::{BoxExt, ButtonExt, FileExt, GtkApplicationExt, RangeExt};
use relm4::{
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, RelmWidgetExt,
};

use crate::ui::preview::{CropMode, Orientation, Preview};
use crate::ui::sidebar::sidebar::{ControlsModel, ControlsMsg, ControlsOutput};
use crate::ui::video_controls::{TimelineModel, TimelineMsg, TimelineOutput};
use crate::video::player::Player;

use super::ui::video_player::{VideoPlayerModel, VideoPlayerMsg};

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
    controls_panel: Controller<ControlsModel>,
    timeline: Controller<TimelineModel>,
    preview: Preview,
    show_video: bool,
    show_spinner: bool,
    video_selected: bool,
    video_is_loaded: bool,
    video_is_playing: bool,
    video_is_exporting: bool,
    video_is_mute: bool,
    player: Rc<RefCell<Player>>,
    uri: Option<String>,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    ExportFrame,
    ExportVideo(String),
    ExportDone,
    OpenFile,
    SaveFile,
    SetVideo(String),
    Orient(Orientation),
    ShowCropBox,
    HideCropBox,
    SetCropMode(CropMode),
    SeekToPercent(f64),
    TogglePlayPause,
    ToggleMute,
    Zoom(f64),
    ZoomTempReset,
    ZoomRestore,
    Quit,
    NewFrame(gst::Sample),
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
                set_sidebar_position: gtk::PackType::End,
                set_min_sidebar_width: 280.,

                #[wrap(Some)]
                set_sidebar = model.controls_panel.widget(),

                #[wrap(Some)]
                set_content = &adw::ToolbarView{
                    add_top_bar: header_bar = &adw::HeaderBar {
                        pack_start = &gtk::Button {
                            set_label: "Save",
                            #[watch]
                            set_visible: model.video_selected && !model.video_is_exporting,
                            add_css_class: "suggested-action",
                            connect_clicked => AppMsg::SaveFile,
                        },

                        pack_start : preview_zoom = &gtk::Scale::with_range(gtk::Orientation::Horizontal, 1f64, 4f64, 0.1f64){
                            #[watch]
                            set_visible: model.video_selected && !model.video_is_exporting,
                            set_width_request: 120,

                            connect_value_changed[sender] => move|scale| {
                                sender.input(AppMsg::Zoom(scale.value()));
                            },
                        },

                        pack_end = &gtk::Button {
                            set_icon_name: "document-open-symbolic",
                            #[watch]
                            set_visible: model.video_selected && !model.video_is_exporting,
                            connect_clicked => AppMsg::OpenFile,
                        },

                        #[wrap(Some)]
                        set_title_widget: page_switcher = &adw::ViewSwitcherBar{
                            #[watch]
                            set_visible: model.video_selected && !model.video_is_exporting,
                            set_reveal: true,
                        },
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box{
                        set_margin_all: 10,
                        set_orientation: gtk::Orientation::Vertical,

                        adw::StatusPage {
                            set_title: "Select Video",
                            set_description: Some("Choose a video file to edit"),
                            set_valign: gtk::Align::Center,
                            set_halign: gtk::Align::Center,
                            set_vexpand: true,
                            set_width_request: 250,
                            #[watch]
                            set_visible: !model.video_selected,

                            #[name = "open_file_btn"]
                            gtk::Button {
                                set_label: "Open File",
                                set_hexpand: false,
                                add_css_class: "suggested-action",
                                add_css_class: "pill",
                                connect_clicked => AppMsg::OpenFile,
                            },
                        },

                        // todo: switch to adw::spinner when relm4 targets gnome 47
                        gtk::Spinner {
                            #[watch]
                            set_spinning: model.show_spinner,
                            #[watch]
                            set_visible: model.show_spinner,
                            set_height_request: 360,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_hexpand: true,
                        },

                        gtk::Box{
                            #[watch]
                            set_visible: model.show_video,
                            model.video_player.widget() {},
                        },

                        gtk::Box{
                            #[watch]
                            set_visible: model.show_video,
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::Box{
                                #[watch]
                                set_spacing: 10,

                                gtk::Button {
                                    add_css_class: "raised",
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
                                    add_css_class: "raised",
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
        let preview = Preview::new();

        let video_player: Controller<VideoPlayerModel> =
            VideoPlayerModel::builder().launch(preview.clone()).detach();

        let controls_panel: Controller<ControlsModel> = ControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ControlsOutput::OrientVideo(orientation) => AppMsg::Orient(orientation),
                ControlsOutput::SetCropMode(mode) => AppMsg::SetCropMode(mode),
                ControlsOutput::ExportFrame => AppMsg::ExportFrame,
                ControlsOutput::ShowCropBox => AppMsg::ShowCropBox,
                ControlsOutput::HideCropBox => AppMsg::HideCropBox,
                ControlsOutput::TempResetZoom => AppMsg::ZoomTempReset,
                ControlsOutput::RestoreZoom => AppMsg::ZoomRestore,
            });

        let timeline: Controller<TimelineModel> =
            TimelineModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    TimelineOutput::SeekToPercent(percent) => AppMsg::SeekToPercent(percent),
                });

        let player = Rc::new(RefCell::new(Player::new(sender.clone())));

        let model = Self {
            video_player,
            controls_panel,
            timeline,
            preview,
            show_video: false,
            show_spinner: false,
            video_selected: false,
            video_is_loaded: false,
            video_is_playing: false,
            video_is_exporting: false,
            video_is_mute: false,
            player,
            uri: None,
        };

        let widgets = view_output!();

        model
            .controls_panel
            .model()
            .connect_switcher_to_stack(&widgets.page_switcher);

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
                self.player.borrow_mut().reset_pipeline();
                main_application().quit()
            }
            AppMsg::OpenFile => {
                self.video_selected = false;
                App::launch_file_opener(&sender)
            }
            AppMsg::SetVideo(uri) => {
                self.video_selected = true;
                self.show_video = false;
                self.show_spinner = true;

                self.video_player.widget().set_visible(false);

                self.video_is_playing = false;
                self.player.borrow_mut().set_is_playing(false);
                self.timeline.emit(TimelineMsg::Reset);
                self.preview.reset_preview();
                self.uri.replace(uri.clone());

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
            AppMsg::ExportFrame => {}
            AppMsg::SaveFile => {
                Self::launch_file_save(&sender);
            }
            AppMsg::ExportVideo(save_uri) => {
                self.video_player.widget().set_visible(false);
                self.show_video = false;
                self.show_spinner = true;

                self.video_is_exporting = true;
                let timeline_export_settings = self
                    .timeline
                    .model()
                    .get_export_settings(self.player.clone());

                self.player.borrow_mut().export_video(
                    self.uri.as_ref().unwrap().clone(),
                    save_uri,
                    timeline_export_settings,
                    self.controls_panel.model().export_settings(),
                    self.preview.export_settings(),
                    sender.clone(),
                );
            }
            AppMsg::ExportDone => {
                self.video_is_exporting = false;
                self.video_selected = false;
                self.video_is_loaded = false;
                self.show_spinner = false;
                self.show_video = false;

                self.player.borrow_mut().reset_pipeline();
                self.timeline.emit(TimelineMsg::Reset);
                // widgets.crop_box.reset_box();
            }
            AppMsg::Orient(orientation) => self.preview.set_orientation(orientation),
            AppMsg::ShowCropBox => {
                self.preview.show_crop_box();
            }
            AppMsg::HideCropBox => {
                self.preview.hide_crop_box();
            }
            AppMsg::SetCropMode(mode) => {
                self.preview.set_crop_mode(mode);
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
            AppMsg::Zoom(level) => self.preview.set_zoom(level),
            AppMsg::ZoomTempReset => {
                widgets.preview_zoom.set_sensitive(false);
                self.preview.hide_zoom();
            }
            AppMsg::ZoomRestore => {
                widgets.preview_zoom.set_sensitive(true);
                self.preview.show_zoom();
            }
            AppMsg::NewFrame(sample) => {
                self.preview.render_sample(sample);
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
                self.show_spinner = false;
                self.show_video = true;
                self.video_is_loaded = true;
                self.video_is_playing = true;

                let mut player = self.player.borrow_mut();

                player.discover_metadata();

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
                            // todo: set update rate based on video length or move to new frame callback?
                            loop {
                                tokio::time::sleep(Duration::from_millis(60)).await;
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
