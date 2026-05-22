//! OpenGL + egui_glow bootstrap for operator window (from egui pure_glow example).

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Context as AnyhowContext, Result};
use egui_glow::winit::EguiGlow;
use glutin::context::{ContextAttributesBuilder, NotCurrentGlContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlSurface;
use glutin::surface::{Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glutin_winit::{ApiPreference, DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

pub struct OperatorGl {
    pub window: Window,
    pub gl_context: glutin::context::PossiblyCurrentContext,
    pub gl_surface: Surface<WindowSurface>,
    pub egui_glow: EguiGlow,
}

impl OperatorGl {
    pub unsafe fn new(
        event_loop: &ActiveEventLoop,
        window_attrs: WindowAttributes,
    ) -> Result<Self> {
        let template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false);

        let (mut win_opt, gl_config) = DisplayBuilder::new()
            .with_preference(ApiPreference::FallbackEgl)
            .with_window_attributes(None)
            .build(
                event_loop,
                template,
                |mut configs| {
                    configs
                        .next()
                        .expect("no matching GL config for operator UI")
                },
            )
            .map_err(|e| anyhow::anyhow!("DisplayBuilder: {e}"))?;

        let window = if let Some(w) = win_opt.take() {
            w
        } else {
            glutin_winit::finalize_window(event_loop, window_attrs, &gl_config)
                .map_err(|e| anyhow::anyhow!("finalize_window: {e}"))?
        };

        let gl_display = gl_config.display();
        let raw = window
            .window_handle()
            .context("window handle")?
            .as_raw();

        let ctx_attrs = ContextAttributesBuilder::new().build(Some(raw));
        let fallback = ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(Some(raw));

        let not_current = gl_display
            .create_context(&gl_config, &ctx_attrs)
            .or_else(|_| gl_display.create_context(&gl_config, &fallback))
            .map_err(|e| anyhow::anyhow!("create_context: {e}"))?;

        let size = window.inner_size();
        let w = NonZeroU32::new(size.width.max(1)).context("width")?;
        let h = NonZeroU32::new(size.height.max(1)).context("height")?;
        let surf_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().context("handle")?.as_raw(),
            w,
            h,
        );
        let gl_surface = gl_display
            .create_window_surface(&gl_config, &surf_attrs)
            .map_err(|e| anyhow::anyhow!("create_window_surface: {e}"))?;

        let gl_context = not_current
            .make_current(&gl_surface)
            .map_err(|e| anyhow::anyhow!("make_current: {e}"))?;

        gl_surface
            .set_swap_interval(
                &gl_context,
                SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            )
            .map_err(|e| anyhow::anyhow!("swap_interval: {e}"))?;

        let gl_display_proc = gl_display.clone();
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function(move |s| {
                let name = std::ffi::CString::new(s).unwrap();
                gl_display_proc.get_proc_address(&name)
            })
        });

        let scale = window.scale_factor() as f32;
        let egui_glow = EguiGlow::new(event_loop, gl, None, Some(scale), true);

        Ok(Self {
            window,
            gl_context,
            gl_surface,
            egui_glow,
        })
    }

    pub fn paint(&mut self, run_ui: impl FnMut(&egui::Context)) {
        self.egui_glow.run(&self.window, run_ui);
        self.egui_glow.paint(&self.window);
        let _ = self.gl_surface.swap_buffers(&self.gl_context);
        self.window.request_redraw();
    }

    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) -> egui_winit::EventResponse {
        self.egui_glow.on_window_event(&self.window, event)
    }

    pub fn resize(&mut self) {
        self.window.resize_surface(&self.gl_surface, &self.gl_context);
    }
}
