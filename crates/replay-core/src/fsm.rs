use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplayState {
    Starting,
    NoSignal,
    Live,
    Marked,
    Replaying,
    ReturningToLive,
    ErrorRecovery,
}

impl ReplayState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "STARTING",
            Self::NoSignal => "NO_SIGNAL",
            Self::Live => "LIVE",
            Self::Marked => "MARKED",
            Self::Replaying => "REPLAYING",
            Self::ReturningToLive => "RETURNING_TO_LIVE",
            Self::ErrorRecovery => "ERROR_RECOVERY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEvent {
    InputReady,
    InputLost,
    Mark,
    Replay,
    ReplayLast,
    ReturnLive,
    ClearMark,
    ReplayFinished,
    Recover,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FsmError {
    InvalidTransition {
        state: ReplayState,
        event: ReplayEvent,
    },
}

impl std::fmt::Display for FsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { state, event } => {
                write!(f, "invalid transition in state {state:?} on event {event:?}")
            }
        }
    }
}

impl std::error::Error for FsmError {}

#[derive(Debug, Clone)]
pub struct ReplayFsm {
    state: ReplayState,
}

impl Default for ReplayFsm {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayFsm {
    pub fn new() -> Self {
        Self {
            state: ReplayState::Starting,
        }
    }

    pub fn state(&self) -> ReplayState {
        self.state
    }

    pub fn apply(&mut self, event: ReplayEvent) -> Result<ReplayState, FsmError> {
        let next = transition(self.state, event)?;
        self.state = next;
        Ok(self.state)
    }

    pub fn force_state(&mut self, state: ReplayState) {
        self.state = state;
    }
}

fn transition(state: ReplayState, event: ReplayEvent) -> Result<ReplayState, FsmError> {
    use ReplayEvent::*;
    use ReplayState::*;

    let next = match (state, event) {
        (Starting, InputReady) => Live,
        (Starting, InputLost) => NoSignal,
        (Starting, Error) => ErrorRecovery,

        (NoSignal, InputReady) => Live,
        (NoSignal, Recover) => Live,

        (Live, InputLost) => NoSignal,
        (Live, Mark) => Marked,
        (Live, Replay) => Replaying,
        (Live, ReplayLast) => Replaying,
        (Live, Error) => ErrorRecovery,

        (Marked, InputLost) => NoSignal,
        (Marked, Replay) => Replaying,
        (Marked, ReplayLast) => Replaying,
        (Marked, ClearMark) => Live,
        (Marked, ReturnLive) => Live,
        (Marked, Error) => ErrorRecovery,

        (Replaying, ReplayFinished) => ReturningToLive,
        (Replaying, ReturnLive) => Live,
        (Replaying, InputLost) => NoSignal,
        (Replaying, Error) => ErrorRecovery,

        (ReturningToLive, InputReady) | (ReturningToLive, Recover) => Live,
        (ReturningToLive, InputLost) => NoSignal,

        (ErrorRecovery, Recover) | (ErrorRecovery, InputReady) => Live,
        (ErrorRecovery, InputLost) => NoSignal,

        (s, e) => return Err(FsmError::InvalidTransition { state: s, event: e }),
    };

    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_all(fsm: &mut ReplayFsm, events: &[ReplayEvent]) -> Result<(), FsmError> {
        for &e in events {
            fsm.apply(e)?;
        }
        Ok(())
    }

    #[test]
    fn starting_to_live() {
        let mut fsm = ReplayFsm::new();
        assert_eq!(fsm.state(), ReplayState::Starting);
        fsm.apply(ReplayEvent::InputReady).unwrap();
        assert_eq!(fsm.state(), ReplayState::Live);
    }

    #[test]
    fn live_mark_replay_live() {
        let mut fsm = ReplayFsm::new();
        apply_all(
            &mut fsm,
            &[
                ReplayEvent::InputReady,
                ReplayEvent::Mark,
                ReplayEvent::Replay,
                ReplayEvent::ReplayFinished,
                ReplayEvent::Recover,
            ],
        )
        .unwrap();
        assert_eq!(fsm.state(), ReplayState::Live);
    }

    #[test]
    fn marked_clear_returns_live() {
        let mut fsm = ReplayFsm::new();
        apply_all(
            &mut fsm,
            &[
                ReplayEvent::InputReady,
                ReplayEvent::Mark,
                ReplayEvent::ClearMark,
            ],
        )
        .unwrap();
        assert_eq!(fsm.state(), ReplayState::Live);
    }

    #[test]
    fn replay_interrupt_return_live() {
        let mut fsm = ReplayFsm::new();
        apply_all(
            &mut fsm,
            &[
                ReplayEvent::InputReady,
                ReplayEvent::ReplayLast,
                ReplayEvent::ReturnLive,
            ],
        )
        .unwrap();
        assert_eq!(fsm.state(), ReplayState::Live);
    }

    #[test]
    fn signal_loss_from_any_live_path() {
        let mut fsm = ReplayFsm::new();
        apply_all(&mut fsm, &[ReplayEvent::InputReady]).unwrap();
        fsm.apply(ReplayEvent::InputLost).unwrap();
        assert_eq!(fsm.state(), ReplayState::NoSignal);
    }

    #[test]
    fn invalid_transition_rejected() {
        let mut fsm = ReplayFsm::new();
        let err = fsm.apply(ReplayEvent::Mark).unwrap_err();
        assert_eq!(
            err,
            FsmError::InvalidTransition {
                state: ReplayState::Starting,
                event: ReplayEvent::Mark,
            }
        );
    }
}
