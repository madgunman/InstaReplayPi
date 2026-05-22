use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use replay_core::config::AppConfig;

use crate::chunk_index_actor::ChunkIndexHandle;
use crate::pipeline::CapturePipeline;
use crate::program_output::ProgramOutputHandle;
use crate::video_stats::VideoStats;

/// Handles returned when live capture starts.
pub struct LiveCaptureHandles {
    pub chunk_index: ChunkIndexHandle,
    pub video_stats: Arc<VideoStats>,
}

pub enum RuntimeCommand {
    StartLive {
        config: AppConfig,
        width: u32,
        height: u32,
        fps: u32,
        replay_finished_tx: Option<Sender<()>>,
        signal_lost_tx: Option<Sender<()>>,
        signal_restored_tx: Option<Sender<()>>,
        buffer_secs_cache: Arc<AtomicU64>,
        reply: Sender<Result<LiveCaptureHandles>>,
    },
    Stop {
        reply: Sender<Result<()>>,
    },
    ReplaySegments {
        segments: Vec<std::path::PathBuf>,
        rate: f64,
        first_offset_ms: u64,
        reply: Sender<Result<()>>,
    },
    ReturnLive {
        reply: Sender<Result<()>>,
    },
    SignalLost {
        reply: Sender<Result<()>>,
    },
    SignalRestored {
        reply: Sender<Result<()>>,
    },
    Shutdown,
}

pub struct GstreamerRuntime {
    tx: Sender<RuntimeCommand>,
    _handle: JoinHandle<()>,
    program: ProgramOutputHandle,
}

impl GstreamerRuntime {
    pub fn with_program(program: ProgramOutputHandle) -> Self {
        let headless = false;
        let (tx, rx) = mpsc::channel();
        let program_for_thread = program.clone();
        let handle = thread::Builder::new()
            .name("gstreamer-runtime".into())
            .spawn(move || run_loop(rx, program_for_thread, headless))
            .expect("spawn gstreamer runtime");
        Self {
            tx,
            _handle: handle,
            program,
        }
    }

    pub fn spawn(headless: bool, program: ProgramOutputHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        let program_for_thread = program.clone();
        let handle = thread::Builder::new()
            .name("gstreamer-runtime".into())
            .spawn(move || run_loop(rx, program_for_thread, headless))
            .expect("spawn gstreamer runtime");
        Self {
            tx,
            _handle: handle,
            program,
        }
    }

    pub fn program_output(&self) -> &ProgramOutputHandle {
        &self.program
    }

    fn send(&self, cmd: RuntimeCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn start_live(
        &self,
        config: AppConfig,
        width: u32,
        height: u32,
        fps: u32,
        replay_finished_tx: Option<Sender<()>>,
        signal_lost_tx: Option<Sender<()>>,
        signal_restored_tx: Option<Sender<()>>,
        buffer_secs_cache: Arc<AtomicU64>,
    ) -> Result<LiveCaptureHandles> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::StartLive {
            config,
            width,
            height,
            fps,
            replay_finished_tx,
            signal_lost_tx,
            signal_restored_tx,
            buffer_secs_cache,
            reply: reply_tx,
        });
        recv_reply(reply_rx)
    }

    pub fn stop(&self) -> Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::Stop { reply: reply_tx });
        recv_reply(reply_rx)
    }

    pub fn replay_segments(
        &self,
        segments: Vec<std::path::PathBuf>,
        rate: f64,
        first_offset_ms: u64,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::ReplaySegments {
            segments,
            rate,
            first_offset_ms,
            reply: reply_tx,
        });
        recv_reply(reply_rx)
    }

    pub fn return_live(&self) -> Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::ReturnLive { reply: reply_tx });
        recv_reply(reply_rx)
    }

    pub fn notify_signal_lost(&self) -> Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::SignalLost { reply: reply_tx });
        recv_reply(reply_rx)
    }

    pub fn notify_signal_restored(&self) -> Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(RuntimeCommand::SignalRestored { reply: reply_tx });
        recv_reply(reply_rx)
    }
}

fn recv_reply<T>(rx: mpsc::Receiver<Result<T>>) -> Result<T> {
    match rx.recv() {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!("gstreamer runtime dropped")),
    }
}

struct RuntimeState {
    capture: Option<CapturePipeline>,
    show_overlay: bool,
    headless: bool,
}

fn run_loop(rx: Receiver<RuntimeCommand>, program: ProgramOutputHandle, headless: bool) {
    // GStreamer bus watches and state changes must run on the same GLib context.
    let context = glib::MainContext::new();
    let mut state = RuntimeState {
        capture: None,
        show_overlay: false,
        headless,
    };

    let _ = context.with_thread_default(|| {
        let mut running = true;
        while running {
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok(RuntimeCommand::Shutdown) => {
                    if let Some(mut cap) = state.capture.take() {
                        let _ = cap.stop();
                    }
                    program.close_window();
                    running = false;
                }
                Ok(cmd) => dispatch_runtime_command(cmd, &mut state, &program),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => running = false,
            }
            // One non-blocking iteration — do not spin on pending() or gRPC starves.
            let _ = context.iteration(false);
        }
    });
}

fn dispatch_runtime_command(
    cmd: RuntimeCommand,
    state: &mut RuntimeState,
    program: &ProgramOutputHandle,
) {
    match cmd {
            RuntimeCommand::StartLive {
                mut config,
                width,
                height,
                fps,
                replay_finished_tx,
                signal_lost_tx,
                signal_restored_tx,
                buffer_secs_cache,
                reply,
            } => {
                config.input.resolution = format!("{width}x{height}");
                config.input.fps = fps;
                let result = (|| -> Result<LiveCaptureHandles> {
                    if let Some(mut cap) = state.capture.take() {
                        let _ = cap.stop();
                    }

                    let display_id = config.output.display_id;
                    let fullscreen = config.output.fullscreen;
                    state.show_overlay = config.output.show_status_overlay;

                    let handle = program.open_window(display_id, fullscreen)?;

                    buffer_secs_cache.store(0, std::sync::atomic::Ordering::Relaxed);
                    let mut cap = CapturePipeline::build(
                        &config,
                        handle,
                        state.headless,
                        replay_finished_tx,
                        signal_lost_tx,
                        signal_restored_tx,
                        buffer_secs_cache,
                    )?;
                    cap.start()?;
                    if state.show_overlay {
                        cap.set_status_overlay("LIVE")?;
                    }
                    let handles = LiveCaptureHandles {
                        chunk_index: cap.chunk_index(),
                        video_stats: cap.video_stats(),
                    };
                    state.capture = Some(cap);
                    tracing::info!("Capture pipeline playing on display {display_id}");
                    Ok(handles)
                })();
                if let Err(ref e) = result {
                    tracing::error!(error = %e, "Start live failed");
                }
                let _ = reply.send(result);
            }
            RuntimeCommand::Stop { reply } => {
                let result = (|| {
                    if let Some(mut cap) = state.capture.take() {
                        let _ = cap.stop();
                    }
                    program.close_window();
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            RuntimeCommand::ReplaySegments {
                segments,
                rate,
                first_offset_ms,
                reply,
            } => {
                let result = (|| {
                    let cap = state
                        .capture
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("capture not running"))?;
                    cap.start_replay(segments, rate, first_offset_ms, state.show_overlay)?;
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            RuntimeCommand::ReturnLive { reply } => {
                let result = (|| {
                    let cap = state
                        .capture
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("capture not running"))?;
                    cap.return_to_live(state.show_overlay)?;
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            RuntimeCommand::SignalLost { reply } => {
                let result = (|| {
                    if let Some(cap) = state.capture.as_mut() {
                        cap.on_signal_lost(state.show_overlay)?;
                    }
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            RuntimeCommand::SignalRestored { reply } => {
                let result = (|| {
                    if let Some(cap) = state.capture.as_mut() {
                        cap.on_signal_restored(state.show_overlay)?;
                    }
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            RuntimeCommand::Shutdown => {}
        }
}
