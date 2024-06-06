use gtk4::prelude::ButtonExt;
use relm4::{adw, ComponentParts, ComponentSender, gtk, SimpleComponent};

pub struct EditControlsModel {}

#[derive(Debug)]
pub enum EditControlsMsg {}

#[relm4::component(pub)]
impl SimpleComponent for EditControlsModel {
    type Input = EditControlsMsg;
    type Output = ();
    type Init = ();

    view! {
        gtk::Box {
            gtk::Button {
                set_label: "Export Frame",
            }
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = EditControlsModel {};

        ComponentParts { model, widgets }
    }
}