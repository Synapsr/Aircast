import { invoke } from "@tauri-apps/api/core";
import type {
  AppMode,
  AudioDevice,
  CartSnapshot,
  MusicSnapshot,
  Preset,
  RelaySource,
  Settings,
  StreamConfig,
  TrackInfo,
} from "@/types";

export const api = {
  listAudioDevices(): Promise<AudioDevice[]> {
    return invoke("list_audio_devices");
  },

  startAudioPreview(deviceId: string): Promise<void> {
    return invoke("start_audio_preview", { deviceId });
  },

  stopAudioPreview(): Promise<void> {
    return invoke("stop_audio_preview");
  },

  startStream(config: StreamConfig): Promise<void> {
    return invoke("start_stream", { config });
  },

  stopStream(): Promise<void> {
    return invoke("stop_stream");
  },

  loadPresets(): Promise<Preset[]> {
    return invoke("load_presets");
  },

  savePreset(name: string, config: StreamConfig): Promise<void> {
    return invoke("save_preset", { name, config });
  },

  deletePreset(name: string): Promise<void> {
    return invoke("delete_preset", { name });
  },

  renamePreset(oldName: string, newName: string): Promise<void> {
    return invoke("rename_preset", { oldName, newName });
  },

  loadSettings(): Promise<Settings> {
    return invoke("load_settings");
  },

  saveSettings(settings: Settings): Promise<void> {
    return invoke("save_settings", { settings });
  },

  loadCurrentConfig(): Promise<StreamConfig | null> {
    return invoke("load_current_config");
  },

  saveCurrentConfig(config: StreamConfig): Promise<void> {
    return invoke("save_current_config", { config });
  },

  // ───── mode ─────
  getMode(): Promise<AppMode> {
    return invoke("get_mode");
  },
  setMode(mode: AppMode): Promise<void> {
    return invoke("set_mode", { mode });
  },

  // ───── mic ─────
  setMicOpen(open: boolean): Promise<void> {
    return invoke("set_mic_open", { open });
  },
  getMicOpen(): Promise<boolean> {
    return invoke("get_mic_open");
  },

  // ───── monitor (local speaker return) ─────
  setMonitorMuted(muted: boolean): Promise<void> {
    return invoke("set_monitor_muted", { muted });
  },
  getMonitorMuted(): Promise<boolean> {
    return invoke("get_monitor_muted");
  },

  // ───── music ─────
  musicEnqueue(paths: string[]): Promise<TrackInfo[]> {
    return invoke("music_enqueue", { paths });
  },
  musicRemove(id: string): Promise<void> {
    return invoke("music_remove", { id });
  },
  musicMove(id: string, delta: number): Promise<void> {
    return invoke("music_move", { id, delta });
  },
  musicPlay(): Promise<void> {
    return invoke("music_play");
  },
  musicPause(): Promise<void> {
    return invoke("music_pause");
  },
  musicStop(): Promise<void> {
    return invoke("music_stop");
  },
  musicNext(): Promise<void> {
    return invoke("music_next");
  },
  musicSnapshot(): Promise<MusicSnapshot> {
    return invoke("music_snapshot");
  },

  // ───── carts ─────
  cartAssign(slot: number, name: string, path: string): Promise<unknown> {
    return invoke("cart_assign", { slot, name, path });
  },
  cartRemove(slot: number): Promise<void> {
    return invoke("cart_remove", { slot });
  },
  cartPlay(slot: number): Promise<void> {
    return invoke("cart_play", { slot });
  },
  cartStop(slot: number): Promise<void> {
    return invoke("cart_stop", { slot });
  },
  cartSnapshot(): Promise<CartSnapshot[]> {
    return invoke("cart_snapshot");
  },

  // ───── external links ─────
  openExternal(url: string): Promise<void> {
    return invoke("open_external", { url });
  },

  // ───── diagnostics ─────
  getDiagnosticBundle(): Promise<string> {
    return invoke("get_diagnostic_bundle");
  },

  // ───── metadata broadcaster ─────
  pushMetadataNow(title: string | null = null): Promise<void> {
    return invoke("push_metadata_now", { title });
  },

  // ───── relay sources ─────
  listRelaySources(): Promise<RelaySource[]> {
    return invoke("list_relay_sources");
  },
  upsertRelaySource(source: RelaySource): Promise<void> {
    return invoke("upsert_relay_source", { source });
  },
  deleteRelaySource(name: string): Promise<void> {
    return invoke("delete_relay_source", { name });
  },
  renameRelaySource(oldName: string, newName: string): Promise<void> {
    return invoke("rename_relay_source", { oldName, newName });
  },
  setActiveRelaySource(name: string | null): Promise<void> {
    return invoke("set_active_relay_source", { name });
  },
  startRelayInput(sourceName: string): Promise<void> {
    return invoke("start_relay_input", { sourceName });
  },
  stopRelayInput(): Promise<void> {
    return invoke("stop_relay_input");
  },
};
