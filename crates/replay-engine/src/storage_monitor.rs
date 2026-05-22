use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::warn;

const WARN_FREE_BYTES: u64 = 500 * 1024 * 1024;
const FULL_FREE_BYTES: u64 = 100 * 1024 * 1024;
const SLOW_WRITE_THRESHOLD_MS: u128 = 2000;
const PROBE_SIZE_BYTES: usize = 1024 * 1024;

pub struct StorageMonitor {
    disk_warning: Arc<AtomicBool>,
    disk_full: Arc<AtomicBool>,
    slow_disk: Arc<AtomicBool>,
}

impl StorageMonitor {
    pub fn spawn(buffer_path: impl AsRef<Path>) -> Self {
        let disk_warning = Arc::new(AtomicBool::new(false));
        let disk_full = Arc::new(AtomicBool::new(false));
        let slow_disk = Arc::new(AtomicBool::new(false));
        let path = buffer_path.as_ref().to_path_buf();
        let warn = disk_warning.clone();
        let full = disk_full.clone();
        let slow = slow_disk.clone();

        std::thread::spawn(move || {
            let mut write_probe_counter = 0u32;
            loop {
                if let Ok(space) = fs2::available_space(&path) {
                    full.store(space < FULL_FREE_BYTES, Ordering::Relaxed);
                    warn.store(
                        space < WARN_FREE_BYTES || slow.load(Ordering::Relaxed),
                        Ordering::Relaxed,
                    );
                    if full.load(Ordering::Relaxed) {
                        warn!(
                            free_mb = space / (1024 * 1024),
                            "Buffer disk nearly full"
                        );
                    }
                }

                write_probe_counter = write_probe_counter.wrapping_add(1);
                if write_probe_counter % 6 == 0 {
                    if let Some(latency_ms) = probe_write_latency(&path) {
                        let is_slow = latency_ms > SLOW_WRITE_THRESHOLD_MS;
                        slow.store(is_slow, Ordering::Relaxed);
                        if is_slow {
                            warn!(latency_ms, "Slow buffer disk writes");
                        }
                    }
                }

                std::thread::sleep(Duration::from_secs(5));
            }
        });

        Self {
            disk_warning,
            disk_full,
            slow_disk,
        }
    }

    pub fn disk_warning(&self) -> bool {
        self.disk_warning.load(Ordering::Relaxed)
    }

    pub fn disk_full(&self) -> bool {
        self.disk_full.load(Ordering::Relaxed)
    }

    pub fn slow_disk(&self) -> bool {
        self.slow_disk.load(Ordering::Relaxed)
    }
}

fn probe_write_latency(buffer_path: &Path) -> Option<u128> {
    let probe_path = buffer_path.join(".write_probe");
    std::fs::create_dir_all(buffer_path).ok()?;
    let data = vec![0u8; PROBE_SIZE_BYTES];
    let start = Instant::now();
    std::fs::write(&probe_path, &data).ok()?;
    let elapsed = start.elapsed().as_millis();
    let _ = std::fs::remove_file(&probe_path);
    Some(elapsed)
}
