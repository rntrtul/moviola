use crate::renderer::renderer::Renderer;
use crate::renderer::timer::SingleTimer;
use crate::renderer::EffectParameters;
use crate::ui::preview::Orientation;
use gtk4::gdk;
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::sync::Mutex;

pub enum RenderCmd {
    RenderFrame,
    RenderSample(gst::Sample),
    UpdateEffects(EffectParameters),
    UpdateOutputResolution(u32, u32),
    UpdateOrientation(Orientation),
}

pub struct RendererHandler {
    thread: thread::JoinHandle<()>,
    cmd_sender: mpsc::Sender<RenderCmd>,
    frame_timer: RefCell<SingleTimer>,
}

fn render_frame(
    sender: mpsc::Sender<gdk::Texture>,
    renderer: Arc<Mutex<Renderer>>,
    render_queued: Arc<AtomicBool>,
    render_cmd_sender: mpsc::Sender<RenderCmd>,
) {
    tokio::spawn(async move {
        let tex;
        {
            let mut renderer = renderer.lock().await;
            tex = renderer.render_frame().await;
        }
        sender.send(tex).unwrap();

        if render_queued.load(std::sync::atomic::Ordering::Relaxed) {
            render_cmd_sender.send(RenderCmd::RenderFrame).unwrap();
        }
    });
}

async fn update_queued(
    renderer: Arc<Mutex<Renderer>>,
    effect_parms: &mut Option<EffectParameters>,
    sample: &mut Option<gst::Sample>,
    orientation: &mut Option<Orientation>,
    output_res: &mut Option<(u32, u32)>,
) {
    let mut renderer = renderer.lock().await;

    if let Some((width, height)) = output_res {
        renderer.update_output_resolution(*width, *height);
    }

    if let Some(orientation) = orientation.take() {
        renderer.orient(orientation);
    }

    if let Some(sample) = sample.take() {
        renderer.upload_new_smple(&sample);
    }

    if let Some(params) = effect_parms.take() {
        renderer.update_effects(params);
    }
}

async fn render_loop(
    texture_sender: mpsc::Sender<gdk::Texture>,
    cmd_recv: mpsc::Receiver<RenderCmd>,
    renderer_cmd_sender: mpsc::Sender<RenderCmd>,
) {
    let renderer = Arc::new(Mutex::new(pollster::block_on(Renderer::new())));

    let mut queued_effect_params: Option<EffectParameters> = None;
    let mut queued_output_resolution: Option<(u32, u32)> = None;
    let mut queued_orientation: Option<Orientation> = None;
    let mut queued_sample: Option<gst::Sample> = None;
    let render_queued = Arc::new(AtomicBool::new(false));

    loop {
        let Ok(cmd) = cmd_recv.recv() else {
            break;
        };

        match cmd {
            RenderCmd::UpdateEffects(params) => {
                queued_effect_params.replace(params);

                if let Ok(mut renderer) = renderer.try_lock() {
                    renderer.update_effects(queued_effect_params.take().unwrap());
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::RenderSample(sample) => {
                queued_sample.replace(sample);

                if let Ok(mut guarded_renderer) = renderer.try_lock() {
                    guarded_renderer.upload_new_smple(&queued_sample.take().unwrap());
                    drop(guarded_renderer);

                    render_frame(
                        texture_sender.clone(),
                        renderer.clone(),
                        render_queued.clone(),
                        renderer_cmd_sender.clone(),
                    );
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::RenderFrame => {
                // fixme: find better way of testing if lockable other than getting lock
                //      and dropping it.
                if let Ok(guarded_renderer) = renderer.try_lock() {
                    drop(guarded_renderer);

                    if render_queued.load(std::sync::atomic::Ordering::Relaxed) {
                        render_queued.store(false, std::sync::atomic::Ordering::Relaxed);
                        update_queued(
                            renderer.clone(),
                            &mut queued_effect_params,
                            &mut queued_sample,
                            &mut queued_orientation,
                            &mut queued_output_resolution,
                        )
                        .await;
                    }

                    render_frame(
                        texture_sender.clone(),
                        renderer.clone(),
                        render_queued.clone(),
                        renderer_cmd_sender.clone(),
                    );
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::UpdateOutputResolution(width, height) => {
                queued_output_resolution.replace((width, height));

                if let Ok(mut renderer) = renderer.try_lock() {
                    let (width, height) = queued_output_resolution.take().unwrap();
                    renderer.update_output_resolution(width, height);
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::UpdateOrientation(orientation) => {
                queued_orientation.replace(orientation);

                if let Ok(mut renderer) = renderer.try_lock() {
                    renderer.orient(queued_orientation.take().unwrap());
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
}

impl RendererHandler {
    pub fn new() -> (Self, mpsc::Receiver<gdk::Texture>) {
        let (cmd_sender, cmd_recv) = mpsc::channel::<RenderCmd>();
        let (output_sender, output_receiver) = mpsc::channel::<gdk::Texture>();

        let renderer_cmd_sender = cmd_sender.clone();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(render_loop(output_sender, cmd_recv, renderer_cmd_sender));
        });

        let handler = Self {
            thread,
            cmd_sender,
            frame_timer: RefCell::new(SingleTimer::new()),
        };

        (handler, output_receiver)
    }

    pub fn cmd_sender(&self) -> mpsc::Sender<RenderCmd> {
        self.cmd_sender.clone()
    }

    pub fn send_cmd(&self, cmd: RenderCmd) {
        self.cmd_sender.send(cmd).unwrap();
    }

    pub fn start_frame_time(&self) {
        self.frame_timer.borrow_mut().start_time();
    }

    pub fn stop_frame_time(&self) {
        self.frame_timer.borrow_mut().stop_time();
    }
}
