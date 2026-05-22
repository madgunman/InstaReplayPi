//! Native operator touch UI (egui + glow on Pi touch display).

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use egui::{Color32, RichText, Rounding};
use replay_core::config::OperatorConfig;
use replay_core::fsm::ReplayState;
use tracing::warn;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, WindowAttributes};

use crate::controller::StatusSnapshot;
use crate::ui::gates::{can_clear_mark, can_mark, can_replay};
use crate::ui::operator_gl::OperatorGl;

const TOAST_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy)]
pub enum OperatorCmd {
    Mark,
    Replay,
    ReplayLast,
    ReturnLive,
    ClearMark,
}

pub struct OperatorShell {
    pub config: OperatorConfig,
    pub gl: Option<OperatorGl>,
    pub window_id: Option<winit::window::WindowId>,
    pub status: Arc<RwLock<StatusSnapshot>>,
    pub toast: Arc<Mutex<Option<(String, bool, Instant)>>>,
    pub cmd_tx: Sender<OperatorCmd>,
    pub resumed: bool,
}

impl OperatorShell {
    pub fn new(
        config: OperatorConfig,
        status: Arc<RwLock<StatusSnapshot>>,
        cmd_tx: Sender<OperatorCmd>,
    ) -> Self {
        Self {
            config,
            gl: None,
            window_id: None,
            status,
            toast: Arc::new(Mutex::new(None)),
            cmd_tx,
            resumed: false,
        }
    }

    fn window_attributes(&self, event_loop: &ActiveEventLoop) -> WindowAttributes {
        let monitors: Vec<_> = event_loop.available_monitors().collect();
        let monitor = monitors
            .get(self.config.display_id as usize)
            .cloned()
            .or_else(|| monitors.first().cloned())
            .or_else(|| event_loop.primary_monitor());

        let mut attrs = WindowAttributes::default()
            .with_title("Instant Replay — Operator")
            .with_decorations(!self.config.fullscreen)
            .with_resizable(false)
            .with_visible(true)
            .with_inner_size(Size::Logical(LogicalSize::new(
                self.config.width as f64,
                self.config.height as f64,
            )));

        if self.config.fullscreen {
            if let Some(ref mon) = monitor {
                attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(Some(mon.clone()))));
            }
        } else if let Some(ref mon) = monitor {
            let pos = mon.position();
            attrs = attrs.with_position(Position::Logical(LogicalPosition::new(
                pos.x as f64,
                pos.y as f64,
            )));
        }
        attrs
    }

    pub fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.gl.is_some() {
            return;
        }
        let attrs = self.window_attributes(event_loop);
        match unsafe { OperatorGl::new(event_loop, attrs) } {
            Ok(gl) => {
                self.window_id = Some(gl.window.id());
                gl.window.request_redraw();
                self.gl = Some(gl);
            }
            Err(e) => warn!(error = %e, "Failed to create operator GL window"),
        }
    }

    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        let Some(gl) = self.gl.as_mut() else {
            return;
        };
        if gl.on_window_event(event).repaint {
            gl.window.request_redraw();
        }
        if matches!(event, winit::event::WindowEvent::Resized(_)) {
            gl.resize();
        }
    }

    pub fn on_redraw(&mut self) {
        let Some(gl) = self.gl.as_mut() else {
            return;
        };
        let status = self
            .status
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let toast = self.toast.lock().ok().and_then(|mut t| {
            if let Some((msg, is_err, at)) = t.as_ref() {
                if at.elapsed() < TOAST_DURATION {
                    return Some((msg.clone(), *is_err));
                }
                *t = None;
            }
            None
        });
        let cmd_tx = self.cmd_tx.clone();
        let toast_tx = self.toast.clone();
        gl.paint(move |ctx| {
            paint_operator_ui(ctx, &status, toast.clone(), &cmd_tx, toast_tx.clone());
        });
    }
}

fn paint_operator_ui(
    ctx: &egui::Context,
    status: &StatusSnapshot,
    toast: Option<(String, bool)>,
    cmd_tx: &Sender<OperatorCmd>,
    toast_tx: Arc<Mutex<Option<(String, bool, Instant)>>>,
) {
    let (banner_bg, banner_text) = state_colors(status.state);

    egui::TopBottomPanel::top("banner").show(ctx, |ui| {
        egui::Frame::default()
            .fill(banner_bg)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(status.state.as_str())
                            .size(22.0)
                            .strong()
                            .color(banner_text),
                    );
                    let detail = format!(
                        "Buffer: {:.1}s{}",
                        status.buffer_seconds_available,
                        if status.last_error.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", status.last_error)
                        }
                    );
                    ui.label(RichText::new(detail).size(14.0).color(Color32::WHITE));
                });
            });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(16.0, 20.0);
        let btn_w = (ui.available_width() / 2.0 - 8.0).max(120.0);
        let btn_size = egui::vec2(btn_w, 72.0);

        ui.columns(2, |cols| {
            if cols[0]
                .add_enabled(
                    can_mark(status),
                    egui::Button::new("Mark (M)").min_size(btn_size),
                )
                .clicked()
            {
                send_cmd(cmd_tx, toast_tx.clone(), OperatorCmd::Mark);
            }
            if cols[1]
                .add_enabled(
                    can_replay(status),
                    egui::Button::new("Replay (R)").min_size(btn_size),
                )
                .clicked()
            {
                send_cmd(cmd_tx, toast_tx.clone(), OperatorCmd::Replay);
            }
            if cols[0]
                .add_enabled(
                    can_replay(status),
                    egui::Button::new("Last 10s (Space)").min_size(btn_size),
                )
                .clicked()
            {
                send_cmd(cmd_tx, toast_tx.clone(), OperatorCmd::ReplayLast);
            }
            if cols[1]
                .add_enabled(true, egui::Button::new("Live (L)").min_size(btn_size))
                .clicked()
            {
                send_cmd(cmd_tx, toast_tx.clone(), OperatorCmd::ReturnLive);
            }
            if cols[0]
                .add_enabled(
                    can_clear_mark(status),
                    egui::Button::new("Clear mark (C)").min_size(btn_size),
                )
                .clicked()
            {
                send_cmd(cmd_tx, toast_tx.clone(), OperatorCmd::ClearMark);
            }
        });
    });

    if let Some((msg, is_err)) = toast {
        egui::Area::new(egui::Id::new("toast"))
            .fixed_pos(egui::pos2(16.0, ctx.screen_rect().max.y - 56.0))
            .show(ctx, |ui| {
                let bg = if is_err {
                    Color32::from_rgb(139, 34, 34)
                } else {
                    Color32::from_rgb(51, 51, 51)
                };
                egui::Frame::default()
                    .fill(bg)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).color(Color32::WHITE));
                    });
            });
    }
}

fn send_cmd(
    cmd_tx: &Sender<OperatorCmd>,
    toast: Arc<Mutex<Option<(String, bool, Instant)>>>,
    cmd: OperatorCmd,
) {
    if cmd_tx.send(cmd).is_err() {
        if let Ok(mut t) = toast.lock() {
            *t = Some(("Engine stopped".into(), true, Instant::now()));
        }
    }
}

fn state_colors(state: ReplayState) -> (Color32, Color32) {
    match state {
        ReplayState::NoSignal | ReplayState::ErrorRecovery => {
            (Color32::from_rgb(139, 34, 34), Color32::WHITE)
        }
        ReplayState::Replaying | ReplayState::ReturningToLive => {
            (Color32::from_rgb(139, 69, 19), Color32::WHITE)
        }
        _ => (Color32::from_rgb(61, 90, 61), Color32::WHITE),
    }
}
