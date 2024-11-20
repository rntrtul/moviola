use crate::renderer::renderer::Renderer;
use crate::renderer::{EffectParameters, FRAME_TIME_IDX};
use crate::ui::preview::{CropMode, Orientation, Preview};
use crate::ui::sidebar::CropExportSettings;
use gtk4::gdk;
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::*;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PreviewFrameModel {
    // todo: pull renderer into app
    renderer: Arc<Mutex<Renderer>>,
    video_is_loaded: bool,
    is_playing: bool,
    preview: Preview,
}

#[derive(Debug)]
pub enum PreviewFrameMsg {
    VideoLoaded,
    FrameRendered(gdk::Texture),
    Orient(Orientation),
    CropMode(CropMode),
    CropBoxShow,
    CropBoxHide,
    Zoom(f64),
    ZoomHide,
    ZoomShow,
    EffectsChanged((EffectParameters, bool)),
}

#[derive(Debug)]
pub enum PreviewFrameOutput {
    TogglePlayPause,
}

#[derive(Debug)]
pub enum PreviewFrameCmd {
    FrameRendered(gdk::Texture),
}

#[relm4::component(pub)]
impl Component for PreviewFrameModel {
    type CommandOutput = PreviewFrameCmd;
    type Input = PreviewFrameMsg;
    type Output = PreviewFrameOutput;
    type Init = ();

    view! {
        #[name = "vid_container"]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_width_request: 426,
            set_height_request: 240,

            add_controller = gtk::GestureClick::builder().button(3).build(){
                connect_pressed[sender] => move |_,_,_,_| {
                    sender.output(PreviewFrameOutput::TogglePlayPause).unwrap()
                }
            },
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let preview = Preview::new();
        let renderer = pollster::block_on(Renderer::new());

        let offload = gtk4::GraphicsOffload::new(Some(&preview));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);

        let model = PreviewFrameModel {
            renderer: Arc::new(Mutex::new(renderer)),
            preview,
            video_is_loaded: false,
            is_playing: false,
        };

        let widgets = view_output!();

        widgets.vid_container.append(&offload);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            PreviewFrameMsg::VideoLoaded => {
                self.is_playing = true;
                self.video_is_loaded = true;
                root.last_child().unwrap().set_visible(true);
            }
            PreviewFrameMsg::FrameRendered(texture) => {
                self.preview.update_texture(texture);

                // todo: move timer out of renderer. at least frame time one
                let mut renderer = self.renderer.blocking_lock();
                renderer.timer.stop_time(FRAME_TIME_IDX);
            }
            PreviewFrameMsg::Orient(orientation) => {
                self.preview.set_orientation(orientation);
                self.renderer.blocking_lock().orient(orientation);
            }
            PreviewFrameMsg::CropMode(mode) => self.preview.set_crop_mode(mode),
            PreviewFrameMsg::CropBoxShow => self.preview.show_crop_box(),
            PreviewFrameMsg::CropBoxHide => self.preview.hide_crop_box(),
            PreviewFrameMsg::Zoom(level) => self.preview.set_zoom(level),
            PreviewFrameMsg::ZoomHide => self.preview.hide_zoom(),
            PreviewFrameMsg::ZoomShow => self.preview.show_zoom(),
            PreviewFrameMsg::EffectsChanged((params, is_playing)) => {
                self.renderer.blocking_lock().update_effects(params);

                if !is_playing {
                    self.start_render(&sender);
                }
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            PreviewFrameCmd::FrameRendered(texture) => {
                self.preview.update_texture(texture);

                let mut renderer = self.renderer.blocking_lock();
                renderer.timer.stop_time(FRAME_TIME_IDX);
            }
        }
    }
}

impl PreviewFrameModel {
    fn start_render(&self, sender: &ComponentSender<Self>) {
        let renderer = Arc::clone(&self.renderer);

        sender.oneshot_command(async move {
            let mut renderer = renderer.lock().await;
            renderer.timer.start_time(FRAME_TIME_IDX);

            let command_buffer = renderer.prepare_video_frame_render_pass();
            let texture = renderer
                .render(command_buffer)
                .await
                .expect("Could not render");

            drop(renderer);
            PreviewFrameCmd::FrameRendered(texture)
        })
    }

    pub fn reset(&self) {
        self.preview.reset_preview();
    }

    pub fn export_settings(&self) -> CropExportSettings {
        self.preview.export_settings()
    }

    pub fn renderer(&self) -> Arc<Mutex<Renderer>> {
        Arc::clone(&self.renderer)
    }
}
