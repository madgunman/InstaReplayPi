//! Buffer index behaviour required for MVP acceptance (mark timeline, retention).

use replay_core::buffer::{BufferMark, ChunkIndex};
use std::path::PathBuf;

fn temp_buffer_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "replay-buffer-test-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn close_chunk(index: &mut ChunkIndex, dir: &PathBuf, n: u32, duration_ms: u64) {
    let path = dir.join(format!("chunk_{n:05}.mkv"));
    std::fs::write(&path, vec![0u8; 64]).unwrap();
    index.on_fragment_closed(path, Some(duration_ms));
}

#[test]
fn rolling_buffer_enforces_max_entries() {
    let dir = temp_buffer_dir();
    let mut idx = ChunkIndex::new(dir.clone(), 20, 5);
    for n in 0..10u32 {
        close_chunk(&mut idx, &dir, n, 5000);
    }
    assert!(
        idx.entries().len() <= 6,
        "retention should cap entries (~20s / 5s + 2), got {}",
        idx.entries().len()
    );
}

#[test]
fn mark_and_replay_segments_from_timeline() {
    let dir = temp_buffer_dir();
    let mut idx = ChunkIndex::new(dir.clone(), 30, 5);
    for n in 0..4u32 {
        close_chunk(&mut idx, &dir, n, 10_000);
    }
    let mark = BufferMark {
        chunk_id: "00001".into(),
        offset_ms: 3000,
        unix_ms: 2_010_000,
    };
    idx.set_mark(mark);
    let segs = idx.segments_from_buffer_mark();
    assert!(!segs.is_empty(), "mark should yield replay segments");
    let last = idx.segments_last_seconds(10);
    assert!(!last.is_empty(), "replay last should find segments");
}
