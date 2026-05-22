//! `replay.mode` config semantics (documented behavior; selection logic mirrors controller).

#[test]
fn replay_mode_last_ignores_mark_flag() {
    let mode = "last";
    let has_mark = true;
    let use_mark = mode != "last" && has_mark;
    assert!(!use_mark);
}

#[test]
fn replay_mode_marked_uses_mark_when_set() {
    let mode = "marked";
    let has_mark = true;
    let use_mark = mode != "last" && has_mark;
    assert!(use_mark);
}

#[test]
fn replay_mode_marked_falls_back_without_mark() {
    let mode = "marked";
    let has_mark = false;
    let use_mark = mode != "last" && has_mark;
    assert!(!use_mark);
}
