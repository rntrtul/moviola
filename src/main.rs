use argh::FromArgs;
use relm4::gtk::gio;
use relm4::{adw, gtk, RelmApp, RELM_THREADS};

use crate::app::App;

mod app;
mod config;
mod geometry;
mod range;
mod renderer;
mod ui;
mod video;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(FromArgs)]
/// option
struct Cli {
    #[argh(option, short = 'f')]
    /// path of media to load immediately
    file_path: Option<String>,
}

fn initilaize_gresources() {
    gio::resources_register_include!("resources.gresource").unwrap();

    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    theme.add_resource_path("/org/fareedsh/Moviola/icons");
}

fn main() {
    let cli: Cli = argh::from_env();

    let _tracing_sub = tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::FULL))
        .with(EnvFilter::from_default_env())
        .init();

    gst::init().unwrap();
    gtk::init().unwrap();

    initilaize_gresources();

    RELM_THREADS.set(4).unwrap();
    let style_manger = adw::StyleManager::default();
    style_manger.set_color_scheme(adw::ColorScheme::ForceDark);

    let uri = if let Some(file_path) = cli.file_path {
        let path = std::path::PathBuf::from(file_path);
        Some(format!("file://{}", path.canonicalize().unwrap().to_str().unwrap()).to_owned())
    } else {
        None
    };

    let app = RelmApp::new("io.rntrtul.github.Moviola").with_args(vec![]);
    app.run::<App>(uri);
}
