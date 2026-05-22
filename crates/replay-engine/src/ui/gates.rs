//! Operator button enable rules (parity with legacy touch UI).

use replay_core::fsm::ReplayState;

use crate::controller::StatusSnapshot;

pub const MIN_BUFFER_SECS: f64 = 1.5;

pub fn can_mark(s: &StatusSnapshot) -> bool {
    s.buffer_ready && matches!(s.state, ReplayState::Live | ReplayState::Marked)
}

pub fn can_replay(s: &StatusSnapshot) -> bool {
    s.buffer_seconds_available >= MIN_BUFFER_SECS
        && matches!(
            s.state,
            ReplayState::Live | ReplayState::Marked | ReplayState::Replaying
        )
}

pub fn can_clear_mark(s: &StatusSnapshot) -> bool {
    s.state == ReplayState::Marked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::StatusSnapshot;
    use replay_core::fsm::ReplayState;

    fn snap(state: ReplayState, buffer_ready: bool, secs: f64) -> StatusSnapshot {
        StatusSnapshot {
            state,
            input_fps: 50.0,
            dropped_frames: 0,
            buffer_seconds_available: secs,
            disk_warning: false,
            last_error: String::new(),
            buffer_ready,
            buffer_error: false,
            mark_timestamp_ns: 0,
            sequence: 0,
        }
    }

    #[test]
    fn mark_requires_buffer_ready_live_or_marked() {
        assert!(!can_mark(&snap(ReplayState::Live, false, 5.0)));
        assert!(can_mark(&snap(ReplayState::Live, true, 5.0)));
        assert!(can_mark(&snap(ReplayState::Marked, true, 5.0)));
        assert!(!can_mark(&snap(ReplayState::Replaying, true, 5.0)));
    }

    #[test]
    fn replay_requires_min_buffer() {
        assert!(!can_replay(&snap(ReplayState::Live, true, 1.0)));
        assert!(can_replay(&snap(ReplayState::Live, true, 2.0)));
    }
}
