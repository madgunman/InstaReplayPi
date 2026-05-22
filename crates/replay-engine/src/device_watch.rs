//! Polls capture device list; notifies when the configured device disappears.

use std::sync::Arc;
use std::time::Duration;

use replay_core::fsm::ReplayState;
use tracing::warn;

use crate::controller::EngineController;
use crate::devices::list_devices;

pub fn spawn_device_watch(controller: Arc<EngineController>) {
    tokio::spawn(async move {
        let mut device_missing = false;
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            if controller.test_mode() {
                continue;
            }
            let cfg = controller.config().await;
            let device_id = cfg.input.device_id;
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
                tracing::info!(device_id = %device_id, "Capture device reconnected");
                controller.signal_restored_notify();
            }
        }
    });
}
