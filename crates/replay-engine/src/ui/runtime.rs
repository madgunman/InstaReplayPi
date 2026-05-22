//! Single winit event loop for operator egui window + HDMI program window.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use replay_core::config::OperatorConfig;
use replay_core::types::DisplayInfo;
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

use crate::controller::StatusSnapshot;
use crate::ui::operator_shell::{OperatorCmd, OperatorShell};
use crate::ui::program_window::ProgramWindowState;

pub enum ProgramRequest {
    ListDisplays(Sender<Vec<DisplayInfo>>),
    OpenWindow {
        display_id: u32,
        fullscreen: bool,
        reply: Sender<Result<usize>>,
    },
    CloseWindow,
    Shutdown,
}

#[derive(Clone)]
pub struct ProgramOutputHandle {
    tx: Sender<ProgramRequest>,
    _thread: Arc<JoinHandle<()>>,
}

pub struct UiSpawnConfig {
    pub operator: Option<OperatorConfig>,
    pub status: Arc<RwLock<StatusSnapshot>>,
    pub cmd_tx: Option<mpsc::Sender<OperatorCmd>>,
}

impl ProgramOutputHandle {
    pub fn headless() -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let handle = thread::Builder::new()
            .name("ui-headless".into())
            .spawn(move || run_headless_loop(rx))
            .expect("spawn ui headless");
        Self {
            tx: thread_tx,
            _thread: Arc::new(handle),
        }
    }

    /// UI thread with program window only (no operator egui).
    pub fn spawn_program_only() -> Self {
        Self::spawn_ui(UiSpawnConfig {
            operator: None,
            status: Arc::new(RwLock::new(StatusSnapshot::default_offline())),
            cmd_tx: None,
        })
    }

    pub fn spawn_ui(config: UiSpawnConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let handle = thread::Builder::new()
            .name("ui-runtime".into())
            .spawn(move || run_ui_loop(rx, config))
            .expect("spawn ui runtime");
        Self {
            tx: thread_tx,
            _thread: Arc::new(handle),
        }
    }

    pub fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(ProgramRequest::ListDisplays(reply_tx))
            .context("ui runtime thread stopped")?;
        reply_rx.recv().context("list_displays reply")
    }

    pub fn open_window(&self, display_id: u32, fullscreen: bool) -> Result<usize> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(ProgramRequest::OpenWindow {
                display_id,
                fullscreen,
                reply: reply_tx,
            })
            .context("ui runtime thread stopped")?;
        reply_rx.recv().context("open_window reply")?
    }

    pub fn close_window(&self) {
        let _ = self.tx.send(ProgramRequest::CloseWindow);
    }
}

struct UiApp {
    rx: Receiver<ProgramRequest>,
    program: ProgramWindowState,
    operator: Option<OperatorShell>,
}

impl UiApp {
    fn new(rx: Receiver<ProgramRequest>, config: UiSpawnConfig) -> Self {
        let operator = config.operator.zip(config.cmd_tx).map(|(op, cmd_tx)| {
            OperatorShell::new(op, config.status, cmd_tx)
        });
        Self {
            rx,
            program: ProgramWindowState::new(),
            operator,
        }
    }

    fn drain_requests(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(req) = self.rx.try_recv() {
            match req {
                ProgramRequest::ListDisplays(reply) => {
                    if self.program.resumed {
                        let _ = reply.send(self.program.enumerate_displays());
                    } else {
                        self.program.pending_list = Some(reply);
                    }
                }
                ProgramRequest::OpenWindow {
                    display_id,
                    fullscreen,
                    reply,
                } => {
                    if self.program.resumed {
                        let result =
                            self.program
                                .create_program_window(event_loop, display_id, fullscreen);
                        let _ = reply.send(result);
                    } else {
                        self.program.pending_open = Some((display_id, fullscreen, reply));
                    }
                }
                ProgramRequest::CloseWindow => self.program.close(),
                ProgramRequest::Shutdown => event_loop.exit(),
            }
        }
    }

    fn on_resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.program.resumed = true;
        self.program.ensure_probe_window(event_loop);
        self.program.flush_pending(event_loop);

        if let Some(op) = self.operator.as_mut() {
            op.resumed = true;
            op.create_window(event_loop);
        }
    }
}

impl ApplicationHandler for UiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.on_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let is_operator = self
            .operator
            .as_ref()
            .and_then(|o| o.window_id)
            .map(|id| id == window_id)
            .unwrap_or(false);

        if is_operator {
            if let Some(op) = self.operator.as_mut() {
                op.on_window_event(&event);
            }
            if let WindowEvent::RedrawRequested = event {
                if let Some(op) = self.operator.as_mut() {
                    op.on_redraw();
                }
            }
            if let WindowEvent::CloseRequested = event {
                event_loop.exit();
            }
            return;
        }

        if let WindowEvent::CloseRequested = event {
            // Program HDMI window stays until engine stops.
        }
        if let WindowEvent::RedrawRequested = event {
            // Program output is driven by GStreamer; no egui redraw.
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_requests(event_loop);
        if let Some(op) = self.operator.as_mut() {
            if op.resumed {
                if let Some(gl) = op.gl.as_ref() {
                    gl.window.request_redraw();
                }
            }
        }
    }
}

fn run_headless_loop(rx: Receiver<ProgramRequest>) {
    while let Ok(req) = rx.recv() {
        match req {
            ProgramRequest::ListDisplays(reply) => {
                let _ = reply.send(vec![DisplayInfo {
                    id: 0,
                    name: "Headless (test)".into(),
                    primary: true,
                    width: 1920,
                    height: 1080,
                }]);
            }
            ProgramRequest::OpenWindow { reply, .. } => {
                let _ = reply.send(Ok(1));
            }
            ProgramRequest::CloseWindow => {}
            ProgramRequest::Shutdown => break,
        }
    }
}

fn build_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::EventLoopBuilderExtWayland;
        use winit::platform::x11::EventLoopBuilderExtX11;
        EventLoopBuilderExtWayland::with_any_thread(&mut builder, true);
        EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
    }
    builder.build()
}

fn run_ui_loop(rx: Receiver<ProgramRequest>, config: UiSpawnConfig) {
    let event_loop = match build_event_loop() {
        Ok(el) => el,
        Err(e) => {
            warn!(error = %e, "Failed to build winit event loop");
            return;
        }
    };

    if config.operator.is_some() {
        info!("Starting native operator UI");
    }

    let mut app = UiApp::new(rx, config);
    if let Err(e) = event_loop.run_app(&mut app) {
        warn!(error = %e, "UI event loop exited");
    }
}

/// Headless program output when there is no display session (e.g. systemd before graphical login).
pub fn should_use_headless(test_mode: bool) -> bool {
    if test_mode {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        match std::env::var("DISPLAY") {
            Ok(d) if !d.trim().is_empty() => false,
            _ => true,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
