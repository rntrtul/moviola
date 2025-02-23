use crate::app::{App, AppMsg};
use crate::renderer::renderer::RenderedFrame;
use crate::renderer::{FrameSize, RenderCmd, TimerCmd};
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::{video_appsink, AppSinkUsage, Player};
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt, ObjectExt,
    PadExt, PadExtManual,
};
use gst::ClockTime;
use gst_app::{AppSink, AppSrc};
use gst_pbutils::prelude::{EncodingProfileBuilder, EncodingProfileExt};
use gst_pbutils::EncodingContainerProfile;
use gst_video::VideoFrameExt;
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
        frame_receiver: mpsc::Receiver<RenderedFrame>,
        app_sender: ComponentSender<App>,
        sample_sender: mpsc::Sender<RenderCmd>,
        timer_sender: mpsc::Sender<TimerCmd>,
    ) {
        self.reset_pipeline();

        let video_app_sink = video_appsink(
            app_sender.clone(),
            sample_sender,
            timer_sender,
            AppSinkUsage::Export,
        );

        let pipeline = export_video(
            source_uri,
            save_uri,
            self.info.clone(),
            output_size,
            controls_export_settings,
            timeline_settings,
            video_app_sink,
            frame_receiver,
        );

        thread::spawn(move || {
            let bus = pipeline.bus().unwrap();
            wait_for_eos(bus);

            pipeline.set_state(gst::State::Null).unwrap();
            app_sender.input(AppMsg::ExportDone);
        });
    }
}

fn wait_for_eos(bus: gst::Bus) {
    let now = SystemTime::now();

    for msg in bus.iter_timed(ClockTime::NONE) {
        use gst::MessageView;
        println!("msg: {msg:?}");

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
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    timeline_settings: TimelineExportSettings,
    video_appsink: AppSink,
    frame_receiver: mpsc::Receiver<RenderedFrame>,
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

    let decodebin = gst::ElementFactory::make("uridecodebin3")
        .property("instant-uri", true)
        .property("uri", source_uri.as_str())
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

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);
    let alloc = gst_allocator::DmaBufAllocator::new();
    let video_app_src = AppSrc::builder()
        .caps(&gst_video_info.to_caps().unwrap())
        .format(gst::Format::Time)
        .callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    let Ok(frame) = frame_receiver.recv() else {
                        let _ = appsrc.end_of_stream();
                        return;
                    };

                    let timer = SystemTime::now();

                    let mut buffer = gst::Buffer::new();
                    let mem = unsafe {
                        alloc
                            .alloc_with_flags(
                                frame.fd,
                                (output_size.width * output_size.height * 4) as usize,
                                gst_allocator::FdMemoryFlags::DONT_CLOSE,
                            )
                            .expect("Failed to allocate buffer")
                    };

                    {
                        let buffer = buffer.get_mut().unwrap();
                        buffer.append_memory(mem);
                        buffer.set_pts(ClockTime::from_seconds_f64(
                            frame_count as f64 * frame_spacing,
                        ));
                    }

                    frame_count += 1;
                    let _ = appsrc.push_buffer(buffer);
                    println!("did vframe #{frame_count} in {:?}", timer.elapsed());
                })
                .build(),
        )
        .build();

    pipeline
        .add_many(&[
            &decodebin,
            &encode_bin,
            &file_sink,
            video_app_src.upcast_ref(),
        ])
        .unwrap();
    gst::Element::link_many([&encode_bin, &file_sink]).unwrap();

    let encode_video_sink = encode_bin.request_pad_simple("video_%u").unwrap();
    let video_src = &video_app_src
        .static_pad("src")
        .expect("no src pad for video appsrc");
    video_src.link(&encode_video_sink).unwrap();

    let pipeline_weak = pipeline.downgrade();

    let exporting_audio = !encoding_settings.container.no_audio;
    decodebin.connect_pad_added(move |decode, src_pad| {
        let Some(pipeline) = pipeline_weak.upgrade() else {
            return;
        };

        let (is_audio, is_video) = (
            src_pad.name().starts_with("audio"),
            src_pad.name().starts_with("video"),
        );

        if exporting_audio && is_audio {
            let encode_audio_sink = encode_bin.request_pad_simple("audio_%u").unwrap();
            src_pad.link(&encode_audio_sink).unwrap();
        } else if is_video {
            let queue = gst::ElementFactory::make("queue").build().unwrap();
            let video_convert = gst::ElementFactory::make("videoconvert").build().unwrap();

            let elements = &[&queue, &video_convert, video_appsink.upcast_ref()];
            pipeline.add_many(elements).unwrap();
            gst::Element::link_many(elements).unwrap();

            for e in elements {
                e.sync_state_with_parent().unwrap();
            }

            let video_sink = queue.static_pad("sink").unwrap();
            src_pad.link(&video_sink).unwrap();
        }
    });

    pipeline.set_state(gst::State::Paused).unwrap();

    // pipeline
    //     .seek(
    //         1.0,
    //         gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
    //         gst::SeekType::Set,
    //         timeline_settings.start,
    //         gst::SeekType::Set,
    //         timeline_settings.end,
    //     )
    //     .unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();
    pipeline
}

#[cfg(test)]
mod tests {
    use crate::config::{VIDEO_EXPORT_DST, VIDEO_TEST_FILE_1080};
    use crate::renderer::renderer::RenderedFrame;
    use crate::renderer::{FrameSize, RenderMode, RenderResopnse, RendererHandler};
    use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
    use crate::video::export::{export_video, wait_for_eos, TimelineExportSettings};
    use gst::prelude::ElementExt;
    use gst::ClockTime;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::{mpsc, Arc};
    use tokio::task::JoinHandle;

    fn render_listner(
        render_response: mpsc::Receiver<RenderResopnse>,
        frame_sender: mpsc::Sender<RenderedFrame>,
        no_more_frames_incoming: Arc<AtomicBool>,
        frames_to_render: Arc<AtomicU64>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut frames_done = 0;
            loop {
                let Ok(response) = render_response.recv() else {
                    break;
                };

                frames_done += 1;
                match response {
                    RenderResopnse::FrameRendered(frame) => {
                        frame_sender.send(frame).unwrap();
                    }
                }

                if no_more_frames_incoming.load(std::sync::atomic::Ordering::Relaxed)
                    && frames_to_render.load(std::sync::atomic::Ordering::Relaxed) == frames_done
                {
                    break;
                }
            }
        })
    }

    // fixme: why do appsrc allocs fail due to fd.
    #[tokio::test(flavor = "multi_thread")]
    async fn export_basic_video() {
        gst::init().unwrap();

        let (handler, render_response) = RendererHandler::new(RenderMode::AllFrames);
        let (frame_sender, frame_recv) = mpsc::channel();
        let frames_to_render = Arc::new(AtomicU64::new(0));
        let export_done = Arc::new(AtomicBool::new(false));

        let listener = render_listner(
            render_response,
            frame_sender,
            export_done.clone(),
            frames_to_render.clone(),
        );

        let render_sender = handler.render_cmd_sender();
        let app_sink = gst_app::AppSink::builder()
            .enable_last_sample(true)
            .max_buffers(1)
            .caps(
                &gst_video::VideoCapsBuilder::new()
                    .format(gst_video::VideoFormat::Rgba)
                    .build(),
            )
            .callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |appsink| {
                        let sample = appsink.pull_sample().unwrap();
                        render_sender
                            .send(crate::renderer::RenderCmd::RenderSample(sample))
                            .unwrap();
                        let a = frames_to_render.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // println!("sending sample {}", a + 1);
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .eos(move |_| {
                        println!("EOS");
                        export_done.store(true, std::sync::atomic::Ordering::Relaxed);
                    })
                    .build(),
            )
            .build();

        // todo: get info populated rather than manual
        let pipeline = export_video(
            VIDEO_TEST_FILE_1080.to_string(),
            VIDEO_EXPORT_DST.to_string(),
            crate::video::metadata::VideoInfo {
                duration: Default::default(),
                framerate: gst::Fraction::new(500, 21),
                width: 1920,
                height: 1080,
                aspect_ratio: 1.777,
                container_info: Default::default(),
                orientation: Default::default(),
            },
            FrameSize::new(1920, 1080),
            ControlsExportSettings {
                container: OutputContainerSettings {
                    no_audio: true,
                    audio_stream_idx: 0,
                    audio_codec: crate::video::metadata::AudioCodec::AAC,
                    audio_bitrate: 0,
                    container: crate::video::metadata::ContainerFormat::MP4,
                    video_codec: crate::video::metadata::VideoCodec::X265,
                    video_bitrate: 0,
                },
                container_is_default: true,
                effect_parameters: Default::default(),
            },
            TimelineExportSettings {
                start: ClockTime::ZERO,
                end: ClockTime::from_seconds_f64(10f64),
            },
            app_sink,
            frame_recv,
        );

        listener.await.expect("could not await on listener");
        wait_for_eos(pipeline.bus().unwrap());
        pipeline.set_state(gst::State::Null).unwrap();
    }
}
