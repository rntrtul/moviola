use crate::renderer::renderer::Renderer;
use crate::renderer::timer::SingleTimer;
use crate::renderer::EffectParameters;
use crate::ui::preview::Orientation;
use gtk4::gdk;
use std::cell::RefCell;
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

async fn render_loop(
    texture_sender: mpsc::Sender<gdk::Texture>,
    cmd_recv: mpsc::Receiver<RenderCmd>,
) {
    let renderer = Arc::new(Mutex::new(pollster::block_on(Renderer::new())));

    loop {
        let Ok(cmd) = cmd_recv.recv() else {
            break;
        };

        // todo: check if already rendering.
        //  if true update next_sample and then after render finishes return tex and do that framet.
        //  Avoids rendering every frame if we will be are slow. User get most up to date img
        //  probably most useful for when changing effects on a static image.
        match cmd {
            RenderCmd::UpdateEffects(params) => {
                renderer.lock().await.update_effects(params);
            }
            RenderCmd::RenderSample(sample) => {
                let sender = texture_sender.clone();
                let renderer = renderer.clone();

                tokio::spawn(async move {
                    let tex;
                    {
                        let mut renderer = renderer.lock().await;
                        tex = renderer.render_sample(&sample).await;
                    }
                    sender.send(tex).unwrap();
                });
            }
            RenderCmd::RenderFrame => {
                let sender = texture_sender.clone();
                let renderer = renderer.clone();
                tokio::spawn(async move {
                    let tex;
                    {
                        let mut renderer = renderer.lock().await;
                        tex = renderer.render_curr_sample().await;
                    }
                    sender.send(tex).unwrap();
                });
            }
            RenderCmd::UpdateOutputResolution(width, height) => renderer
                .lock()
                .await
                .update_output_resolution(width, height),
            RenderCmd::UpdateOrientation(orientation) => renderer.lock().await.orient(orientation),
        }
    }
}

impl RendererHandler {
    pub fn new() -> (Self, mpsc::Receiver<gdk::Texture>) {
        let (cmd_sender, cmd_recv) = mpsc::channel::<RenderCmd>();
        let (output_sender, output_receiver) = mpsc::channel::<gdk::Texture>();

        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(render_loop(output_sender, cmd_recv));
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
