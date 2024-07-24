use relm4::{RelmApp, RELM_THREADS};

use crate::app::App;

mod app;
mod ui;

fn main() {
    gst::init().unwrap();
    RELM_THREADS.set(2).unwrap();
    relm4_icons::initialize_icons();
    let app = RelmApp::new("relm4.test.videditor");
    app.run::<App>(0);
}
