use std::path::PathBuf;

use replay_core::config::config_dir;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Writable log directory: prefer `/etc/instant-replay/logs` on Pi, else user config / tmp.
pub fn log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("INSTANT_REPLAY_LOG_DIR") {
        let p = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&p);
        return p;
    }

    let etc = config_dir().join("logs");
    if std::fs::create_dir_all(&etc).is_ok() {
        return etc;
    }

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let user = home.join(".config/instant-replay/logs");
    if std::fs::create_dir_all(&user).is_ok() {
        return user;
    }

    let tmp = PathBuf::from("/tmp/instant-replay/logs");
    let _ = std::fs::create_dir_all(&tmp);
    tmp
}

pub fn init() {
    let log_dir = log_dir();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "replay-engine.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(_guard);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,replay_engine=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();
}
