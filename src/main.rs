use relm4::{adw, gtk, RelmApp, RELM_THREADS};

use crate::app::App;

mod app;
mod config;
mod ui;
mod video;

fn main() {
    gst::init().unwrap();
    gtk::init().unwrap();
    RELM_THREADS.set(2).unwrap();
    // relm4_icons::initialize_icons();
    let style_manger = adw::StyleManager::default();
    style_manger.set_color_scheme(adw::ColorScheme::ForceDark);

    let app = RelmApp::new("org.fareedsh.Moviola");
    app.run::<App>(0);
}
