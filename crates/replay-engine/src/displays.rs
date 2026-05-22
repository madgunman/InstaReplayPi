use replay_core::types::DisplayInfo;

use crate::program_output::ProgramOutputHandle;

pub fn list_displays(program: &ProgramOutputHandle) -> Vec<DisplayInfo> {
    program.list_displays().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "list_displays failed; using primary fallback");
        vec![DisplayInfo {
            id: 0,
            name: "Primary Display".into(),
            primary: true,
            width: 1920,
            height: 1080,
        }]
    })
}
