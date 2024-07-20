use gst_video::VideoOrientationMethod;
use gtk::glib;
use gtk::prelude::{ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk4::gio;
use gtk4::prelude::{
    BoxExt, ButtonExt, EventControllerExt, FileExt, GestureDragExt, GtkApplicationExt,
};
use relm4::{
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, RelmWidgetExt,
};

use crate::ui::crop_box::CropMode;
use crate::ui::timeline::{TimelineModel, TimelineMsg, TimelineOutput};
use crate::ui::CropBoxWidget;

use super::ui::edit_controls::{EditControlsModel, EditControlsOutput};
use super::ui::video_player::{FrameInfo, VideoPlayerModel, VideoPlayerMsg, VideoPlayerOutput};

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
    edit_controls: Controller<EditControlsModel>,
    timeline: Controller<TimelineModel>,
    video_is_open: bool,
    video_is_playing: bool,
    video_is_mute: bool,
    show_crop_box: bool,
    uri: Option<String>,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    AudioMute,
    AudioPlaying,
    FrameInfo(FrameInfo),
    ExportFrame,
    ExportVideo,
    OpenFile,
    SetVideo(String),
    Orient(VideoOrientationMethod),
    ShowCropBox,
    HideCropBox,
    SetCropMode(CropMode),
    CropBoxDetectHandle((f32, f32)),
    CropBoxDragUpdate((f32, f32)),
    CropBoxDragEnd,
    SeekToPercent(f64),
    UpdateSeekBarPos(f64),
    TogglePlayPause,
    ToggleMute,
    VideoLoaded,
    VideoPaused,
    VideoPlaying,
    Quit,
}

impl App {
    fn launch_file_opener(sender: &ComponentSender<Self>) {
        let filters = gio::ListStore::new::<gtk::FileFilter>();

        let video_filter = gtk::FileFilter::new();
        video_filter.add_mime_type("video/*");
        video_filter.set_name(Some("Video"));
        filters.append(&video_filter);

        let audio_filter = gtk::FileFilter::new();
        audio_filter.add_mime_type("audio/*");
        audio_filter.set_name(Some("Audio"));
        filters.append(&audio_filter);

        let file_dialog = gtk::FileDialog::builder()
            .title("Open Video")
            .accept_label("Open")
            .modal(true)
            .filters(&filters)
            .build();

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
}

#[relm4::component(pub)]
impl Component for App {
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();
    type Init = u8;
    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_default_width: 640,
            set_default_height: 600,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[name="tool_bar_view"]
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                     pack_end = &gtk::Button {
                        set_icon_name: "document-open-symbolic",
                        #[watch]
                        set_visible: model.video_is_open,
                        connect_clicked => AppMsg::OpenFile,
                    }
                },

                #[name = "content"]
                gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 10,

                    adw::StatusPage {
                        set_title: "Select Video",
                        set_description: Some("select a video file to edit"),

                        #[name = "open_file_btn"]
                        gtk::Button {
                            set_label: "Open File",
                            set_hexpand: false,
                            add_css_class: "suggested-action",
                            add_css_class: "pill",
                        },

                        #[watch]
                        set_visible: !model.video_is_open,
                    },

                    gtk::Overlay{
                        #[wrap(Some)]
                        set_child = model.video_player.widget(),
                        #[watch]
                        set_visible: model.video_is_open,

                        add_overlay: crop_box = &CropBoxWidget::default(){
                            #[watch]
                            set_visible: model.show_crop_box,
                            add_controller = gtk::GestureDrag {
                                connect_drag_begin[sender] => move |_,x,y| {
                                    sender.input(AppMsg::CropBoxDetectHandle((x as f32,y as f32)));
                                },
                                connect_drag_update[sender] => move |drag, x_offset, y_offset| {
                                    let (start_x, start_y) = drag.start_point().unwrap();

                                    let (x, y) = CropBoxWidget::get_cordinate_percent_from_drag(
                                        drag.widget().width(),
                                        drag.widget().height(),
                                        start_x + x_offset,
                                        start_y + y_offset,
                                    );

                                    sender.input(AppMsg::CropBoxDragUpdate((x,y)));
                                },
                                connect_drag_end[sender] => move |_,_,_| {
                                    sender.input(AppMsg::CropBoxDragEnd);
                                },
                             },
                        },
                    },



                    gtk::Box{
                        #[watch]
                        set_spacing: 10,
                        add_css_class: "toolbar",
                        #[watch]
                        set_visible: model.video_is_open,

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

                    model.edit_controls.widget() {
                        set_visible: false,
                    }
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
                VideoPlayerOutput::AudioMute => AppMsg::AudioMute,
                VideoPlayerOutput::AudioPlaying => AppMsg::AudioPlaying,
                VideoPlayerOutput::UpdateSeekBarPos(percent) => AppMsg::UpdateSeekBarPos(percent),
                VideoPlayerOutput::VideoLoaded => AppMsg::VideoLoaded,
                VideoPlayerOutput::VideoPlaying => AppMsg::VideoPlaying,
                VideoPlayerOutput::VideoPaused => AppMsg::VideoPaused,
            });

        let edit_controls: Controller<EditControlsModel> = EditControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                EditControlsOutput::ExportFrame => AppMsg::ExportFrame,
                EditControlsOutput::ExportVideo => AppMsg::ExportVideo,
                EditControlsOutput::OrientVideo(orientation) => AppMsg::Orient(orientation),
                EditControlsOutput::ShowCropBox => AppMsg::ShowCropBox,
                EditControlsOutput::HideCropBox => AppMsg::HideCropBox,
                EditControlsOutput::SetCropMode(mode) => AppMsg::SetCropMode(mode),
            });

        let timeline: Controller<TimelineModel> =
            TimelineModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    TimelineOutput::SeekToPercent(percent) => AppMsg::SeekToPercent(percent),
                    TimelineOutput::FrameInfo(info) => AppMsg::FrameInfo(info),
                });

        let model = Self {
            video_player,
            edit_controls,
            timeline,
            video_is_open: false,
            video_is_playing: false,
            video_is_mute: false,
            show_crop_box: false,
            uri: None,
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
            AppMsg::OpenFile => App::launch_file_opener(&sender),
            AppMsg::SetVideo(file_name) => {
                self.video_player
                    .emit(VideoPlayerMsg::NewVideo(file_name.clone()));

                self.video_is_open = true;
                self.uri.replace(file_name);

                self.video_player.widget().set_visible(true);
                self.edit_controls.widget().set_visible(true);
            }
            // todo: do directly in controller init
            AppMsg::ExportFrame => self.video_player.emit(VideoPlayerMsg::ExportFrame),
            AppMsg::ExportVideo => self.video_player.emit(VideoPlayerMsg::ExportVideo),
            AppMsg::Orient(orientation) => self
                .video_player
                .emit(VideoPlayerMsg::OrientVideo(orientation)),
            AppMsg::ShowCropBox => self.show_crop_box = true,
            AppMsg::HideCropBox => self.show_crop_box = false,
            AppMsg::SetCropMode(mode) => {
                widgets.crop_box.set_crop_mode(mode);
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDetectHandle(pos) => {
                widgets.crop_box.is_point_in_handle(pos.0, pos.1);
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDragUpdate(pos) => {
                widgets.crop_box.update_drag_pos(pos.0, pos.1);
                widgets.crop_box.queue_draw();
            }
            AppMsg::CropBoxDragEnd => {
                widgets.crop_box.set_drag_active(false);
                widgets.crop_box.queue_draw();
            }
            AppMsg::SeekToPercent(percent) => self
                .video_player
                .emit(VideoPlayerMsg::SeekToPercent(percent)),
            AppMsg::UpdateSeekBarPos(percent) => {
                self.timeline.emit(TimelineMsg::UpdateSeekBarPos(percent))
            }
            AppMsg::TogglePlayPause => self.video_player.emit(VideoPlayerMsg::TogglePlayPause),
            AppMsg::ToggleMute => self.video_player.emit(VideoPlayerMsg::ToggleMute),
            AppMsg::VideoLoaded => {
                self.timeline
                    .emit(TimelineMsg::GenerateThumbnails(self.uri.clone().unwrap()));
            }
            AppMsg::VideoPaused => self.video_is_playing = false,
            AppMsg::VideoPlaying => self.video_is_playing = true,
            AppMsg::AudioMute => self.video_is_mute = true,
            AppMsg::AudioPlaying => self.video_is_mute = false,
            AppMsg::FrameInfo(info) => {
                widgets.crop_box.set_asepct_ratio(info.aspect_ratio);
                self.video_player.emit(VideoPlayerMsg::FrameInfo(info))
            }
        }

        self.update_view(widgets, sender);
    }
}
