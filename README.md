# Aircast

A minimal, portable desktop app for radio operators that pushes a microphone input to an Icecast server.

> _Screenshot placeholder — drop a PNG in `docs/screenshot.png` once the UI is final._

## What it does

- Pick any audio input the OS exposes (built-in mic, USB interface, virtual cable…).
- Configure your Icecast server in a single modal: host, port, mount, credentials, format, bitrate.
- Save and recall multiple **presets** for different stations or rehearsal/production servers.
- Click **Go Live**. Watch the **VU meter** to confirm levels. Watch the **status badge** for connection health.
- If the connection drops mid-stream, Aircast **auto-reconnects** every _N_ seconds (configurable; off if you set 0).

It's intentionally small. The default "Simple" mode is one device dropdown, one VU bar, one big button.

A future "Studio" mode is planned: music queue, jingle pads (cartoucheur), and auto-ducking when the mic opens. The Rust audio path is structured to receive that cleanly without a rewrite.

## Tech stack

| Layer | Choice |
|---|---|
| Shell | [Tauri 2](https://tauri.app/) |
| UI | React 18 + TypeScript + [Tailwind CSS v4](https://tailwindcss.com/) |
| Audio capture | [`cpal`](https://crates.io/crates/cpal) (Rust) — input device enumeration + PCM capture |
| Encoding + transport | `ffmpeg` subprocess, fed raw `f32le` PCM via stdin, encoding to MP3 (libmp3lame) or AAC (native) and pushing via the `icecast://` protocol |
| Persistence | a single `aircast.json` in the OS app-data directory |

ffmpeg is **bundled as a Tauri sidecar** for distribution builds (LGPL, audio-only — no x264/x265). For `pnpm tauri dev`, Aircast falls back to whatever `ffmpeg` is on your `PATH`.

## Installing (end users)

> Pre-built binaries are not yet published. For now, build from source — see below.

When releases ship, the install will be a single `.dmg` (macOS), `.exe` (Windows) or `.AppImage` / `.deb` (Linux). No admin rights, no separate ffmpeg install.

The first time you select an input device, macOS will ask for microphone permission.

## Building from source

### Requirements

- **Rust** (stable). Install via [rustup](https://rustup.rs/).
- **Node 20+** and **pnpm 10+**.
- An **Icecast** server to push to. For local testing, a [tiny Docker image](https://hub.docker.com/r/infiniteproject/icecast) is the fastest route:
  ```sh
  docker run -p 8000:8000 -e ICECAST_SOURCE_PASSWORD=hackme infiniteproject/icecast
  ```
- For development only, **ffmpeg** on your `PATH`. macOS: `brew install ffmpeg`. Linux: your package manager. Windows: [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) or [BtbN](https://github.com/BtbN/FFmpeg-Builds/releases).
  Distribution builds bundle ffmpeg automatically — see "Building for distribution" below.

### Run in development

```sh
pnpm install
pnpm tauri dev
```

The window opens. Pick your microphone. Click ⚙ **Setup** to enter your Icecast details, save a preset, then close the modal. Click **Go Live**.

### Building for distribution (.dmg / .exe / .AppImage)

This bundles a static LGPL ffmpeg next to the app so end users don't need to install anything.

```sh
pnpm install
pnpm build:bundle      # = pnpm fetch-ffmpeg && pnpm tauri build --config src-tauri/tauri.bundle.conf.json
```

`pnpm fetch-ffmpeg` downloads the appropriate ffmpeg static build for your host platform from [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases) (LGPL variant) and places it at `src-tauri/binaries/ffmpeg-<rust-target-triple>`. The Tauri bundler then includes it in the app artifact.

For multi-platform release builds, run the equivalent steps inside platform-specific CI jobs.

## Architecture

```
┌─────────────────┐                       ┌─────────────────────┐
│ React (Vite)    │ ── tauri::invoke ──→  │ Rust (Tauri 2)      │
│ TS + Tailwind   │ ←── tauri events ──── │  audio (cpal)       │
└─────────────────┘                       │  stream::pipeline   │
                                          │  presets::store     │
                                          └─────────┬───────────┘
                                                    │ stdin
                                                    ▼
                                          ┌─────────────────────┐
                                          │ ffmpeg (subprocess) │
                                          │  -f f32le -i pipe:0 │
                                          │  → libmp3lame/aac   │
                                          │  → icecast://…      │
                                          └─────────────────────┘
```

- **`audio/capture.rs`** — opens a `cpal` stream on the chosen device, hands raw `f32` samples to a consumer callback. Single thread, drop-to-stop semantics.
- **`vu.rs`** — RMS+peak emitter, throttled to ~20 Hz, emits the `vu-meter` event.
- **`stream/pipeline.rs`** — orchestrator. Owns a `CaptureSession` and an `FfmpegProcess`. Handles connect → live → reconnect → idle as a state machine. Emits `stream-status` events.
- **`stream/ffmpeg.rs`** — subprocess management. `tokio::process::Command` spawn, stdin pump (bounded mpsc, drop-on-full to avoid back-pressure freezes), stderr line parsing for status detection (`progress=continue` ⇒ live; auth/refused/timeout strings ⇒ structured error message).
- **`stream/ffmpeg_path.rs`** — locates the ffmpeg binary: bundled sidecar first, system `PATH` fallback.
- **`presets/store.rs`** — atomic-write JSON file at `<app_data_dir>/aircast.json` with `currentConfig`, `presets`, `settings`.

The frontend is split between presentational components (`DeviceSelector`, `VuMeter`, `StatusBadge`, `GoLiveButton`, `ServerForm`, `PresetManager`) and a single `App.tsx` shell that wires hooks (`useCurrentConfig`, `useSettings`, `usePresets`, `useStreamStatus`, `useVuLevel`) to the Rust commands.

## How streaming works under the hood

1. Frontend calls `start_stream(config)`.
2. Rust resolves the device's native sample rate + channel count (cpal `default_input_config`).
3. ffmpeg is spawned with: `-f f32le -ar SR -ac CH -i pipe:0 -codec:a libmp3lame -b:a Nk -content_type audio/mpeg -f mp3 -progress pipe:2 icecast://user:pass@host:port/mount`.
4. A cpal capture session is started; its callback converts each `f32` sample to little-endian bytes and `try_send`s them into a bounded tokio channel feeding ffmpeg's stdin. (If the channel is full, the chunk is dropped — better a glitch than a freeze.)
5. The pipeline polls ffmpeg's `became_live` flag (set by the stderr parser on the first `progress=continue`); on flip, it emits `StreamStatus::Live` to the frontend.
6. On unexpected ffmpeg exit, the pipeline emits `Reconnecting { nextAttemptInMs }` and retries after the configured delay. On user `stop_stream` or with reconnect set to 0, it emits `Idle` and exits cleanly.

## Project layout

```
Aircast/
├── PLAN.md                       Implementation plan & roadmap (delete once stable)
├── README.md
├── LICENSE                       MIT
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind v4 (config in src/styles.css via @theme)
├── index.html
├── scripts/
│   └── fetch-ffmpeg.mjs          Downloads LGPL ffmpeg for the host platform
├── public/
├── src/                          Frontend (React + TS)
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles.css
│   ├── types.ts
│   ├── lib/api.ts
│   ├── hooks/
│   │   ├── useStreamStatus.ts
│   │   ├── useCurrentConfig.ts
│   │   ├── useSettings.ts
│   │   ├── usePresets.ts
│   │   └── useVuLevel.ts
│   └── components/
│       ├── SimpleMode.tsx
│       ├── SettingsModal.tsx
│       ├── DeviceSelector.tsx
│       ├── ServerForm.tsx
│       ├── PresetManager.tsx
│       ├── StatusBadge.tsx
│       ├── GoLiveButton.tsx
│       └── VuMeter.tsx
└── src-tauri/                    Backend (Rust)
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── tauri.bundle.conf.json    Override that adds the ffmpeg sidecar at bundle time
    ├── Info.plist                Adds NSMicrophoneUsageDescription on macOS
    ├── binaries/                 (gitignored) where fetch-ffmpeg places the sidecar
    ├── icons/
    ├── capabilities/default.json
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── commands.rs
        ├── error.rs
        ├── state.rs
        ├── vu.rs
        ├── audio/
        │   ├── mod.rs
        │   ├── devices.rs
        │   └── capture.rs
        ├── presets/
        │   ├── mod.rs
        │   └── store.rs
        └── stream/
            ├── mod.rs
            ├── pipeline.rs
            ├── ffmpeg.rs
            ├── ffmpeg_path.rs
            └── status.rs
```

## Contributing

PRs welcome. Before opening one:

1. Run `cargo fmt` in `src-tauri/` and `pnpm build` at the repo root — both must succeed without warnings.
2. Test the change in `pnpm tauri dev` against a real Icecast (the Docker one-liner above works).
3. Keep `PLAN.md` honest if your change shifts the roadmap.

If you're adding to **Studio mode**, prefer creating new modules (`music/`, `cartoucheur/`, `mixer/`) rather than expanding `stream/pipeline.rs`. Simple mode must stay tiny.

## Licensing

- **Aircast source:** MIT (see [`LICENSE`](LICENSE)).
- **Bundled ffmpeg:** static LGPL build from BtbN, downloaded at packaging time. Aircast spawns ffmpeg as a separate process via stdin/stdout — under the FSF's "mere aggregation" interpretation, this does not propagate ffmpeg's license to Aircast's source. Distribution must include ffmpeg's license alongside (the LGPL build's `LICENSE.txt` is preserved in `src-tauri/binaries/` after `fetch-ffmpeg`).
- **Tauri, React, cpal**, etc.: see each crate/package's own license.

## Acknowledgements

The Studio-mode roadmap takes inspiration from [MyRadiomatisme](https://www.radiomatisme.fr/) — credit to its design for the layout primitives (now-playing + playlist queue + cartoucheur + mic toggle).
