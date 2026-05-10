export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export type StreamFormat = "mp3" | "aac";

export type Bitrate = 64 | 128 | 192 | 320;

export interface StreamConfig {
  deviceId: string;
  host: string;
  port: number;
  mount: string;
  username: string;
  password: string;
  bitrate: Bitrate;
  format: StreamFormat;
}

export interface Preset {
  name: string;
  config: StreamConfig;
}

export type MetadataMode = "auto" | "static" | "file";

export interface MetadataSettings {
  enabled: boolean;
  mode: MetadataMode;
  template: string;
  staticText: string;
  filePath: string | null;
  filePollSecs: number;
  micOverride: string;
  stationName: string;
  showName: string;
}

export interface Settings {
  reconnectIntervalSeconds: number;
  language: string; // "auto" | "en" | "fr"
  activePreset?: string | null;
  musicVolumeWhenMicOpen: number; // 0.0 (silent) .. 1.0 (full)
  crossfadeSeconds: number; // 0 .. 30 — fade between music tracks on Next
  metadata: MetadataSettings;
}

export type StreamStatus =
  | { kind: "idle" }
  | { kind: "connecting" }
  | { kind: "live" }
  | { kind: "reconnecting"; nextAttemptInMs: number }
  | { kind: "error"; message: string; details?: string };

export type AppMode = "simple" | "studio";

export interface TrackInfo {
  id: string;
  path: string;
  title: string;
  artist?: string | null;
  album?: string | null;
  titleFromTag?: boolean;
  durationSecs: number | null;
}

export type PlayerState = "stopped" | "playing" | "paused";

export interface CurrentTrackSnapshot {
  info: TrackInfo;
  elapsedSecs: number;
  durationSecs: number | null;
}

export interface MusicSnapshot {
  state: PlayerState;
  queue: TrackInfo[];
  current: CurrentTrackSnapshot | null;
}

export interface CartSnapshot {
  slot: number;
  name: string;
  durationSecs: number;
  elapsedSecs: number;
  playing: boolean;
}

export const DEFAULT_CONFIG: Omit<StreamConfig, "deviceId"> = {
  host: "localhost",
  port: 8000,
  mount: "/aircast.mp3",
  username: "source",
  password: "",
  bitrate: 128,
  format: "mp3",
};
