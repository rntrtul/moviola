use gtk4::gio;
use relm4::{adw, gtk, RelmApp, RELM_THREADS};

use crate::app::App;

use self::config::RESOURCES_FILE;

mod app;
mod config;
mod renderer;
mod ui;
mod video;

use clap::Parser;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use url::Url;

#[derive(Parser)]
#[clap(about, version, author, long_about = None)]
struct Cli {
    #[arg(short, long)]
    file_path: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let _tracing_sub = tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::FULL))
        .with(EnvFilter::from_default_env())
        .init();

    gst::init().unwrap();
    gtk::init().unwrap();

    let res = gio::Resource::load(RESOURCES_FILE).unwrap();
    gio::resources_register(&res);

    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    theme.add_resource_path("/org/fareedsh/Moviola/icons");

    RELM_THREADS.set(4).unwrap();
    let style_manger = adw::StyleManager::default();
    style_manger.set_color_scheme(adw::ColorScheme::ForceDark);

    let uri = if let Some(path) = cli.file_path {
        Some(
            Url::from_file_path(path.canonicalize().unwrap())
                .unwrap()
                .to_string(),
        )
    } else {
        None
    };

    let app = RelmApp::new("org.fareedsh.Moviola").with_args(vec![]);
    app.run::<App>(uri);
}
