//! Shared control facade for hotkeys, HTTP, and future GPIO.

use std::sync::Arc;

use anyhow::Result;
use replay_core::config::AppConfig;
use replay_core::types::{DisplayInfo, VideoDevice, VideoFormat};

use crate::controller::{Diagnostics, EngineController, StatusSnapshot};
use crate::devices::{self, CaptureDevice};

#[derive(Clone)]
pub struct ControlApi {
    inner: Arc<EngineController>,
}

impl ControlApi {
    pub fn new(inner: Arc<EngineController>) -> Self {
        Self { inner }
    }

    pub fn controller(&self) -> &Arc<EngineController> {
        &self.inner
    }

    pub async fn engine_ready(&self) -> bool {
        self.inner.engine_ready().await
    }

    pub async fn get_config(&self) -> AppConfig {
        self.inner.config().await
    }

    pub async fn set_config(&self, config: AppConfig) -> Result<()> {
        self.inner.set_config(config).await
    }

    pub fn list_devices(&self) -> Vec<CaptureDevice> {
        devices::list_devices(self.inner.test_mode())
    }

    pub fn list_devices_json(&self) -> Vec<VideoDevice> {
        devices::to_video_devices(&self.list_devices())
    }

    pub fn list_formats(&self, device_id: &str) -> Vec<VideoFormat> {
        devices::list_formats(device_id)
    }

    pub fn list_displays(&self) -> Vec<DisplayInfo> {
        self.inner.list_displays()
    }

    pub async fn start_live(
        &self,
        device_id: String,
        width: u32,
        height: u32,
        fps: u32,
        pixel_format: String,
        display_id: u32,
        fullscreen: bool,
    ) -> Result<()> {
        self.inner
            .start_live(device_id, width, height, fps, pixel_format, display_id, fullscreen)
            .await
    }

    pub async fn stop(&self) -> Result<()> {
        self.inner.stop().await
    }

    pub async fn mark(&self) -> Result<i64> {
        self.inner.mark().await
    }

    pub async fn replay(&self) -> Result<()> {
        self.inner.replay_from_mark_or_last().await
    }

    pub async fn replay_last(&self, seconds: u32) -> Result<()> {
        self.inner.replay_last(seconds).await
    }

    pub async fn return_live(&self) -> Result<()> {
        self.inner.return_live().await
    }

    pub async fn clear_mark(&self) -> Result<()> {
        self.inner.clear_mark().await
    }

    pub async fn status(&self) -> StatusSnapshot {
        self.inner.status_snapshot().await
    }

    pub async fn diagnostics(&self) -> Diagnostics {
        self.inner.get_diagnostics().await
    }

    pub fn subscribe_status(
        &self,
    ) -> tokio::sync::broadcast::Receiver<StatusSnapshot> {
        self.inner.subscribe_status()
    }
}
