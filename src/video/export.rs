use crate::app::{App, AppMsg};
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::Player;
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, ObjectExt, PadExt,
};
use gst::{ClockTime, State};
use gst_app::AppSrc;
use gst_pbutils::prelude::EncodingProfileBuilder;
use gst_pbutils::EncodingContainerProfile;
use relm4::gtk::gdk;
use relm4::gtk::prelude::{TextureExt, TextureExtManual};
use relm4::ComponentSender;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

#[derive(Debug)]
pub struct TimelineExportSettings {
    pub start: ClockTime,
    pub end: ClockTime,
}

impl Player {
    pub fn export_video(
        &mut self,
        save_uri: String,
        timeline_settings: TimelineExportSettings,
        controls_export_settings: ControlsExportSettings,
        app_sender: ComponentSender<App>,
        texture_receiver: mpsc::Receiver<gdk::Texture>,
    ) {
        self.set_is_playing(false);

        self.app_sink.set_property("sync", false);
        self.set_is_mute(true);

        println!("seeking to: {timeline_settings:?}");
        self.seek_segment(timeline_settings.start, timeline_settings.end);

        export_video(
            save_uri,
            self.info.clone(),
            controls_export_settings,
            app_sender,
            texture_receiver,
        );

        self.set_is_playing(true);
    }
}

fn build_container_profile(
    info: &VideoInfo,
    container: OutputContainerSettings,
) -> EncodingContainerProfile {
    let container_caps = container.container.caps_builder().build();
    let video_caps = container.video_codec.caps_builder().build();

    let video_profile = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
        .name("video_profile")
        .build();
    let container_builder = EncodingContainerProfile::builder(&container_caps)
        .name("Container")
        .add_profile(video_profile);

    if container.no_audio {
        container_builder.build()
    } else {
        let audio_stream = &info.container_info.audio_streams[container.audio_stream_idx as usize];

        let audio_caps = audio_stream.codec.caps_builder().build();
        let audio_profile = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
            .name("audio_profile")
            .build();

        container_builder.add_profile(audio_profile).build()
    }
}

fn export_video(
    save_uri: String,
    info: VideoInfo,
    encoding_settings: ControlsExportSettings,
    app_sender: ComponentSender<App>,
    texture_receiver: mpsc::Receiver<gdk::Texture>,
) {
    let now = SystemTime::now();
    let gst_video_info =
        gst_video::VideoInfo::builder(gst_video::VideoFormat::Rgba, info.width, info.height)
            .fps(info.framerate.clone())
            .build()
            .expect("Couldn't build video info");
    let container_profile = build_container_profile(&info, encoding_settings.container);

    let pipeline = gst::Pipeline::default();

    let app_src = AppSrc::builder()
        .caps(&gst_video_info.to_caps().unwrap())
        .format(gst::Format::Time)
        .build();

    let encode_bin = gst::ElementFactory::make("encodebin")
        .property("profile", &container_profile)
        .build()
        .unwrap();
    let file_sink = gst::ElementFactory::make("filesink")
        .property("location", save_uri.as_str())
        .build()
        .unwrap();

    pipeline
        .add_many([app_src.upcast_ref(), &encode_bin, &file_sink])
        .expect("Could not add elements to pipeline");

    gst::Element::link_many([&encode_bin, &file_sink]).unwrap();

    let encode_video_sink = encode_bin.request_pad_simple("video_%u").unwrap();
    let src_pad = &app_src.static_pad("src").expect("no src pad for appsrc");
    src_pad.link(&encode_video_sink).unwrap();

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);

    app_src.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                let Ok(texture) = texture_receiver.recv() else {
                    let _ = appsrc.end_of_stream();
                    return;
                };

                let timer = SystemTime::now();
                let mut frame = vec![0u8; (info.width * info.height * 4) as usize];
                texture.download(&mut frame, gst_video_info.stride()[0] as usize);

                let mut buffer = gst::Buffer::with_size(gst_video_info.size()).unwrap();
                {
                    let buffer = buffer.get_mut().unwrap();
                    buffer.set_pts(ClockTime::from_seconds_f64(
                        frame_count as f64 * frame_spacing,
                    ));

                    let mut vframe =
                        gst_video::VideoFrameRef::from_buffer_ref_writable(buffer, &gst_video_info)
                            .unwrap();

                    vframe
                        .plane_data_mut(0)
                        .unwrap()
                        .copy_from_slice(&frame[..]);
                }
                // println!("did frame #{frame_count} in {:?}", timer.elapsed().unwrap());
                frame_count += 1;
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    pipeline.set_state(State::Playing).unwrap();

    thread::spawn(move || {
        let bus = pipeline.bus().unwrap();

        for msg in bus.iter_timed(ClockTime::NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => {
                    println!("Done? in {:?}", now.elapsed());
                    break;
                }
                MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                _ => (),
            }
        }
        app_sender.input(AppMsg::ExportDone);
        pipeline.set_state(State::Null).unwrap();
    });
}

#[cfg(test)]
mod tests {
    // todo: figure out how to get around needing relm4 sender for app for player
    #[test]
    fn export_basic_video() {
        // let (handler, texture_receiver) = RendererHandler::new(RenderMode::MostRecentFrame);
        // let player = Rc::new(RefCell::new(Player::new(
        //     sender.clone(),
        //     handler.render_cmd_sender(),
        //     handler.timer_cmd_sender(),
        // )));
    }
}
