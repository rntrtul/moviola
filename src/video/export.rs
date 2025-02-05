use crate::app::{App, AppMsg};
use crate::renderer::FrameSize;
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::Player;
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt, ObjectExt,
    PadExt, PadExtManual,
};
use gst::ClockTime;
use gst_app::AppSrc;
use gst_pbutils::prelude::{EncodingProfileBuilder, EncodingProfileExt};
use gst_pbutils::EncodingContainerProfile;
use gst_video::VideoFrameExt;
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

impl TimelineExportSettings {
    pub fn duration(&self) -> ClockTime {
        self.end - self.start
    }
}

impl Player {
    pub fn export_video(
        &mut self,
        source_uri: String,
        save_uri: String,
        timeline_settings: TimelineExportSettings,
        controls_export_settings: ControlsExportSettings,
        output_size: FrameSize,
        app_sender: ComponentSender<App>,
        texture_receiver: mpsc::Receiver<gdk::Texture>,
    ) {
        self.set_is_playing(false);
        self.seek_segment(timeline_settings.start, timeline_settings.end);

        self.app_sink.set_property("sync", false);
        self.set_is_mute(true);

        let pipeline = export_video(
            source_uri,
            save_uri,
            timeline_settings,
            self.info.clone(),
            output_size,
            controls_export_settings,
            texture_receiver,
        );

        thread::spawn(move || {
            let now = SystemTime::now();
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
            pipeline.set_state(gst::State::Null).unwrap();
            app_sender.input(AppMsg::ExportDone);
        });

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
    source_uri: String,
    save_uri: String,
    timeline_settings: TimelineExportSettings,
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    texture_receiver: mpsc::Receiver<gdk::Texture>,
) -> gst::Pipeline {
    let gst_video_info = gst_video::VideoInfo::builder(
        gst_video::VideoFormat::Rgba,
        output_size.width,
        output_size.height,
    )
    .fps(info.framerate.clone())
    .build()
    .expect("Couldn't build video info");
    let container_profile = build_container_profile(&info, encoding_settings.container);

    let pipeline = gst::Pipeline::with_name("export_pipeline");

    let video_app_src = AppSrc::builder()
        .caps(&gst_video_info.to_caps().unwrap())
        .format(gst::Format::Time)
        .build();
    let audio_decodebin = gst::ElementFactory::make("uridecodebin3")
        .property("instant-uri", true)
        .property("uri", &source_uri)
        .build()
        .unwrap();
    let encode_bin = gst::ElementFactory::make("encodebin")
        .property("profile", &container_profile)
        .build()
        .unwrap();
    let file_sink = gst::ElementFactory::make("filesink")
        .property("location", save_uri.as_str())
        .property("sync", false)
        .build()
        .unwrap();

    let export_elements = [
        &audio_decodebin,
        video_app_src.upcast_ref(),
        &encode_bin,
        &file_sink,
    ];

    pipeline
        .add_many(&export_elements)
        .expect("Could not add elements to pipeline");
    gst::Element::link_many([&encode_bin, &file_sink]).unwrap();

    let encode_video_sink = encode_bin.request_pad_simple("video_%u").unwrap();
    let video_src = &video_app_src
        .static_pad("src")
        .expect("no src pad for video appsrc");
    video_src.link(&encode_video_sink).unwrap();

    let encode_audio_sink = encode_bin.request_pad_simple("audio_%u").unwrap();
    audio_decodebin.connect_pad_added(move |_, pad| {
        if pad.name().starts_with("audio") {
            // fixme: attach correct audio stream
            pad.link(&encode_audio_sink).unwrap();
        }
    });

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);

    video_app_src.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                let Ok(texture) = texture_receiver.recv() else {
                    let _ = appsrc.end_of_stream();
                    return;
                };

                let timer = SystemTime::now();
                // todo: resue buffer by allocating it once
                let mut frame = vec![0u8; (output_size.width * output_size.height * 4) as usize];
                texture.download(&mut frame, gst_video_info.stride()[0] as usize);

                let mut buffer = gst::Buffer::from_slice(frame);
                {
                    let buffer = buffer.get_mut().unwrap();
                    buffer.set_pts(ClockTime::from_seconds_f64(
                        frame_count as f64 * frame_spacing,
                    ));
                }

                frame_count += 1;
                let _ = appsrc.push_buffer(buffer);
                // println!("did frame #{frame_count} in {:?}", timer.elapsed().unwrap());
            })
            .build(),
    );

    pipeline.set_state(gst::State::Playing).unwrap();
    pipeline
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
