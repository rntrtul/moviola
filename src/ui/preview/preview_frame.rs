use crate::renderer::{TimerCmd, TimerEvent};
use crate::ui::preview::{CropMode, Preview};
use crate::ui::sidebar::CropExportSettings;
use relm4::gtk::gdk;
use relm4::gtk::prelude::WidgetExt;
use relm4::*;
use std::fmt::Debug;
use std::sync::mpsc;
use std::time::Instant;

pub struct PreviewFrameModel {
    preview: Preview,
    timer_sender: mpsc::Sender<TimerCmd>,
    video_is_loaded: bool,
    is_playing: bool,
}

#[derive(Debug)]
pub enum PreviewFrameMsg {
    VideoLoaded,
    FrameRendered(gdk::Texture),
    StraightenStart,
    Straighten(f64),
    StraightenEnd,
    CropMode(CropMode),
    CropBoxShow,
    CropBoxHide,
    Zoom(f64),
    ZoomHide,
    ZoomShow,
}

#[derive(Debug)]
pub enum PreviewFrameOutput {
    TogglePlayPause,
}

#[derive(Debug)]
pub enum PreviewFrameCmd {}

#[relm4::component(pub)]
impl Component for PreviewFrameModel {
    type CommandOutput = PreviewFrameCmd;
    type Input = PreviewFrameMsg;
    type Output = PreviewFrameOutput;
    type Init = mpsc::Sender<TimerCmd>;

    view! {
        #[name = "vid_container"]
        adw::Clamp {
            set_hexpand: true,
            set_vexpand: true,
            set_maximum_size: 1080,
            set_unit: adw::LengthUnit::Px,

            add_controller = gtk::GestureClick::builder().button(3).build(){
                connect_pressed[sender] => move |_,_,_,_| {
                    sender.output(PreviewFrameOutput::TogglePlayPause).unwrap()
                }
            },
        }
    }

    fn init(
        timer_sender: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let preview = Preview::new();

        let offload = relm4::gtk::GraphicsOffload::new(Some(&preview));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);
        offload.set_vexpand(true);

        let model = PreviewFrameModel {
            timer_sender,
            preview,
            video_is_loaded: false,
            is_playing: false,
        };

        let widgets = view_output!();

        widgets.vid_container.set_child(Some(&offload));

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
                self.video_is_loaded = true;
                self.is_playing = true;
                root.last_child().unwrap().set_visible(true);
            }
            PreviewFrameMsg::FrameRendered(texture) => {
                self.preview.update_texture(texture);
                let now = Instant::now();
                self.timer_sender
                    .send(TimerCmd::Stop(TimerEvent::FrameTime, now))
                    .unwrap();
                self.timer_sender
                    .send(TimerCmd::Stop(TimerEvent::Transmission, now))
                    .unwrap();
            }
            PreviewFrameMsg::StraightenStart => self.preview.straigtening_begun(),
            PreviewFrameMsg::Straighten(angle) => {
                self.preview.set_straigten_angle(angle);
            }
            PreviewFrameMsg::StraightenEnd => self.preview.straigtening_finished(),
            PreviewFrameMsg::CropMode(mode) => self.preview.set_crop_mode(mode),
            PreviewFrameMsg::CropBoxShow => self.preview.show_crop_box(),
            PreviewFrameMsg::CropBoxHide => self.preview.hide_crop_box(),
            PreviewFrameMsg::Zoom(level) => self.preview.set_zoom(level),
            PreviewFrameMsg::ZoomHide => self.preview.hide_zoom(),
            PreviewFrameMsg::ZoomShow => self.preview.show_zoom(),
        }

        self.update_view(widgets, sender);
    }
}

impl PreviewFrameModel {
    pub fn reset(&self) {
        self.preview.reset_preview();
    }

    pub fn export_settings(&self) -> CropExportSettings {
        self.preview.crop_settings()
    }
}
