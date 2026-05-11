pub mod store;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::studio::cart::CartSlot;

/// Selects which source feeds the Icecast "now playing" title:
/// - `Auto`   — render `MetadataSettings::template` from current track tags
/// - `Static` — push the literal `MetadataSettings::static_text`
/// - `File`   — poll an external text file at `MetadataSettings::file_path`
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MetadataMode {
    #[default]
    Auto,
    Static,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MetadataSettings {
    /// Master toggle. When false the updater is dormant entirely.
    pub enabled: bool,
    pub mode: MetadataMode,

    // Auto mode
    /// Template with placeholders: {title} {artist} {album} {next_title}
    /// {next_artist} {show} {station}. Empty placeholders are stripped and
    /// runs of whitespace collapsed before sending.
    pub template: String,

    // Static mode
    pub static_text: String,

    // File mode
    pub file_path: Option<PathBuf>,
    pub file_poll_secs: u32,

    // Mic override (applies on top of any mode)
    /// When non-empty AND the mic is open, this template overrides the
    /// computed title until the mic closes.
    pub mic_override: String,

    // Identity used by `{show}` and `{station}` placeholders.
    pub station_name: String,
    pub show_name: String,
}

impl Default for MetadataSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: MetadataMode::Auto,
            template: "{artist} — {title}".into(),
            static_text: String::new(),
            file_path: None,
            file_poll_secs: 5,
            mic_override: String::new(),
            station_name: String::new(),
            show_name: String::new(),
        }
    }
}

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

/// Named upstream stream URL the user can relay to Icecast. The Relay mode
/// shows a picker over the list, then ffmpeg decodes the URL and feeds the
/// existing mixer/encoder pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RelaySource {
    pub name: String,
    /// Anything ffmpeg can open: http(s)://…/stream.mp3, HLS .m3u8, an
    /// Icecast `/listen/...` URL, even a local file path. Validated only
    /// lightly client-side (must be non-empty) — ffmpeg surfaces the real
    /// errors at connect time.
    pub url: String,
}

/// Per-mode visibility toggles. Hidden modes don't appear in the header
/// switch — useful when a school radio only ever uses one mode and the
/// extra picker buttons are clutter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EnabledModes {
    pub simple: bool,
    pub studio: bool,
    pub relay: bool,
}

impl Default for EnabledModes {
    fn default() -> Self {
        Self {
            simple: true,
            studio: true,
            relay: true,
        }
    }
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
    /// Icecast "now playing" broadcaster — see MetadataSettings.
    pub metadata: MetadataSettings,
    /// Upstream stream URLs available to the Relay mode (CRUD).
    pub relay_sources: Vec<RelaySource>,
    /// Currently-selected relay source by name (or None on first run).
    pub active_relay_source: Option<String>,
    /// Which top-level modes are exposed in the header switch.
    pub enabled_modes: EnabledModes,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            reconnect_interval_seconds: 5,
            language: "auto".into(),
            active_preset: None,
            music_volume_when_mic_open: 0.3,
            crossfade_seconds: 3.0,
            metadata: MetadataSettings::default(),
            relay_sources: Vec::new(),
            active_relay_source: None,
            enabled_modes: EnabledModes::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Simple,
    Studio,
    Relay,
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
