use replay_core::fsm::{ReplayEvent, ReplayFsm, ReplayState};

#[test]
fn full_table_tennis_flow() {
    let mut fsm = ReplayFsm::new();
    fsm.apply(ReplayEvent::InputReady).unwrap();
    assert_eq!(fsm.state(), ReplayState::Live);
    fsm.apply(ReplayEvent::Mark).unwrap();
    assert_eq!(fsm.state(), ReplayState::Marked);
    fsm.apply(ReplayEvent::Replay).unwrap();
    assert_eq!(fsm.state(), ReplayState::Replaying);
    fsm.apply(ReplayEvent::ReplayFinished).unwrap();
    assert_eq!(fsm.state(), ReplayState::ReturningToLive);
    fsm.apply(ReplayEvent::Recover).unwrap();
    assert_eq!(fsm.state(), ReplayState::Live);
}
