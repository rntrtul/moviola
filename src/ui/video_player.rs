use std::fmt::Debug;

use gst::prelude::*;
use gst::Element;
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::Align;
use relm4::adw::gdk;
use relm4::*;

use crate::ui::crop_box::MARGIN;

pub struct VideoPlayerModel {
    video_is_loaded: bool,
    is_playing: bool,
    gtk_sink: Element,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    TogglePlayPause,
    VideoLoaded,
}

#[derive(Debug)]
pub enum VideoPlayerOutput {
    ToggleVideoPlay,
}

impl VideoPlayerModel {
    pub fn sink(&self) -> &Element {
        &self.gtk_sink
    }
}

#[relm4::component(pub)]
impl Component for VideoPlayerModel {
    type CommandOutput = ();
    type Input = VideoPlayerMsg;
    type Output = VideoPlayerOutput;
    view! {
        #[name = "vid_container"]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 640,
            set_height_request: 360,

            gtk::Spinner {
                #[watch]
                set_spinning: !model.video_is_loaded,
                #[watch]
                set_visible: !model.video_is_loaded,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
            },

            add_controller = gtk::GestureClick {
                connect_pressed[sender] => move |_,_,_,_| {
                    sender.input(VideoPlayerMsg::TogglePlayPause)
                }
            },
        }
    }

    type Init = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .unwrap();

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        // todo: need gst-plugins-gtk4 13.0 to be able to use orientation property with paintable
        let picture = gtk::Picture::new();

        picture.set_paintable(Some(&paintable));
        picture.set_valign(Align::Center);
        picture.set_margin_all(MARGIN as i32);

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);

        let model = VideoPlayerModel {
            video_is_loaded: false,
            is_playing: false,
            gtk_sink,
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
            VideoPlayerMsg::VideoLoaded => {
                self.is_playing = true;
                self.video_is_loaded = true;
                root.last_child().unwrap().set_visible(true);
            }
            VideoPlayerMsg::TogglePlayPause => {
                sender.output(VideoPlayerOutput::ToggleVideoPlay).unwrap();
            }
        }

        self.update_view(widgets, sender);
    }
}

impl VideoPlayerModel {
    // todo: hookup with ui/keyboard. add support for stepping backwards
    fn _step_next_frame(&mut self) {
        let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
        self.gtk_sink.send_event(step);
    }
}
