//! Single-threaded owner of `ChunkIndex` — GStreamer bus posts fragments; mark uses a lock-free snapshot.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use replay_core::buffer::{
    BufferMark, ChunkIndex, OpenFragmentMarkInput, chunk_id_from_path, timeline_mark_now,
};
use tracing::debug;

#[derive(Clone)]
struct OpenFragmentSnap {
    chunk_id: String,
    opened_unix_ms: i64,
}

#[derive(Clone, Default)]
struct LastFinalizedSnap {
    chunk_id: String,
}

enum FragmentMsg {
    Opened(PathBuf),
    Closed(PathBuf, Option<u64>),
}

enum ControlMsg {
    WithIndex {
        f: Box<dyn FnOnce(&mut ChunkIndex) + Send>,
    },
}

/// Handle to the chunk-index worker (cloneable, cheap).
#[derive(Clone)]
pub struct ChunkIndexHandle {
    fragment_tx: Sender<FragmentMsg>,
    control_tx: Sender<ControlMsg>,
    /// Updated on the GStreamer bus thread when a fragment opens (no worker queue).
    open_snapshot: Arc<RwLock<Option<OpenFragmentSnap>>>,
    /// Updated on the worker when a fragment closes (for lock-free mark fallback).
    last_finalized: Arc<RwLock<Option<LastFinalizedSnap>>>,
}

pub struct ChunkIndexWorker {
    handle: ChunkIndexHandle,
    _thread: JoinHandle<()>,
}

impl ChunkIndexWorker {
    pub fn spawn(
        index: ChunkIndex,
        buffer_secs_cache: Arc<AtomicU64>,
    ) -> Self {
        let (fragment_tx, fragment_rx) = mpsc::channel();
        let (control_tx, control_rx) = mpsc::channel();
        let open_snapshot = Arc::new(RwLock::new(None));
        let last_finalized = Arc::new(RwLock::new(None));
        let handle = ChunkIndexHandle {
            fragment_tx,
            control_tx,
            open_snapshot: open_snapshot.clone(),
            last_finalized: last_finalized.clone(),
        };
        let thread = std::thread::Builder::new()
            .name("chunk-index".into())
            .spawn(move || {
                run_worker(
                    fragment_rx,
                    control_rx,
                    index,
                    buffer_secs_cache,
                    last_finalized,
                )
            })
            .expect("spawn chunk-index worker");
        Self {
            handle,
            _thread: thread,
        }
    }

    pub fn handle(&self) -> ChunkIndexHandle {
        self.handle.clone()
    }
}

impl ChunkIndexHandle {
    /// Called from the GStreamer bus before queuing fragment-opened to the worker.
    pub fn note_fragment_opened(&self, path: &PathBuf) {
        let opened_unix_ms = unix_now_ms();
        let chunk_id = chunk_id_from_path(path);
        if let Ok(mut snap) = self.open_snapshot.write() {
            *snap = Some(OpenFragmentSnap {
                chunk_id,
                opened_unix_ms,
            });
        }
    }

    /// Fast mark for gRPC — uses the same timeline algorithm as `ChunkIndex::create_mark`.
    pub fn snapshot_mark(&self) -> Result<BufferMark, &'static str> {
        let open = self
            .open_snapshot
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|o| OpenFragmentMarkInput {
                chunk_id: o.chunk_id.clone(),
                opened_unix_ms: o.opened_unix_ms,
            }));
        let last_id = self
            .last_finalized
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|l| l.chunk_id.clone()));
        timeline_mark_now(open.as_ref(), last_id.as_deref())
    }

    /// Apply mark on the worker asynchronously (replay reads index there).
    pub fn post_set_mark(&self, mark: BufferMark) {
        let _ = self.control_tx.send(ControlMsg::WithIndex {
            f: Box::new(move |idx| {
                idx.set_mark(mark);
                idx.persist_index_background();
            }),
        });
    }

    pub fn post_fragment_opened(&self, path: PathBuf) {
        let _ = self.fragment_tx.send(FragmentMsg::Opened(path));
    }

    pub fn post_fragment_closed(&self, path: PathBuf, duration_ms: Option<u64>) {
        let _ = self
            .fragment_tx
            .send(FragmentMsg::Closed(path, duration_ms));
    }

    pub fn with_index_blocking<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut ChunkIndex) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let (done_tx, done_rx) = mpsc::channel();
        self.control_tx
            .send(ControlMsg::WithIndex {
                f: Box::new(move |idx| {
                    let _ = done_tx.send(f(idx));
                }),
            })
            .map_err(|_| anyhow::anyhow!("chunk index worker stopped"))?;
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| anyhow::anyhow!("chunk index op timed out"))?
    }

    pub async fn with_index<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut ChunkIndex) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = self.clone();
        std::thread::spawn(move || {
            let _ = tx.send(handle.with_index_blocking(f));
        });
        rx.await
            .map_err(|_| anyhow::anyhow!("chunk index op dropped result"))?
    }
}

fn unix_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn run_worker(
    fragment_rx: Receiver<FragmentMsg>,
    control_rx: Receiver<ControlMsg>,
    mut index: ChunkIndex,
    buffer_secs_cache: Arc<AtomicU64>,
    last_finalized: Arc<RwLock<Option<LastFinalizedSnap>>>,
) {
    if let Some(last) = index.entries().back() {
        if let Ok(mut snap) = last_finalized.write() {
            *snap = Some(LastFinalizedSnap {
                chunk_id: last.id.clone(),
            });
        }
    }

    fn sync_last_finalized(index: &ChunkIndex, last_finalized: &Arc<RwLock<Option<LastFinalizedSnap>>>) {
        if let Some(last) = index.entries().back() {
            if let Ok(mut snap) = last_finalized.write() {
                *snap = Some(LastFinalizedSnap {
                    chunk_id: last.id.clone(),
                });
            }
        }
    }
    loop {
        while let Ok(msg) = control_rx.try_recv() {
            if let ControlMsg::WithIndex { f } = msg {
                f(&mut index);
            }
        }

        match fragment_rx.try_recv() {
            Ok(FragmentMsg::Opened(path)) => {
                index.on_fragment_opened(path);
            }
            Ok(FragmentMsg::Closed(path, duration_ms)) => {
                index.on_fragment_closed(path.clone(), duration_ms);
                sync_last_finalized(&index, &last_finalized);
                let secs = index.available_seconds();
                buffer_secs_cache.store(secs.to_bits(), Ordering::Relaxed);
                debug!(
                    file = %path.display(),
                    seconds = secs,
                    "Fragment closed and registered"
                );
            }
            Err(mpsc::TryRecvError::Empty) => {
                match control_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(ControlMsg::WithIndex { f }) => f(&mut index),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                while let Ok(ControlMsg::WithIndex { f }) = control_rx.try_recv() {
                    f(&mut index);
                }
                break;
            }
        }
    }
}
