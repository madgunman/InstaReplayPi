use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub input: InputConfig,
    pub output: OutputConfig,
    pub replay: ReplaySettings,
    pub storage: StorageConfig,
    pub hotkeys: HotkeyConfig,
    pub appliance: ApplianceConfig,
    pub operator: OperatorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OperatorConfig {
    pub enabled: bool,
    pub display_id: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// Empty = PIN disabled; long-press on banner still unlocks setup.
    pub setup_pin: String,
    /// Seconds setup stays unlocked after PIN or long-press.
    pub setup_unlock_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub device_id: String,
    pub resolution: String,
    pub fps: u32,
    pub pixel_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub display_id: u32,
    pub fullscreen: bool,
    pub show_status_overlay: bool,
    /// When true and multiple monitors exist, audience HDMI is non-operator display.
    pub auto_display: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplaySettings {
    pub buffer_seconds: u32,
    pub default_replay_seconds: u32,
    pub chunk_seconds: u32,
    pub speed: f64,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub buffer_path: PathBuf,
    pub auto_clean_on_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub mark: String,
    pub replay: String,
    pub replay_last: String,
    pub return_live: String,
    pub clear_mark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApplianceConfig {
    pub enabled: bool,
    pub autostart_live: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            input: InputConfig::default(),
            output: OutputConfig::default(),
            replay: ReplaySettings::default(),
            storage: StorageConfig::default(),
            hotkeys: HotkeyConfig::default(),
            appliance: ApplianceConfig::default(),
            operator: OperatorConfig::default(),
        }
    }
}

impl Default for OperatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_id: 0,
            width: 800,
            height: 480,
            fullscreen: false,
            setup_pin: "0000".to_string(),
            setup_unlock_seconds: 600,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            device_id: "auto".to_string(),
            resolution: "auto".to_string(),
            fps: 0,
            pixel_format: "auto".to_string(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            display_id: 0,
            fullscreen: true,
            show_status_overlay: false,
            auto_display: true,
        }
    }
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self {
            buffer_seconds: 20,
            default_replay_seconds: 10,
            chunk_seconds: 1,
            speed: 0.5,
            mode: "marked".to_string(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            buffer_path: default_buffer_path(),
            auto_clean_on_start: true,
        }
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            mark: "M".to_string(),
            replay: "R".to_string(),
            replay_last: "Space".to_string(),
            return_live: "L".to_string(),
            clear_mark: "C".to_string(),
        }
    }
}

impl Default for ApplianceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart_live: true,
        }
    }
}

pub fn default_buffer_path() -> PathBuf {
    PathBuf::from("/var/lib/instant-replay/buffer")
}

/// Pi appliance: `/etc/instant-replay/config.toml`, then legacy JSON in config dir.
pub fn config_dir() -> PathBuf {
    PathBuf::from("/etc/instant-replay")
}

impl AppConfig {
    pub fn config_path_toml() -> PathBuf {
        config_dir().join("config.toml")
    }

    pub fn config_path_json_legacy() -> PathBuf {
        dirs_home().join(".config/instant-replay/config.json")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let toml_path = Self::config_path_toml();
        if toml_path.exists() {
            let data = std::fs::read_to_string(&toml_path)?;
            return Ok(toml::from_str(&data)?);
        }
        let json_path = Self::config_path_json_legacy();
        if json_path.exists() {
            let data = std::fs::read_to_string(&json_path)?;
            return Ok(serde_json::from_str(&data)?);
        }
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        let data = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path_toml(), data)?;
        Ok(())
    }

    pub fn parse_resolution(&self) -> Option<(u32, u32)> {
        let r = self.input.resolution.trim();
        if r.is_empty() || r.eq_ignore_ascii_case("auto") {
            return None;
        }
        let parts: Vec<_> = r.split('x').collect();
        if parts.len() == 2 {
            Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
        } else {
            None
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/pi"))
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn default_replay_mode_is_marked() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.replay.mode, "marked");
    }

    #[test]
    fn default_appliance_autostart() {
        let cfg = AppConfig::default();
        assert!(cfg.appliance.enabled);
        assert!(cfg.appliance.autostart_live);
    }

    #[test]
    fn default_input_auto_detect() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.input.device_id, "auto");
        assert_eq!(cfg.input.resolution, "auto");
        assert_eq!(cfg.input.fps, 0);
        assert!(cfg.output.auto_display);
    }
}
