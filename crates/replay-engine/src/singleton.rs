//! Exclusive process lock so only one replay-engine runs per Pi.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

const LOCK_DIR: &str = "/run/instant-replay";
const LOCK_FILE: &str = "/run/instant-replay/replay-engine.lock";

pub const ALREADY_RUNNING_MSG: &str =
    "replay-engine already running (stop: sudo systemctl stop replay-engine)";

/// Held for process lifetime; lock released on drop.
pub struct EngineLock {
    _file: File,
}

#[derive(Debug)]
pub enum LockError {
    AlreadyRunning,
    Io(std::io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AlreadyRunning => write!(f, "{ALREADY_RUNNING_MSG}"),
            LockError::Io(e) => write!(f, "engine lock: {e}"),
        }
    }
}

impl std::error::Error for LockError {}

impl EngineLock {
    /// Acquire an exclusive lock. Fails if another instance holds it.
    pub fn acquire() -> Result<Self, LockError> {
        std::fs::create_dir_all(LOCK_DIR).map_err(LockError::Io)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(LOCK_FILE)
            .map_err(LockError::Io)?;
        fs2::FileExt::try_lock_exclusive(&file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                LockError::AlreadyRunning
            } else {
                LockError::Io(e)
            }
        })?;
        let mut file = file;
        let _ = file.set_len(0);
        let _ = writeln!(file, "{}", std::process::id());
        Ok(Self { _file: file })
    }

    pub fn lock_path() -> &'static Path {
        Path::new(LOCK_FILE)
    }
}
