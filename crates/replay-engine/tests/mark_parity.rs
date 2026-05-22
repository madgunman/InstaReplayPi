//! Mark snapshot vs worker index must agree on the same timeline helper.

use replay_core::buffer::{ChunkIndex, OpenFragmentMarkInput, timeline_mark_now};

#[test]
fn snapshot_mark_matches_create_mark_with_open_fragment() {
    let dir = std::env::temp_dir().join("replay-mark-parity-open");
    let _ = std::fs::create_dir_all(&dir);
    let mut idx = ChunkIndex::new(dir.clone(), 20, 2);
    let path = dir.join("chunk_00012.mkv");
    idx.on_fragment_opened(path);

    let from_index = idx.create_mark().unwrap();
    let open = OpenFragmentMarkInput {
        chunk_id: from_index.chunk_id.clone(),
        opened_unix_ms: from_index.unix_ms - from_index.offset_ms as i64,
    };
    let from_snapshot = timeline_mark_now(Some(&open), None).unwrap();
    assert_eq!(from_index.chunk_id, from_snapshot.chunk_id);
    assert_eq!(from_index.offset_ms, from_snapshot.offset_ms);
}

#[test]
fn snapshot_mark_matches_create_mark_with_finalized_only() {
    let dir = std::env::temp_dir().join("replay-mark-parity-final");
    let _ = std::fs::create_dir_all(&dir);
    let mut idx = ChunkIndex::new(dir.clone(), 20, 1);
    let p = dir.join("chunk_00003.mkv");
    std::fs::write(&p, b"x").ok();
    idx.on_fragment_closed(p, Some(1000));

    let from_index = idx.create_mark().unwrap();
    let from_snapshot = timeline_mark_now(None, Some("00003")).unwrap();
    assert_eq!(from_index.chunk_id, from_snapshot.chunk_id);
    assert_eq!(from_index.offset_ms, 0);
}
