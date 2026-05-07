pub mod store;

use serde::{Deserialize, Serialize};

use crate::studio::cart::CartSlot;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StreamFormat {
    #[default]
    Mp3,
    Aac,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamConfig {
    pub device_id: String,
    pub host: String,
    pub port: u16,
    pub mount: String,
    pub username: String,
    pub password: String,
    pub bitrate: u32,
    pub format: StreamFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: String,
    pub config: StreamConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub reconnect_interval_seconds: u64,
    pub language: String, // "auto" | "en" | "fr"
    pub active_preset: Option<String>,
    /// 0.0 (fully muted) .. 1.0 (full volume) for music while the mic is open.
    /// Default 0.3 (music ducks to 30% of its level).
    pub music_volume_when_mic_open: f32,
    /// Crossfade duration when skipping to the next track, in seconds.
    /// 0 disables the crossfade. Default 3 s.
    pub crossfade_seconds: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            reconnect_interval_seconds: 5,
            language: "auto".into(),
            active_preset: None,
            music_volume_when_mic_open: 0.3,
            crossfade_seconds: 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Simple,
    Studio,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PersistedData {
    pub settings: Settings,
    pub current_config: Option<StreamConfig>,
    pub presets: Vec<Preset>,
    pub mode: Mode,
    pub carts: Vec<CartSlot>,
}
