use std::fmt::Debug;

use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::*;

use crate::ui::preview::Preview;

pub struct VideoPlayerModel {
    video_is_loaded: bool,
    is_playing: bool,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    VideoLoaded,
}

#[relm4::component(pub)]
impl Component for VideoPlayerModel {
    type CommandOutput = ();
    type Input = VideoPlayerMsg;
    type Output = ();
    type Init = Preview;

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
        preview: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let offload = gtk4::GraphicsOffload::new(Some(&preview));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);

        let model = VideoPlayerModel {
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
            VideoPlayerMsg::VideoLoaded => {
                self.is_playing = true;
                self.video_is_loaded = true;
                root.last_child().unwrap().set_visible(true);
            }
        }

        self.update_view(widgets, sender);
    }
}
