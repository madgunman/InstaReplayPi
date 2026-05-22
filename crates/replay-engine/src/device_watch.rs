//! Polls capture devices; signal loss while live, autostart when idle and hardware appears.

use std::sync::Arc;
use std::time::Duration;

use replay_core::fsm::ReplayState;
use tracing::{info, warn};

use crate::capture_select;
use crate::controller::EngineController;
use crate::control_api::ControlApi;
use crate::devices::{list_devices, short_operator_error};

pub fn spawn_device_watch(controller: Arc<EngineController>, test_mode: bool) {
    let api = ControlApi::new(controller.clone());
    tokio::spawn(async move {
        let mut device_missing = false;
        let mut backoff_secs: u64 = 5;
        let mut last_error_key = String::new();
        let mut last_autostart_attempt = std::time::Instant::now() - Duration::from_secs(120);
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            if test_mode {
                continue;
            }

            let cfg = controller.config().await;
            let device_id = cfg.input.device_id.clone();

            if !controller.capture_running() {
                device_missing = false;
                let state = controller.fsm_state().await;
                if matches!(
                    state,
                    ReplayState::Starting
                        | ReplayState::NoSignal
                        | ReplayState::ErrorRecovery
                ) {
                    if controller.live_start_in_progress() {
                        continue;
                    }
                    let elapsed = last_autostart_attempt.elapsed();
                    if elapsed < Duration::from_secs(backoff_secs) {
                        continue;
                    }
                    if capture_select::discover_capture_devices().is_empty() {
                        last_autostart_attempt = std::time::Instant::now();
                        continue;
                    }
                    last_autostart_attempt = std::time::Instant::now();
                    info!("Capture device available — attempting autostart");
                    match api.start_live_from_config(&cfg).await {
                        Ok(()) => {
                            backoff_secs = 5;
                            last_error_key.clear();
                        }
                        Err(e) => {
                            let key = short_operator_error(&e.to_string());
                            warn!(error = %key, "Hotplug autostart failed");
                            if key == last_error_key {
                                backoff_secs = (backoff_secs * 2).min(60);
                            } else {
                                last_error_key = key;
                                backoff_secs = 5;
                            }
                        }
                    }
                }
                continue;
            }

            if device_id.is_empty() || device_id == "test" {
                continue;
            }
            let state = controller.fsm_state().await;
            if !matches!(
                state,
                ReplayState::Live | ReplayState::Marked | ReplayState::Replaying
            ) {
                device_missing = false;
                continue;
            }
            let devices = list_devices(false);
            let present = devices.iter().any(|d| d.id == device_id);
            if !present && !device_missing {
                device_missing = true;
                warn!(device_id = %device_id, "Capture device disconnected");
                controller.signal_lost_notify();
            } else if present && device_missing {
                device_missing = false;
                info!(device_id = %device_id, "Capture device reconnected");
                controller.signal_restored_notify();
            }
        }
    });
}
