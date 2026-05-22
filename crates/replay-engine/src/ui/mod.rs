//! Unified winit UI: native operator shell + HDMI program window.

mod gates;
mod operator_gl;
mod operator_shell;
mod program_window;
pub mod runtime;

pub use operator_shell::OperatorCmd;
pub use runtime::{should_use_headless, ProgramOutputHandle, UiSpawnConfig};
