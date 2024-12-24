use crate::ui::preview::{CropMode, Orientation, Preview};
use crate::ui::sidebar::CropExportSettings;
use gtk4::gdk;
use gtk4::prelude::WidgetExt;
use relm4::*;
use std::cell::Cell;
use std::fmt::Debug;

pub struct PreviewFrameModel {
    preview: Preview,
    video_is_loaded: bool,
    is_playing: bool,
    prev_preview_size: (i32, i32),
    preview_size_changed: Cell<bool>,
}

#[derive(Debug)]
pub enum PreviewFrameMsg {
    VideoLoaded(u32, u32),
    FrameRendered(gdk::Texture),
    Orient(Orientation),
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
    type Init = ();

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
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let preview = Preview::new();

        let offload = gtk4::GraphicsOffload::new(Some(&preview));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);
        offload.set_vexpand(true);

        let model = PreviewFrameModel {
            preview,
            video_is_loaded: false,
            is_playing: false,
            prev_preview_size: (0, 0),
            preview_size_changed: Cell::new(false),
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
            PreviewFrameMsg::VideoLoaded(width, height) => {
                self.is_playing = true;
                self.video_is_loaded = true;
                root.last_child().unwrap().set_visible(true);
                self.preview.update_native_resolution(width, height);
            }
            PreviewFrameMsg::FrameRendered(texture) => {
                self.preview.update_texture(texture);
                let preview_size = self.preview.preview_frame_size();

                if preview_size != self.prev_preview_size {
                    self.prev_preview_size = preview_size;
                    self.preview_size_changed.set(true);
                }
            }
            PreviewFrameMsg::Orient(orientation) => self.preview.set_orientation(orientation),
            PreviewFrameMsg::StraightenStart => self.preview.straigtening_begun(),
            PreviewFrameMsg::Straighten(angle) => {
                self.preview.set_straigten_angle(angle);
                // todo: get new frame width considering the zoom that has happened. So can get max
                //  max resolution version possible.
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

    pub fn preview_size(&self) -> (i32, i32) {
        self.preview.preview_frame_size()
    }

    pub fn check_and_lower_preview_size_changed(&self) -> bool {
        let changed = self.preview_size_changed.get();
        self.preview_size_changed.set(false);
        changed
    }

    pub fn export_settings(&self) -> CropExportSettings {
        self.preview.export_settings()
    }
}
