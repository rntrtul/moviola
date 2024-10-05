use gtk4::gio;
use relm4::{adw, gtk, RelmApp, RELM_THREADS};

use crate::app::App;

use self::config::RESOURCES_FILE;

mod app;
mod config;
mod ui;
mod video;

fn main() {
    env_logger::init();
    gst::init().unwrap();
    gtk::init().unwrap();

    gst_plugin_gtk4::plugin_register_static().expect("failed to register plugin");

    let res = gio::Resource::load(RESOURCES_FILE).unwrap();
    gio::resources_register(&res);

    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    theme.add_resource_path("/org/fareedsh/Moviola/icons");

    RELM_THREADS.set(2).unwrap();
    let style_manger = adw::StyleManager::default();
    style_manger.set_color_scheme(adw::ColorScheme::ForceDark);

    let app = RelmApp::new("org.fareedsh.Moviola");
    app.run::<App>(0);
}
