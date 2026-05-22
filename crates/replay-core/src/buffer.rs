use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Minimum buffered seconds required before replay is allowed.
pub const MIN_REPLAY_BUFFER_SECS: f64 = 1.5;

/// Mark position on the rolling buffer timeline (chunk + offset within chunk).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferMark {
    pub chunk_id: String,
    pub offset_ms: u64,
    pub unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkEntry {
    pub id: String,
    pub path: PathBuf,
    pub start_unix_ms: i64,
    /// Start offset on the buffer timeline (ms from first chunk in this session).
    #[serde(default)]
    pub buffer_offset_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
struct OpenFragment {
    chunk_id: String,
    path: PathBuf,
    opened_unix_ms: i64,
    buffer_offset_ms: u64,
}

#[derive(Debug, Default)]
pub struct ChunkIndex {
    entries: VecDeque<ChunkEntry>,
    max_entries: usize,
    buffer_path: PathBuf,
    chunk_duration_ms: u64,
    open_fragment: Option<OpenFragment>,
    /// Stored mark on buffer timeline (survives until clear/replay).
    mark: Option<BufferMark>,
}

impl ChunkIndex {
    pub fn new(buffer_path: PathBuf, max_seconds: u32, chunk_seconds: u32) -> Self {
        let chunk_duration_ms = (chunk_seconds.max(1) as u64) * 1000;
        let max_entries = (max_seconds / chunk_seconds.max(1)) as usize + 2;
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
            buffer_path,
            chunk_duration_ms,
            open_fragment: None,
            mark: None,
        }
    }

    pub fn buffer_path(&self) -> &Path {
        &self.buffer_path
    }

    pub fn chunk_duration_ms(&self) -> u64 {
        self.chunk_duration_ms
    }

    pub fn set_mark(&mut self, mark: BufferMark) {
        self.mark = Some(mark);
    }

    pub fn clear_mark(&mut self) {
        self.mark = None;
    }

    pub fn mark(&self) -> Option<&BufferMark> {
        self.mark.as_ref()
    }

    /// Called when splitmuxsink opens a new fragment (not yet safe for replay).
    pub fn on_fragment_opened(&mut self, location: PathBuf) {
        let chunk_id = chunk_id_from_path(&location);
        let opened_unix_ms = unix_now_ms();
        let buffer_offset_ms = self.finalized_duration_ms();
        self.open_fragment = Some(OpenFragment {
            chunk_id,
            path: location,
            opened_unix_ms,
            buffer_offset_ms,
        });
    }

    /// Register a finalized fragment (atomic — file is complete).
    pub fn on_fragment_closed(&mut self, location: PathBuf, duration_ms: Option<u64>) {
        let duration_ms = duration_ms.unwrap_or(self.chunk_duration_ms);
        let (chunk_id, start_unix_ms, buffer_offset_ms) =
            if let Some(open) = self.open_fragment.take() {
                if open.path == location {
                    (
                        open.chunk_id,
                        open.opened_unix_ms,
                        open.buffer_offset_ms,
                    )
                } else {
                    (
                        chunk_id_from_path(&location),
                        unix_now_ms(),
                        self.finalized_duration_ms(),
                    )
                }
            } else {
                (
                    chunk_id_from_path(&location),
                    unix_now_ms(),
                    self.finalized_duration_ms(),
                )
            };

        // Trust splitmuxsink-fragment-closed; avoid blocking the index worker on cloud-synced paths.

        let entry = ChunkEntry {
            id: chunk_id,
            path: location,
            start_unix_ms,
            buffer_offset_ms,
            duration_ms,
        };
        self.entries.push_back(entry);
        self.enforce_retention();
        self.persist_index_background();
    }

    fn enforce_retention(&mut self) {
        let mut to_delete = Vec::new();
        while self.entries.len() > self.max_entries {
            if let Some(old) = self.entries.pop_front() {
                to_delete.push(old.path);
            }
        }
        if !to_delete.is_empty() {
            std::thread::spawn(move || {
                for path in to_delete {
                    if let Err(e) = std::fs::remove_file(&path) {
                        warn!(path = %path.display(), error = %e, "Failed to remove old buffer chunk");
                    }
                }
            });
        }
    }

    pub fn available_seconds(&self) -> f64 {
        let finalized = self.finalized_duration_ms();
        let open = self.open_fragment_elapsed_ms();
        (finalized + open) as f64 / 1000.0
    }

    pub fn finalized_duration_ms(&self) -> u64 {
        self.entries.iter().map(|e| e.duration_ms).sum()
    }

    fn open_fragment_elapsed_ms(&self) -> u64 {
        let Some(open) = &self.open_fragment else {
            return 0;
        };
        let elapsed = unix_now_ms().saturating_sub(open.opened_unix_ms) as u64;
        elapsed.min(self.chunk_duration_ms)
    }

    pub fn entries(&self) -> &VecDeque<ChunkEntry> {
        &self.entries
    }

    /// Create a mark at the current buffer write position (open fragment preferred).
    pub fn create_mark(&self) -> Result<BufferMark, &'static str> {
        let open = self.open_fragment.as_ref().map(|o| OpenFragmentMarkInput {
            chunk_id: o.chunk_id.clone(),
            opened_unix_ms: o.opened_unix_ms,
        });
        let last_id = self.entries.back().map(|e| e.id.as_str());
        timeline_mark_now(open.as_ref(), last_id)
    }

    pub fn has_markable_fragment(&self) -> bool {
        self.open_fragment.is_some() || !self.entries.is_empty()
    }

    pub fn segments_from_mark(&self, mark: &BufferMark) -> Vec<PathBuf> {
        let mark_idx = parse_chunk_index(&mark.chunk_id);
        let mut paths = Vec::new();
        for e in &self.entries {
            if parse_chunk_index(&e.id) >= mark_idx {
                paths.push(e.path.clone());
            }
        }
        paths
    }

    pub fn segments_from_buffer_mark(&self) -> Vec<PathBuf> {
        self.mark
            .as_ref()
            .map(|m| self.segments_from_mark(m))
            .unwrap_or_default()
    }

    pub fn segments_last_seconds(&self, seconds: u32) -> Vec<PathBuf> {
        let cutoff_ms = self
            .buffer_timeline_now_ms()
            .saturating_sub((seconds as u64) * 1000);
        self.entries
            .iter()
            .filter(|e| e.buffer_offset_ms + e.duration_ms > cutoff_ms)
            .map(|e| e.path.clone())
            .collect()
    }

    fn buffer_timeline_now_ms(&self) -> u64 {
        self.finalized_duration_ms() + self.open_fragment_elapsed_ms()
    }

    pub fn clean_all(&self) {
        if self.buffer_path.exists() {
            let _ = std::fs::remove_dir_all(&self.buffer_path);
        }
        let _ = std::fs::create_dir_all(&self.buffer_path);
    }

    pub fn persist_index(&self) {
        let list: Vec<_> = self.entries.iter().cloned().collect();
        Self::write_index_files(&self.buffer_path, &list, self.mark.as_ref());
    }

    /// Persist index/mark JSON without blocking the GStreamer bus or gRPC handlers.
    pub fn persist_index_background(&self) {
        let buffer_path = self.buffer_path.clone();
        let list: Vec<_> = self.entries.iter().cloned().collect();
        let mark = self.mark.clone();
        std::thread::spawn(move || {
            Self::write_index_files(&buffer_path, &list, mark.as_ref());
        });
    }

    fn write_index_files(
        buffer_path: &Path,
        entries: &[ChunkEntry],
        mark: Option<&BufferMark>,
    ) {
        let index_path = buffer_path.join("index.json");
        if let Ok(json) = serde_json::to_string_pretty(entries) {
            let _ = std::fs::write(index_path, json);
        }
        if let Some(mark) = mark {
            let mark_path = buffer_path.join("mark.json");
            if let Ok(json) = serde_json::to_string_pretty(mark) {
                let _ = std::fs::write(mark_path, json);
            }
        }
    }

    /// Fallback when splitmux bus messages are missed: register finalized chunk files on disk.
    pub fn ingest_closed_fragments_from_disk(&mut self) {
        let Ok(read) = std::fs::read_dir(&self.buffer_path) else {
            return;
        };
        let mut paths: Vec<PathBuf> = read
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("mkv")
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|n| n.starts_with("chunk_"))
            })
            .collect();
        paths.sort();
        for path in paths {
            let id = chunk_id_from_path(&path);
            if self.entries.iter().any(|e| e.id == id) {
                continue;
            }
            self.on_fragment_closed(path, Some(self.chunk_duration_ms));
        }
    }

    pub fn load_from_disk(buffer_path: &Path, max_seconds: u32, chunk_seconds: u32) -> Self {
        let mut index = Self::new(buffer_path.to_path_buf(), max_seconds, chunk_seconds);
        let index_path = buffer_path.join("index.json");
        if let Ok(data) = std::fs::read_to_string(index_path) {
            if let Ok(entries) = serde_json::from_str::<Vec<ChunkEntry>>(&data) {
                for e in entries {
                    if e.path.exists() && file_size(&e.path) > 0 {
                        index.entries.push_back(e);
                    }
                }
            }
        }
        let mark_path = buffer_path.join("mark.json");
        if let Ok(data) = std::fs::read_to_string(mark_path) {
            if let Ok(mark) = serde_json::from_str::<BufferMark>(&data) {
                index.mark = Some(mark);
            }
        }
        index
    }
}

fn unix_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Extract numeric index from `chunk_00042.mkv` → `00042`.
pub fn chunk_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix("chunk_"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "00000".to_string())
}

pub fn parse_chunk_index(chunk_id: &str) -> u32 {
    chunk_id.parse().unwrap_or(0)
}

/// Open fragment snapshot for lock-free mark (gRPC hot path).
#[derive(Debug, Clone)]
pub struct OpenFragmentMarkInput {
    pub chunk_id: String,
    pub opened_unix_ms: i64,
}

/// Single mark algorithm shared by the chunk-index worker and lock-free snapshot path.
pub fn timeline_mark_now(
    open: Option<&OpenFragmentMarkInput>,
    last_finalized_chunk_id: Option<&str>,
) -> Result<BufferMark, &'static str> {
    let unix_ms = unix_now_ms();
    if let Some(open) = open {
        let offset_ms = unix_ms.saturating_sub(open.opened_unix_ms) as u64;
        return Ok(BufferMark {
            chunk_id: open.chunk_id.clone(),
            offset_ms,
            unix_ms,
        });
    }
    if let Some(chunk_id) = last_finalized_chunk_id {
        return Ok(BufferMark {
            chunk_id: chunk_id.to_string(),
            offset_ms: 0,
            unix_ms,
        });
    }
    Err("no buffer fragment available to mark")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_last_seconds_filters_by_timeline() {
        let dir = std::env::temp_dir().join("replay-test-buffer-3");
        let _ = std::fs::create_dir_all(&dir);
        let mut idx = ChunkIndex::new(dir.clone(), 20, 1);
        let p1 = dir.join("chunk_00001.mkv");
        let p2 = dir.join("chunk_00002.mkv");
        std::fs::write(&p1, b"x").ok();
        std::fs::write(&p2, b"x").ok();

        idx.entries.push_back(ChunkEntry {
            id: "00001".into(),
            path: p1.clone(),
            start_unix_ms: 0,
            buffer_offset_ms: 0,
            duration_ms: 1000,
        });
        idx.entries.push_back(ChunkEntry {
            id: "00002".into(),
            path: p2.clone(),
            start_unix_ms: 1000,
            buffer_offset_ms: 1000,
            duration_ms: 1000,
        });

        let segs = idx.segments_last_seconds(1);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0], p2);
    }

    #[test]
    fn segments_from_mark_includes_from_chunk() {
        let dir = std::env::temp_dir().join("replay-test-mark");
        let _ = std::fs::create_dir_all(&dir);
        let mut idx = ChunkIndex::new(dir.clone(), 20, 1);
        let p1 = dir.join("chunk_00001.mkv");
        let p2 = dir.join("chunk_00002.mkv");
        let p3 = dir.join("chunk_00003.mkv");
        for (i, p) in [(1, &p1), (2, &p2), (3, &p3)] {
            std::fs::write(p, b"x").ok();
            idx.entries.push_back(ChunkEntry {
                id: format!("{:05}", i),
                path: p.clone(),
                start_unix_ms: i * 1000,
                buffer_offset_ms: ((i - 1) * 1000) as u64,
                duration_ms: 1000,
            });
        }
        let mark = BufferMark {
            chunk_id: "00002".into(),
            offset_ms: 500,
            unix_ms: 2500,
        };
        let segs = idx.segments_from_mark(&mark);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0], p2);
        assert_eq!(segs[1], p3);
    }

    #[test]
    fn create_mark_uses_open_fragment() {
        let dir = std::env::temp_dir().join("replay-test-open-frag");
        let _ = std::fs::create_dir_all(&dir);
        let mut idx = ChunkIndex::new(dir.clone(), 20, 2);
        let path = dir.join("chunk_00005.mkv");
        idx.on_fragment_opened(path);
        let mark = idx.create_mark().unwrap();
        assert_eq!(mark.chunk_id, "00005");
    }

    #[test]
    fn timeline_mark_now_open_over_last_finalized() {
        let open = OpenFragmentMarkInput {
            chunk_id: "00010".into(),
            opened_unix_ms: unix_now_ms() - 500,
        };
        let mark = timeline_mark_now(Some(&open), Some("00009")).unwrap();
        assert_eq!(mark.chunk_id, "00010");
        assert!(mark.offset_ms >= 400);
    }

    #[test]
    fn timeline_mark_now_falls_back_to_last_finalized() {
        let mark = timeline_mark_now(None, Some("00007")).unwrap();
        assert_eq!(mark.chunk_id, "00007");
        assert_eq!(mark.offset_ms, 0);
    }

    #[test]
    fn timeline_mark_now_rejects_empty_buffer() {
        assert!(timeline_mark_now(None, None).is_err());
    }

    #[test]
    fn create_mark_rejects_empty_index() {
        let dir = std::env::temp_dir().join("replay-test-empty-mark");
        let _ = std::fs::create_dir_all(&dir);
        let idx = ChunkIndex::new(dir, 20, 2);
        assert!(idx.create_mark().is_err());
    }
}
