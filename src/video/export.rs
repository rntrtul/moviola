use crate::app::{App, AppMsg};
use crate::renderer::renderer::U32_SIZE;
use crate::renderer::FrameSize;
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::Player;
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt, ObjectExt, PadExt,
};
use gst::{ClockTime, FlowSuccess};
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

impl Player {
    pub fn export_video(
        &mut self,
        save_uri: String,
        timeline_settings: TimelineExportSettings,
        controls_export_settings: ControlsExportSettings,
        output_size: FrameSize,
        app_sender: ComponentSender<App>,
        texture_receiver: mpsc::Receiver<gdk::Texture>,
    ) {
        self.set_is_playing(false);

        let audio_app_sink = gst_app::AppSink::builder()
            .enable_last_sample(true)
            .max_buffers(1)
            .caps(&gst::Caps::new_any())
            .build();
        self.bin.add(&audio_app_sink).unwrap();

        let export_audio_src = self.audio_selector.request_pad_simple("src_%u").unwrap();
        let export_audio_sink = audio_app_sink.static_pad("sink").unwrap();
        export_audio_src.link(&export_audio_sink).unwrap();

        self.audio_selector
            .set_property("active_pad", &export_audio_src);

        let (sender, receiver) = mpsc::channel();
        let eos_sender = sender.clone();
        audio_app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |app| {
                    let sample = app.pull_sample().unwrap();
                    sender
                        .send(Some(sample))
                        .expect("failed to send audio sample");
                    Ok(FlowSuccess::Ok)
                })
                .eos(move |_| eos_sender.send(None).unwrap())
                .build(),
        );

        self.app_sink.set_property("sync", false);
        audio_app_sink.set_property("sync", false);
        // fixme: seek hangs with audio_sink added
        // self.seek_segment(timeline_settings.start, timeline_settings.end);

        let pipeline = export_video(
            save_uri,
            self.info.clone(),
            output_size,
            controls_export_settings,
            texture_receiver,
            receiver,
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
    save_uri: String,
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    texture_receiver: mpsc::Receiver<gdk::Texture>,
    audio_receiver: mpsc::Receiver<Option<gst::Sample>>,
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

    let pipeline = gst::Pipeline::new();

    let video_app_src = AppSrc::builder()
        .caps(&gst_video_info.to_caps().unwrap())
        .format(gst::Format::Time)
        .build();
    let audio_app_src = AppSrc::builder()
        .caps(&gst::Caps::new_any())
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

    let export_elements = [
        video_app_src.upcast_ref(),
        audio_app_src.upcast_ref(),
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
    let audio_src = &audio_app_src
        .static_pad("src")
        .expect("no src pad for audio appsrc");
    audio_src.link(&encode_audio_sink).unwrap();

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);

    let pixel_size = 4 * U32_SIZE as usize;

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

                let mut buffer =
                    gst::Buffer::with_size(output_size.frame_buffer_size(pixel_size)).unwrap();
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
                frame_count += 1;
                let _ = appsrc.push_buffer(buffer);
                println!("did frame #{frame_count} in {:?}", timer.elapsed().unwrap());
            })
            .build(),
    );

    audio_app_src.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                let Ok(Some(audio_sample)) = audio_receiver.recv() else {
                    let _ = appsrc.end_of_stream();
                    return;
                };
                let _ = appsrc.push_sample(&audio_sample);
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
