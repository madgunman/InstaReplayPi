//! Audience HDMI program window (borderless fullscreen).

use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use replay_core::types::DisplayInfo;
use tracing::info;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowAttributes};

use crate::pipeline::program_sink::window_handle_from_winit;

pub struct ProgramWindowState {
    pub window: Option<Window>,
    pub cached_handle: Option<usize>,
    pub open_display_id: Option<u32>,
    pub open_fullscreen: Option<bool>,
    pub pending_open: Option<(u32, bool, Sender<Result<usize>>)>,
    pub pending_list: Option<Sender<Vec<DisplayInfo>>>,
    pub resumed: bool,
}

impl ProgramWindowState {
    pub fn new() -> Self {
        Self {
            window: None,
            cached_handle: None,
            open_display_id: None,
            open_fullscreen: None,
            pending_open: None,
            pending_list: None,
            resumed: false,
        }
    }

    pub fn ensure_probe_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Instant Replay")
            .with_visible(false)
            .with_decorations(false);
        if let Ok(w) = event_loop.create_window(attrs) {
            self.window = Some(w);
        }
    }

    pub fn enumerate_displays(&self) -> Vec<DisplayInfo> {
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

    pub fn create_program_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        display_id: u32,
        fullscreen: bool,
    ) -> Result<usize> {
        if let Some(handle) = self.cached_handle {
            if self.open_display_id == Some(display_id)
                && self.open_fullscreen == Some(fullscreen)
                && self.window.is_some()
            {
                info!(display_id, fullscreen, handle, "Reusing program output window");
                return Ok(handle);
            }
        }

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
        self.open_display_id = Some(display_id);
        self.open_fullscreen = Some(fullscreen);
        self.window = Some(window);
        Ok(handle)
    }

    pub fn flush_pending(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(reply) = self.pending_list.take() {
            let _ = reply.send(self.enumerate_displays());
        }
        if let Some((display_id, fullscreen, reply)) = self.pending_open.take() {
            let result = self.create_program_window(event_loop, display_id, fullscreen);
            let _ = reply.send(result);
        }
    }

    pub fn close(&mut self) {
        self.window = None;
        self.cached_handle = None;
        self.open_display_id = None;
        self.open_fullscreen = None;
    }
}
