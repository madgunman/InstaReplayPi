//! Borderless fullscreen program window on a chosen display (winit + GStreamer overlay).

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use replay_core::types::DisplayInfo;
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use crate::pipeline::program_sink::window_handle_from_winit;

enum ProgramRequest {
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
    _thread: std::sync::Arc<JoinHandle<()>>,
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

impl ProgramOutputHandle {
    /// No winit window — for `--test`, CI, and headless systemd boot.
    pub fn headless() -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let handle = thread::Builder::new()
            .name("program-output-headless".into())
            .spawn(move || run_headless_loop(rx))
            .expect("spawn headless program-output");
        Self {
            tx: thread_tx,
            _thread: std::sync::Arc::new(handle),
        }
    }

    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let handle = thread::Builder::new()
            .name("program-output".into())
            .spawn(move || run_program_loop(rx))
            .expect("spawn program-output thread");
        Self {
            tx: thread_tx,
            _thread: std::sync::Arc::new(handle),
        }
    }

    pub fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(ProgramRequest::ListDisplays(reply_tx))
            .context("program output thread stopped")?;
        reply_rx
            .recv()
            .context("program output list_displays reply")
    }

    pub fn open_window(&self, display_id: u32, fullscreen: bool) -> Result<usize> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(ProgramRequest::OpenWindow {
                display_id,
                fullscreen,
                reply: reply_tx,
            })
            .context("program output thread stopped")?;
        reply_rx.recv().context("program output open_window reply")?
    }

    pub fn close_window(&self) {
        let _ = self.tx.send(ProgramRequest::CloseWindow);
    }
}

struct ProgramApp {
    rx: Receiver<ProgramRequest>,
    window: Option<Window>,
    cached_handle: Option<usize>,
    pending_open: Option<(u32, bool, Sender<Result<usize>>)>,
    pending_list: Option<Sender<Vec<DisplayInfo>>>,
    resumed: bool,
}

impl ProgramApp {
    fn new(rx: Receiver<ProgramRequest>) -> Self {
        Self {
            rx,
            window: None,
            cached_handle: None,
            pending_open: None,
            pending_list: None,
            resumed: false,
        }
    }

    fn drain_requests(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(req) = self.rx.try_recv() {
            match req {
                ProgramRequest::ListDisplays(reply) => {
                    if self.resumed {
                        let _ = reply.send(self.enumerate_displays());
                    } else {
                        self.pending_list = Some(reply);
                    }
                }
                ProgramRequest::OpenWindow {
                    display_id,
                    fullscreen,
                    reply,
                } => {
                    if self.resumed {
                        let result = self.create_program_window(event_loop, display_id, fullscreen);
                        let _ = reply.send(result);
                    } else {
                        self.pending_open = Some((display_id, fullscreen, reply));
                    }
                }
                ProgramRequest::CloseWindow => {
                    self.window = None;
                    self.cached_handle = None;
                }
                ProgramRequest::Shutdown => {
                    event_loop.exit();
                }
            }
        }
    }

    fn enumerate_displays(&self) -> Vec<DisplayInfo> {
        let Some(window) = self.window.as_ref() else {
            return vec![DisplayInfo {
                id: 0,
                name: "Primary Display".into(),
                primary: true,
                width: 1920,
                height: 1080,
            }];
        };

        let primary = window.primary_monitor();
        let monitors: Vec<_> = window.available_monitors().collect();
        monitors
            .into_iter()
            .enumerate()
            .map(|(id, monitor)| {
                let size = monitor.size();
                let name = monitor
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("Display {id}"));
                let is_primary = primary
                    .as_ref()
                    .map(|p| p.name() == monitor.name())
                    .unwrap_or(id == 0);
                DisplayInfo {
                    id: id as u32,
                    name,
                    primary: is_primary,
                    width: size.width as i32,
                    height: size.height as i32,
                }
            })
            .collect()
    }

    fn create_program_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        display_id: u32,
        fullscreen: bool,
    ) -> Result<usize> {
        let monitors: Vec<_> = self
            .window
            .as_ref()
            .map(|w| w.available_monitors().collect())
            .unwrap_or_default();

        let monitor = monitors
            .get(display_id as usize)
            .cloned()
            .or_else(|| monitors.first().cloned())
            .or_else(|| event_loop.primary_monitor());

        self.window = None;
        self.cached_handle = None;

        let mut attrs = WindowAttributes::default()
            .with_title("Instant Replay — Program")
            .with_decorations(false)
            .with_visible(true)
            .with_resizable(false);

        if fullscreen {
            if let Some(ref mon) = monitor {
                attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(Some(mon.clone()))));
            } else {
                attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        } else if let Some(ref mon) = monitor {
            let size = mon.size();
            let pos = mon.position();
            use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
            attrs = attrs
                .with_inner_size(Size::Logical(LogicalSize::new(
                    size.width as f64,
                    size.height as f64,
                )))
                .with_position(Position::Logical(LogicalPosition::new(
                    pos.x as f64,
                    pos.y as f64,
                )));
        }

        let window = event_loop
            .create_window(attrs)
            .context("create program window")?;
        window.set_cursor_visible(false);

        let handle = window_handle_from_winit(&window)?;
        info!(display_id, fullscreen, handle, "Program output window ready");
        self.cached_handle = Some(handle);
        self.window = Some(window);
        Ok(handle)
    }

    fn flush_pending(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(reply) = self.pending_list.take() {
            let _ = reply.send(self.enumerate_displays());
        }
        if let Some((display_id, fullscreen, reply)) = self.pending_open.take() {
            let result = self.create_program_window(event_loop, display_id, fullscreen);
            let _ = reply.send(result);
        }
    }
}

impl ApplicationHandler for ProgramApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resumed = true;
        if self.window.is_none() {
            // Hidden probe window so monitor enumeration works before first OpenWindow.
            let attrs = WindowAttributes::default()
                .with_title("Instant Replay")
                .with_visible(false)
                .with_decorations(false);
            if let Ok(w) = event_loop.create_window(attrs) {
                self.window = Some(w);
            }
        }
        self.flush_pending(event_loop);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            // Operator display stays up until engine stops.
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_requests(event_loop);
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

fn build_program_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::EventLoopBuilderExtWayland;
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_any_thread(true);
    }
    builder.build()
}

fn run_program_loop(rx: Receiver<ProgramRequest>) {
    let event_loop = match build_program_event_loop() {
        Ok(el) => el,
        Err(e) => {
            warn!(error = %e, "Failed to build winit event loop for program output");
            return;
        }
    };

    let mut app = ProgramApp::new(rx);
    if let Err(e) = event_loop.run_app(&mut app) {
        warn!(error = %e, "Program output event loop exited");
    }
}
