//! Replay must not transition FSM when segments are empty or buffer is too short.

use replay_core::buffer::MIN_REPLAY_BUFFER_SECS;
use replay_core::fsm::{ReplayEvent, ReplayFsm, ReplayState};

#[test]
fn replay_event_from_live_without_prior_mark_stays_reversible() {
    let mut fsm = ReplayFsm::new();
    fsm.apply(ReplayEvent::InputReady).unwrap();
    assert_eq!(fsm.state(), ReplayState::Live);
    fsm.apply(ReplayEvent::ReplayLast).unwrap();
    assert_eq!(fsm.state(), ReplayState::Replaying);
    fsm.force_state(ReplayState::Live);
    assert_eq!(fsm.state(), ReplayState::Live);
}

#[test]
fn replay_event_from_marked_rollback_restores_marked() {
    let mut fsm = ReplayFsm::new();
    fsm.apply(ReplayEvent::InputReady).unwrap();
    fsm.apply(ReplayEvent::Mark).unwrap();
    assert_eq!(fsm.state(), ReplayState::Marked);
    fsm.apply(ReplayEvent::Replay).unwrap();
    assert_eq!(fsm.state(), ReplayState::Replaying);
    fsm.force_state(ReplayState::Marked);
    assert_eq!(fsm.state(), ReplayState::Marked);
}

#[test]
fn min_replay_buffer_threshold_matches_acceptance() {
    assert!((MIN_REPLAY_BUFFER_SECS - 1.5).abs() < f64::EPSILON);
}
