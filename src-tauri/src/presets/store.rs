use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::presets::{Mode, PersistedData, Preset, RelaySource, Settings, StreamConfig};
use crate::studio::cart::CartSlot;

pub struct PresetStore {
    path: PathBuf,
    data: Mutex<PersistedData>,
}

impl PresetStore {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::Preset(format!("app_data_dir: {e}")))?;
        std::fs::create_dir_all(&dir)?;
        Self::open(dir.join("aircast.json"))
    }

    /// Test-friendly constructor that loads (or creates) the JSON store at an
    /// explicit path. Callers outside tests should use `new(&AppHandle)` so
    /// the path resolves to the right OS-specific app-data directory.
    pub fn open(path: PathBuf) -> AppResult<Self> {
        let data = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            serde_json::from_str::<PersistedData>(&raw).unwrap_or_default()
        } else {
            PersistedData::default()
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub fn list_presets(&self) -> Vec<Preset> {
        self.data.lock().unwrap().presets.clone()
    }

    pub fn upsert_preset(&self, name: String, config: StreamConfig) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        if let Some(existing) = data.presets.iter_mut().find(|p| p.name == name) {
            existing.config = config;
        } else {
            data.presets.push(Preset { name, config });
        }
        write(&self.path, &data)
    }

    pub fn delete_preset(&self, name: &str) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.presets.retain(|p| p.name != name);
        write(&self.path, &data)
    }

    pub fn rename_preset(&self, old_name: &str, new_name: &str) -> AppResult<()> {
        if old_name == new_name {
            return Ok(());
        }
        let mut data = self.data.lock().unwrap();
        if data.presets.iter().any(|p| p.name == new_name) {
            return Err(AppError::Preset(format!(
                "preset '{new_name}' already exists"
            )));
        }
        let idx = data
            .presets
            .iter()
            .position(|p| p.name == old_name)
            .ok_or_else(|| AppError::Preset(format!("preset '{old_name}' not found")))?;
        data.presets[idx].name = new_name.to_string();
        if data.settings.active_preset.as_deref() == Some(old_name) {
            data.settings.active_preset = Some(new_name.to_string());
        }
        write(&self.path, &data)
    }

    pub fn settings(&self) -> Settings {
        self.data.lock().unwrap().settings.clone()
    }

    pub fn save_settings(&self, settings: Settings) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.settings = settings;
        write(&self.path, &data)
    }

    pub fn current_config(&self) -> Option<StreamConfig> {
        self.data.lock().unwrap().current_config.clone()
    }

    pub fn save_current_config(&self, config: StreamConfig) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.current_config = Some(config);
        write(&self.path, &data)
    }

    pub fn mode(&self) -> Mode {
        self.data.lock().unwrap().mode
    }

    pub fn save_mode(&self, mode: Mode) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.mode = mode;
        write(&self.path, &data)
    }

    pub fn carts(&self) -> Vec<CartSlot> {
        self.data.lock().unwrap().carts.clone()
    }

    pub fn save_carts(&self, carts: Vec<CartSlot>) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.carts = carts;
        write(&self.path, &data)
    }

    // ── Relay sources ───────────────────────────────────────────────────────

    pub fn relay_sources(&self) -> Vec<RelaySource> {
        self.data.lock().unwrap().settings.relay_sources.clone()
    }

    pub fn upsert_relay_source(&self, source: RelaySource) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        let trimmed_name = source.name.trim().to_string();
        if trimmed_name.is_empty() {
            return Err(AppError::Preset("relay source name cannot be empty".into()));
        }
        let trimmed_url = source.url.trim().to_string();
        if trimmed_url.is_empty() {
            return Err(AppError::Preset("relay source URL cannot be empty".into()));
        }
        let candidate = RelaySource {
            name: trimmed_name.clone(),
            url: trimmed_url,
        };
        let sources = &mut data.settings.relay_sources;
        if let Some(existing) = sources.iter_mut().find(|s| s.name == trimmed_name) {
            *existing = candidate;
        } else {
            sources.push(candidate);
        }
        write(&self.path, &data)
    }

    pub fn delete_relay_source(&self, name: &str) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.settings.relay_sources.retain(|s| s.name != name);
        if data.settings.active_relay_source.as_deref() == Some(name) {
            data.settings.active_relay_source = None;
        }
        write(&self.path, &data)
    }

    pub fn rename_relay_source(&self, old_name: &str, new_name: &str) -> AppResult<()> {
        if old_name == new_name {
            return Ok(());
        }
        let new_trim = new_name.trim().to_string();
        if new_trim.is_empty() {
            return Err(AppError::Preset("relay source name cannot be empty".into()));
        }
        let mut data = self.data.lock().unwrap();
        if data
            .settings
            .relay_sources
            .iter()
            .any(|s| s.name == new_trim)
        {
            return Err(AppError::Preset(format!(
                "relay source '{new_trim}' already exists"
            )));
        }
        let idx = data
            .settings
            .relay_sources
            .iter()
            .position(|s| s.name == old_name)
            .ok_or_else(|| AppError::Preset(format!("relay source '{old_name}' not found")))?;
        data.settings.relay_sources[idx].name = new_trim.clone();
        if data.settings.active_relay_source.as_deref() == Some(old_name) {
            data.settings.active_relay_source = Some(new_trim);
        }
        write(&self.path, &data)
    }

    pub fn set_active_relay_source(&self, name: Option<String>) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        if let Some(n) = &name {
            if !data.settings.relay_sources.iter().any(|s| &s.name == n) {
                return Err(AppError::Preset(format!("relay source '{n}' not found")));
            }
        }
        data.settings.active_relay_source = name;
        write(&self.path, &data)
    }
}

fn write(path: &std::path::Path, data: &PersistedData) -> AppResult<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{Mode, StreamFormat};
    use tempfile::TempDir;

    fn store_in(dir: &TempDir) -> PresetStore {
        PresetStore::open(dir.path().join("aircast.json")).unwrap()
    }

    fn sample_config() -> StreamConfig {
        StreamConfig {
            device_id: "dev".into(),
            host: "stream.example.com".into(),
            port: 8000,
            mount: "/live.mp3".into(),
            username: "source".into(),
            password: "secret".into(),
            bitrate: 128,
            format: StreamFormat::Mp3,
        }
    }

    #[test]
    fn fresh_store_has_default_data() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        assert!(s.list_presets().is_empty());
        assert_eq!(s.settings().reconnect_interval_seconds, 5);
        assert_eq!(s.settings().language, "auto");
        assert!(s.current_config().is_none());
        assert_eq!(s.mode(), Mode::Simple);
        assert!(s.carts().is_empty());
    }

    #[test]
    fn upsert_preset_adds_new_then_updates_existing() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        let mut cfg = sample_config();
        s.upsert_preset("Prod".into(), cfg.clone()).unwrap();
        assert_eq!(s.list_presets().len(), 1);
        // Update the same name
        cfg.host = "other.example.com".into();
        s.upsert_preset("Prod".into(), cfg).unwrap();
        let presets = s.list_presets();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].config.host, "other.example.com");
    }

    #[test]
    fn delete_preset_removes_entry() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.upsert_preset("A".into(), sample_config()).unwrap();
        s.upsert_preset("B".into(), sample_config()).unwrap();
        s.delete_preset("A").unwrap();
        let names: Vec<String> = s.list_presets().iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["B"]);
    }

    #[test]
    fn rename_preset_updates_name_and_active_pointer() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.upsert_preset("Old".into(), sample_config()).unwrap();
        let mut settings = s.settings();
        settings.active_preset = Some("Old".into());
        s.save_settings(settings).unwrap();

        s.rename_preset("Old", "New").unwrap();
        let names: Vec<String> = s.list_presets().iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["New"]);
        assert_eq!(s.settings().active_preset, Some("New".into()));
    }

    #[test]
    fn rename_preset_to_existing_name_is_rejected() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.upsert_preset("A".into(), sample_config()).unwrap();
        s.upsert_preset("B".into(), sample_config()).unwrap();
        let result = s.rename_preset("A", "B");
        assert!(result.is_err());
    }

    #[test]
    fn rename_preset_to_same_name_is_noop() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.upsert_preset("A".into(), sample_config()).unwrap();
        s.rename_preset("A", "A").unwrap();
        assert_eq!(s.list_presets()[0].name, "A");
    }

    #[test]
    fn settings_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        let new_settings = Settings {
            reconnect_interval_seconds: 12,
            language: "fr".into(),
            active_preset: Some("Prod".into()),
            music_volume_when_mic_open: 0.5,
            crossfade_seconds: 4.0,
            metadata: Default::default(),
            relay_sources: Vec::new(),
            active_relay_source: None,
            enabled_modes: Default::default(),
        };
        s.save_settings(new_settings.clone()).unwrap();
        let loaded = s.settings();
        assert_eq!(loaded.reconnect_interval_seconds, 12);
        assert_eq!(loaded.language, "fr");
        assert_eq!(loaded.active_preset, Some("Prod".into()));
    }

    #[test]
    fn current_config_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.save_current_config(sample_config()).unwrap();
        let loaded = s.current_config().unwrap();
        assert_eq!(loaded.host, "stream.example.com");
    }

    #[test]
    fn mode_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        s.save_mode(Mode::Studio).unwrap();
        assert_eq!(s.mode(), Mode::Studio);
    }

    #[test]
    fn carts_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store_in(&dir);
        let carts = vec![
            CartSlot {
                slot: 1,
                name: "Jingle".into(),
                path: "/tmp/jingle.mp3".into(),
                duration_secs: 2.5,
            },
            CartSlot {
                slot: 5,
                name: "Outro".into(),
                path: "/tmp/outro.mp3".into(),
                duration_secs: 8.0,
            },
        ];
        s.save_carts(carts.clone()).unwrap();
        let loaded = s.carts();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Jingle");
        assert_eq!(loaded[1].slot, 5);
    }

    #[test]
    fn data_persists_across_store_reopen() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("aircast.json");
        {
            let s = PresetStore::open(path.clone()).unwrap();
            s.upsert_preset("Prod".into(), sample_config()).unwrap();
            s.save_mode(Mode::Studio).unwrap();
        }
        // Reopen and verify
        let s = PresetStore::open(path).unwrap();
        assert_eq!(s.list_presets().len(), 1);
        assert_eq!(s.list_presets()[0].name, "Prod");
        assert_eq!(s.mode(), Mode::Studio);
    }

    #[test]
    fn corrupt_json_falls_back_to_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("aircast.json");
        std::fs::write(&path, "this is not json {{{").unwrap();
        let s = PresetStore::open(path).unwrap();
        // Should not panic — falls back to default
        assert!(s.list_presets().is_empty());
        assert_eq!(s.mode(), Mode::Simple);
    }

    #[test]
    fn write_is_atomic_via_rename() {
        // Verify the .tmp file is removed after a successful write.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("aircast.json");
        let s = PresetStore::open(path.clone()).unwrap();
        s.upsert_preset("A".into(), sample_config()).unwrap();
        let tmp = path.with_extension("json.tmp");
        assert!(!tmp.exists(), ".tmp file should be renamed away");
        assert!(path.exists(), "final file should exist");
    }
}
