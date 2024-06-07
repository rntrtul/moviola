use relm4::{adw, gtk, main_application, ComponentParts, ComponentSender, SimpleComponent, Controller, Component, ComponentController};

use gtk::prelude::{
    ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk::{glib};
use super::ui::video_player::VideoPlayerModel;
use super::ui::edit_controls::EditControlsModel;

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
    edit_controls: Controller<EditControlsModel>,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    Quit,
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = u8;
    type Input = AppMsg;
    type Output = ();
    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,
            set_default_height: 480,
            set_default_width: 640,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[name="tool_bar_view"]
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[name = "content"]
                gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,

                    model.video_player.widget(),

                    #[name="stack"]
                    adw::ViewStack {
                        add_titled_with_icon: (model.edit_controls.widget(), Some("Edit"), "Edit", "cut"),
                        add_titled: (&adw::StatusPage::builder().title("FFF").description("HERE").build(), Some("Convert"), "Convert"),
                    },
                },

                #[name="switch_bar"]
                add_bottom_bar = &adw::ViewSwitcherBar{},
            }

        }
    }

    type Widgets = AppWidgets;

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let video_player: Controller<VideoPlayerModel> =
            VideoPlayerModel::builder()
                .launch(2)
                .detach();

        let edit_controls: Controller<EditControlsModel> =
            EditControlsModel::builder()
                .launch(())
                .detach();

        let model = Self {
            video_player,
            edit_controls,
        };

        let widgets = view_output!();

        widgets.switch_bar.set_reveal(true);
        widgets.switch_bar.set_stack(Some(&widgets.stack));
        widgets.tool_bar_view.set_content(Some(&widgets.content));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
        }
    }
}
