use gtk4::prelude::{ButtonExt, WidgetExt};
use relm4::adw::prelude::{ComboRowExt, ExpanderRowExt, PreferencesRowExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

pub struct OutputControlsModel {}

#[derive(Debug)]
pub enum OutputControlsMsg {}

#[derive(Debug)]
pub enum OutputControlsOutput {
    ExportFrame,
}

#[relm4::component(pub)]
impl SimpleComponent for OutputControlsModel {
    type Input = OutputControlsMsg;
    type Output = OutputControlsOutput;
    type Init = ();

    view! {
        adw::PreferencesPage{
            set_hexpand: true,

            adw::PreferencesGroup{
                adw::ExpanderRow {
                    set_title: "Codec",

                    add_row = &adw::ComboRow{
                        set_title: "Video Codec",
                        #[wrap(Some)]
                        set_model = &gtk::StringList::new(&["AV1", "MPEG", "VP8", "VP9", "X264", "X265"]),
                    },

                    add_row= &adw::ComboRow{
                        set_title: "Audio Codec",
                        #[wrap(Some)]
                        set_model = &gtk::StringList::new(&["AAC", "OPUS", "RAW",]),
                    },

                    add_row = &adw::ComboRow{
                        set_title: "Output Container",
                        #[wrap(Some)]
                        set_model = &gtk::StringList::new(&["MP4", "MKV", "MOV", "WEBM"]),
                    },
                },
            },

            adw::PreferencesGroup{
                 gtk::Button {
                    set_label: "Export Frame",
                    connect_clicked[sender] => move |_| {
                        sender.output(OutputControlsOutput::ExportFrame).unwrap()
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = OutputControlsModel {};

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {}
    }
}

impl OutputControlsModel {}
