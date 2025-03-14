use crate::renderer::renderer::RenderedFrame;
use crate::renderer::{
    EffectParameters, FramePosition, FrameSize, RenderCmd, RenderMode, RenderResopnse,
    RendererHandler,
};
use crate::ui::preview::preview_frame::{PreviewFrameModel, PreviewFrameMsg, PreviewFrameOutput};
use crate::ui::preview::{CropMode, Orientation};
use crate::ui::sidebar::sidebar::{ControlsModel, ControlsMsg, ControlsOutput};
use crate::ui::video_controls::{VideoControlModel, VideoControlMsg, VideoControlOutput};
use crate::video::metadata::VideoInfo;
use crate::video::player::Player;
use gst::ClockTime;
use gtk::prelude::{ApplicationExt, WidgetExt};
use relm4::gtk::prelude::{
    ButtonExt, FileExt, GtkApplicationExt, GtkWindowExt, OrientableExt, RangeExt,
};
use relm4::gtk::{gio, glib};
use relm4::{
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, RelmWidgetExt,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

pub(super) struct App {
    renderer: RendererHandler,
    preview_frame: Controller<PreviewFrameModel>,
    sidebar_panel: Controller<ControlsModel>,
    video_controls: Controller<VideoControlModel>,
    player: Rc<RefCell<Player>>,
    show_video: bool,
    show_spinner: bool,
    video_selected: bool,
    video_is_loaded: bool,
    video_is_exporting: bool,
    uri: Option<String>,
    export_sender: Option<mpsc::Sender<RenderedFrame>>,
    export_video_decode_finished: bool,
    frames_exported: u32,
    export_target_frame_count: u32,
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
    StraightenBegin,
    Straighten(f64),
    StraightenEnd,
    ShowCropBox,
    HideCropBox,
    SetCropMode(CropMode),
    EffectsChanged(EffectParameters),
    Seek(ClockTime),
    // fixme: get better names for these 2
    TogglePlayPause,
    TogglePlayPauseRequested,
    ToggleMute,
    Zoom(f64),
    ZoomTempReset,
    ZoomRestore,
    Quit,
    VideoFinished,
}

#[derive(Debug)]
pub enum AppCommandMsg {
    InitWithvideo,
    VideoLoaded,
    FrameRendered(RenderedFrame),
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
            sender.input(AppMsg::ExportVideo(
                file.path().unwrap().to_str().unwrap().to_string(),
            ));
        });
    }

    fn export_frame_position(&self) -> FramePosition {
        // todo: convert crop percents into pixel values.
        let crop_settings = self.preview_frame.model().export_settings();
        let (orientation, angel) = self.sidebar_panel.model().orientation_and_angle();
        let info = self.player.borrow().info();
        let size = FrameSize::new(info.width, info.height);

        let mut position = FramePosition::new(size);
        position.orientation = orientation;
        position.straigthen_angle = angel.to_radians();

        position
    }
}

#[relm4::component(pub)]
impl Component for App {
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCommandMsg;
    type Init = Option<String>;
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
                set_sidebar = model.sidebar_panel.widget(),

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

                        adw::Spinner{
                            #[watch]
                            set_visible: model.show_spinner,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_height_request: 64,
                            set_vexpand: true,
                        },

                        gtk::Box{
                            #[watch]
                            set_visible: model.show_video,
                            model.preview_frame.widget() {},
                        },

                        gtk::Box{
                            #[watch]
                            set_visible: model.show_video,
                            model.video_controls.widget() {},
                        },
                    },
                },
            }
        }
    }

    fn init(
        path: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (handler, render_response) = RendererHandler::new(RenderMode::MostRecentFrame);

        let preview_frame: Controller<PreviewFrameModel> = PreviewFrameModel::builder()
            .launch(handler.timer_cmd_sender())
            .forward(sender.input_sender(), |msg| match msg {
                PreviewFrameOutput::TogglePlayPause => AppMsg::TogglePlayPauseRequested,
            });

        let player = Rc::new(RefCell::new(Player::new(
            sender.clone(),
            handler.render_cmd_sender(),
            handler.timer_cmd_sender(),
        )));

        let controls_panel: Controller<ControlsModel> = ControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ControlsOutput::OrientVideo(orientation) => AppMsg::Orient(orientation),
                ControlsOutput::StraightenBegin => AppMsg::StraightenBegin,
                ControlsOutput::Straigten(angle) => AppMsg::Straighten(angle),
                ControlsOutput::StraightenEnd => AppMsg::StraightenEnd,
                ControlsOutput::SetCropMode(mode) => AppMsg::SetCropMode(mode),
                ControlsOutput::ExportFrame => AppMsg::ExportFrame,
                ControlsOutput::ShowCropBox => AppMsg::ShowCropBox,
                ControlsOutput::HideCropBox => AppMsg::HideCropBox,
                ControlsOutput::TempResetZoom => AppMsg::ZoomTempReset,
                ControlsOutput::RestoreZoom => AppMsg::ZoomRestore,
                ControlsOutput::EffectsChanged(params) => AppMsg::EffectsChanged(params),
            });

        let timeline: Controller<VideoControlModel> = VideoControlModel::builder()
            .launch(player.clone())
            .forward(sender.input_sender(), |msg| match msg {
                VideoControlOutput::Seek(timestamp) => AppMsg::Seek(timestamp),
                VideoControlOutput::TogglePlayPause => AppMsg::TogglePlayPause,
                VideoControlOutput::ToggleMute => AppMsg::ToggleMute,
            });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        let Ok(response) = render_response.recv() else {
                            break;
                        };

                        let app_msg = match response {
                            RenderResopnse::FrameRendered(frame) => {
                                AppCommandMsg::FrameRendered(frame)
                            }
                        };

                        out.send(app_msg).unwrap();
                    }
                })
                .drop_on_shutdown()
        });

        let model = Self {
            renderer: handler,
            preview_frame,
            sidebar_panel: controls_panel,
            video_controls: timeline,
            show_video: false,
            show_spinner: false,
            video_selected: false,
            video_is_loaded: false,
            video_is_exporting: false,
            player,
            uri: path,
            export_sender: None,
            export_video_decode_finished: false,
            frames_exported: 0,
            export_target_frame_count: 0,
        };

        let widgets = view_output!();

        model
            .sidebar_panel
            .model()
            .connect_switcher_to_stack(&widgets.page_switcher);

        if model.uri.is_some() {
            sender.oneshot_command(async move {
                // fixme: find way to wait for widgets to size and init instead of sleeping
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                AppCommandMsg::InitWithvideo
            });
        }

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
            AppMsg::OpenFile => App::launch_file_opener(&sender),
            AppMsg::SetVideo(uri) => {
                self.video_selected = true;
                self.show_video = false;
                self.show_spinner = true;

                self.preview_frame.widget().set_visible(false);

                self.player.borrow_mut().set_is_playing(false);
                self.video_controls.emit(VideoControlMsg::Reset);
                self.preview_frame.model().reset();
                self.uri.replace(uri.clone());

                self.video_controls
                    .emit(VideoControlMsg::GenerateThumbnails(uri.clone()));

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
            AppMsg::SaveFile => Self::launch_file_save(&sender),
            AppMsg::ExportVideo(save_uri) => {
                self.preview_frame.widget().set_visible(false);
                self.show_video = false;
                self.show_spinner = true;

                self.player.borrow_mut().set_is_playing(false);
                self.renderer
                    .send_render_cmd(RenderCmd::ChangeRenderMode(RenderMode::AllFrames));

                let position = self.export_frame_position();
                self.renderer
                    .send_render_cmd(RenderCmd::PositionFrame(position));

                let timeline_export_settings = self
                    .video_controls
                    .model()
                    .get_export_settings(self.player.clone());

                let (tex_sender, receiver) = mpsc::channel();
                self.export_sender = Some(tex_sender);

                let duration = timeline_export_settings.duration();
                let framerate = self.player.borrow().info.framerate;
                let fps = framerate.numer() as f64 / framerate.denom() as f64;
                let target_frames = (duration.seconds_f64() * fps).ceil() as u32;
                println!("for duration: {duration} @ {fps}fps. total frames: {target_frames}",);
                self.export_target_frame_count = target_frames;
                self.frames_exported = 0;

                self.player.borrow_mut().export_video(
                    self.uri.as_ref().unwrap().clone(),
                    save_uri,
                    timeline_export_settings,
                    self.sidebar_panel.model().export_settings(),
                    position.output_frame_size(),
                    sender.clone(),
                    receiver,
                );
                self.video_is_exporting = true;
            }
            AppMsg::ExportDone => {
                self.video_is_exporting = false;
                self.export_video_decode_finished = false;
                self.video_selected = false;
                self.video_is_loaded = false;
                self.show_spinner = false;
                self.show_video = false;

                self.player.borrow_mut().reset_pipeline();
                self.video_controls.emit(VideoControlMsg::Reset);
                self.export_sender = None;
                self.renderer
                    .send_render_cmd(RenderCmd::ChangeRenderMode(RenderMode::MostRecentFrame));
            }
            AppMsg::TogglePlayPauseRequested => {
                self.video_controls.emit(VideoControlMsg::TogglePlayPause)
            }
            AppMsg::Seek(timestamp) => self.player.borrow().seek(timestamp),
            AppMsg::TogglePlayPause => self.player.borrow_mut().toggle_play_plause(),
            AppMsg::ToggleMute => self.player.borrow_mut().toggle_mute(),
            AppMsg::VideoFinished => {
                if self.video_is_exporting {
                    self.export_video_decode_finished = true;
                } else {
                    self.player.borrow_mut().set_is_finished()
                }
            }
            AppMsg::Orient(orientation) => {
                self.renderer
                    .send_render_cmd(RenderCmd::UpdateOrientation(orientation));

                if !self.player.borrow().is_playing() {
                    self.renderer.send_render_cmd(RenderCmd::RenderFrame);
                }
            }
            AppMsg::StraightenBegin => self.preview_frame.emit(PreviewFrameMsg::StraightenStart),
            AppMsg::Straighten(angle) => {
                self.preview_frame.emit(PreviewFrameMsg::Straighten(angle))
            }
            AppMsg::StraightenEnd => self.preview_frame.emit(PreviewFrameMsg::StraightenEnd),
            AppMsg::ShowCropBox => self.preview_frame.emit(PreviewFrameMsg::CropBoxShow),
            AppMsg::HideCropBox => self.preview_frame.emit(PreviewFrameMsg::CropBoxHide),
            AppMsg::SetCropMode(mode) => self.preview_frame.emit(PreviewFrameMsg::CropMode(mode)),
            AppMsg::Zoom(level) => self.preview_frame.emit(PreviewFrameMsg::Zoom(level)),
            AppMsg::ZoomTempReset => {
                widgets.preview_zoom.set_sensitive(false);
                self.preview_frame.emit(PreviewFrameMsg::ZoomHide)
            }
            AppMsg::ZoomRestore => {
                widgets.preview_zoom.set_sensitive(true);
                self.preview_frame.emit(PreviewFrameMsg::ZoomShow)
            }
            AppMsg::EffectsChanged(params) => {
                self.renderer
                    .send_render_cmd(RenderCmd::UpdateEffects(params));

                if !self.player.borrow().is_playing() {
                    self.renderer.send_render_cmd(RenderCmd::RenderFrame);
                }
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
            AppCommandMsg::VideoLoaded => {
                self.show_spinner = false;
                self.show_video = true;
                self.video_is_loaded = true;

                let mut player = self.player.borrow_mut();

                player.discover_metadata();

                self.sidebar_panel.emit(ControlsMsg::VideoLoaded((
                    player.info.container_info.clone(),
                    player.info.orientation,
                )));

                player.set_is_playing(true);

                let (width, height) = preview_size(&player.info);

                self.renderer
                    .send_render_cmd(RenderCmd::UpdateOutputResolution(width, height));

                self.video_controls.emit(VideoControlMsg::VideoLoaded);
                self.preview_frame.emit(PreviewFrameMsg::VideoLoaded);

                self.preview_frame.widget().set_visible(true);
            }
            AppCommandMsg::FrameRendered(frame) => {
                if self.video_is_exporting {
                    self.frames_exported += 1;
                    if let Some(sender) = self.export_sender.as_ref() {
                        sender.send(frame).unwrap();
                    };

                    if self.frames_exported == self.export_target_frame_count {
                        self.export_sender = None;
                    }
                } else {
                    let texture = frame.build_gdk_texture();
                    self.preview_frame
                        .emit(PreviewFrameMsg::FrameRendered(texture));
                }
            }
            AppCommandMsg::InitWithvideo => {
                sender.input(AppMsg::SetVideo(self.uri.as_ref().unwrap().clone()));
            }
        }

        self.update_view(widgets, sender);
    }
}

fn preview_size(info: &VideoInfo) -> (u32, u32) {
    let scale_factor = (info.width as f32 / 1280.0).max(1.0);
    let scaled_width = (info.width as f32 / scale_factor) as u32;
    let scaled_height = (info.height as f32 / scale_factor) as u32;

    (scaled_width, scaled_height)
}
