//! Unified winit UI: native operator shell + HDMI program window.

mod gates;
mod operator_gl;
mod operator_shell;
mod program_window;
mod setup_panel;
pub mod runtime;

pub use operator_shell::{show_toast, OperatorCmd};
pub use runtime::{should_use_headless, ProgramOutputHandle, UiSpawnConfig};
pub use setup_panel::SetupUiState;
