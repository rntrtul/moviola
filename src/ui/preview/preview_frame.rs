use std::fmt::Debug;

use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::*;

use crate::ui::preview::{CropMode, EffectParameters, Orientation, Preview};
use crate::ui::sidebar::CropExportSettings;

pub struct PreviewFrameModel {
    video_is_loaded: bool,
    is_playing: bool,
    preview: Preview,
}

#[derive(Debug)]
pub enum PreviewFrameMsg {
    VideoLoaded,
    NewVideoFrame(gst::Sample),
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
pub enum PreviewFrameCmd {
    FrameRendered,
}

#[relm4::component(pub)]
impl Component for PreviewFrameModel {
    type CommandOutput = PreviewFrameCmd;
    type Input = PreviewFrameMsg;
    type Output = ();
    type Init = ();

    view! {
        #[name = "vid_container"]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_width_request: 426,
            set_height_request: 240,
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let preview = Preview::new();

        let offload = gtk4::GraphicsOffload::new(Some(&preview));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);

        let model = PreviewFrameModel {
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
            PreviewFrameMsg::NewVideoFrame(frame_sample) => {
                self.preview.upload_new_sample(frame_sample);
                // todo: have render frame be async in command. Need to wrap preview in Arc + Mutex
                self.preview.render_frame();
            }
            PreviewFrameMsg::Orient(orientation) => self.preview.set_orientation(orientation),
            PreviewFrameMsg::CropMode(mode) => self.preview.set_crop_mode(mode),
            PreviewFrameMsg::CropBoxShow => self.preview.show_crop_box(),
            PreviewFrameMsg::CropBoxHide => self.preview.hide_crop_box(),
            PreviewFrameMsg::Zoom(level) => self.preview.set_zoom(level),
            PreviewFrameMsg::ZoomHide => self.preview.hide_zoom(),
            PreviewFrameMsg::ZoomShow => self.preview.show_zoom(),
            PreviewFrameMsg::EffectsChanged((params, is_playing)) => {
                self.preview.update_effect_parameters(params);
                if !is_playing {
                    self.preview.render_frame();
                }
            }
        }

        self.update_view(widgets, sender);
    }
}

impl PreviewFrameModel {
    pub fn reset(&self) {
        self.preview.reset_preview();
    }

    pub fn export_settings(&self) -> CropExportSettings {
        self.preview.export_settings()
    }
}
