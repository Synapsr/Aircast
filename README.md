<div align="right">

🌐 **Language**: [Français](README.fr.md) | **English**

</div>

<div align="center">

# 🎙️ Aircast

### The Open-Source Desktop App for Radio Streaming

**Stop paying for proprietary streaming software. Own your radio.**

[![Latest Release](https://img.shields.io/github/v/release/Synapsr/Aircast?style=for-the-badge&logo=github&label=Release)](https://github.com/Synapsr/Aircast/releases/latest)
[![GitHub Stars](https://img.shields.io/github/stars/Synapsr/Aircast?style=for-the-badge&logo=github)](https://github.com/Synapsr/Aircast)
[![License](https://img.shields.io/badge/License-GPL_3.0-blue?style=for-the-badge)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Tauri-2-FFC131?style=for-the-badge&logo=tauri&logoColor=black)](https://tauri.app/)

[⬇️ Download](#%EF%B8%8F-quick-start) • [✨ Features](#-features) • [🏗️ Architecture](docs/architecture.md) • [🤝 Contributing](docs/contributing.md)

<br>

<img src="src-tauri/icons/icon.png" width="120" alt="Aircast logo" />

</div>

---

## 💡 What is Aircast?

Aircast is a **portable native desktop app** that captures any audio input and streams it to an Icecast server. Pick your microphone, save your server presets, click **Go Live**. Add a music queue, jingle cartridges with one-shot triggering, mic ducking and crossfade in **Studio mode**.

Built for radio operators who want to **own their tooling** — no subscriptions, no cloud lock-in, no hidden costs.

```
                                                     ┌────────────────┐
   🎙️  Mic   ─┐                                       │  🌐 Icecast    │
              │                                       │     server     │
   🎵  Music ─┼─►  Mixer  ─►  ffmpeg (PUT) ─►  ─────► │  (your radio)  │
              │                                       └────────────────┘
   🎚️  Cart  ─┘
              │
              └────────► 🔊 Local monitor (always on, never cuts)
```

---

## 🎯 Why Aircast?

|             💸 **Free & Open-Source**             |          🎙️ **Always-On Audio**          |       🔁 **Studio Mode**       |
| :-----------------------------------------------: | :--------------------------------------: | :----------------------------: |
| GPL-3.0 licensed. No subscriptions, no per-MB fees, ever. | Local monitor never cuts when going live. | Music queue, carts, ducking, crossfade. |

|        📦 **Self-Contained**         |         🖥️ **Truly Cross-Platform**         |        🌍 **i18n FR / EN**         |
| :----------------------------------: | :-----------------------------------------: | :--------------------------------: |
| ffmpeg bundled. No install required. | Native macOS, Windows, Linux. No Electron.  | French & English. Easy to extend.  |

---

## ⬇️ Quick Start

Download the latest build for your platform from the [Releases](https://github.com/Synapsr/Aircast/releases/latest) page:

| Platform                  | File                                       |
| ------------------------- | ------------------------------------------ |
| 🍎 **macOS** (Apple Silicon) | `Aircast_<version>_aarch64.dmg`           |
| 🪟 **Windows** (portable)    | `Aircast-portable-windows-x64.zip`        |
| 🪟 **Windows** (installer)   | `Aircast_<version>_x64-setup.exe`         |
| 🐧 **Linux**                  | `aircast_<version>_amd64.deb` / `.AppImage` |

> Aircast isn't signed yet (Apple Developer ID / Windows EV), so the OS
> will show a one-time warning the first time you launch the app. See the
> **[full installation guide](docs/installing.md)** for the exact 30-second
> procedure per platform (a single Terminal command for macOS, "Run anyway"
> for Windows, nothing for Linux).

Once installed: pick your microphone, add your Icecast server in **Setup**, click **Go Live**.

---

## ✨ Features

### 🎚️ Three modes, one app

|     | Mode      | What it does                                                                            |
| :-: | --------- | --------------------------------------------------------------------------------------- |
| 🎙️  | **Simple** | Pick a mic → pick a server → Go Live. The fastest path to air, source→destination at a glance. |
| 🎛️  | **Studio** | Music queue, 9-cart jingle bank, mic ducking, crossfade, on-air title chip. Full-featured radio panel. |
| 📡  | **Relay**  | Rebroadcast another stream URL (HTTP/HTTPS/HLS/Icecast). Drop in named upstream sources, transcoded on the fly. |

Each mode can be hidden in Settings → Advanced if a station only uses one workflow.

### 📡 Streaming

|     | Feature                       | Description                                                                              |
| :-: | ----------------------------- | ---------------------------------------------------------------------------------------- |
| ⚙️  | **Server presets**            | Save and recall as many Icecast servers as you need (host, port, mount, codec, bitrate). |
| 🔌  | **Any input device**          | Built-in mic, USB interface, virtual cable — anything the OS exposes.                    |
| 🎚️  | **Live VU meter**             | Real-time RMS + peak monitoring at 20 Hz, with correct color-zoned scale (green/yellow/red at fixed positions, not stretched). |
| 🔁  | **Auto-reconnect**            | Configurable retry interval on stream drops; instant fail-over on network blips.         |
| 🚨  | **Rich error dialog**         | ffmpeg/Icecast errors are classified into actionable messages; the raw output stays available for debugging. |
| 🎧  | **Codecs**                    | MP3 (libmp3lame) and AAC (native), 64–320 kbps.                                          |
| 🌐  | **Icecast 2.4+**              | HTTP PUT protocol — supports root mount path `/`, unlike the legacy `icecast://`.        |
| ⚠️  | **Live mode-switch guard**    | Switching modes while on-air shows a modal listing the concrete consequences (music stops, mic opens, etc.) so you don't air silence by accident. |

### 🎵 Studio mode

|     | Feature                  | Description                                                                                  |
| :-: | ------------------------ | -------------------------------------------------------------------------------------------- |
| 🎵  | **Music queue**          | Drag in MP3/WAV/FLAC/OGG. Play, pause, reorder, remove. Stream-decoded, low memory.          |
| 🎚️  | **Cart bank**            | 9 jingle slots, one-shot trigger, pre-decoded for instant playback.                          |
| 🎙️  | **Mic ducking**          | Music ramps down automatically when the mic opens. Configurable level.                       |
| 🔄  | **Crossfade**            | Smooth transitions between tracks with a configurable duration.                              |
| 🎼  | **Format-agnostic**      | Custom `FrameResampler` handles any source rate (22 / 44.1 / 48 / 96 kHz, mono or stereo).   |
| 🏷️  | **Now Playing chip**     | The title actually broadcast to listeners is shown live in the Now Playing card — one click to edit the broadcast settings. |

### 📡 Relay mode

|     | Feature                  | Description                                                                              |
| :-: | ------------------------ | ---------------------------------------------------------------------------------------- |
| 🔗  | **Named sources**        | Save as many upstream URLs as you want (HTTP/HTTPS audio streams, HLS .m3u8, Icecast, local files). |
| 🔁  | **Upstream reconnect**   | If the upstream drops, Aircast retries with a 5 s linear backoff and live status feedback (connecting / streaming / reconnecting). |
| 🎚️  | **Same destination UX**  | Source → arrow → server flow makes "what plays from where" obvious at a glance. |

### 🏷️ Now-playing broadcaster

Push the title to your Icecast `/admin/metadata` endpoint with **source credentials only** (no admin password needed — works with the mount-level auth that libshout and butt have used for years).

|     | Mode      | What it pushes                                                                            |
| :-: | --------- | ----------------------------------------------------------------------------------------- |
| 🎼  | **Auto**   | Renders a template from ID3 tags (`{title}` `{artist}` `{album}` `{next_title}` `{station}` `{show}` …). Configurable per preset. |
| 📝  | **Static** | Fixed text — useful for talk shows or pauses (e.g. *"You're listening to Radio XYZ"*). |
| 📂  | **File**   | Polls an external text file at a configurable interval (UTF-8 / UTF-16 BOM aware). Perfect for syncing with Mixxx, RadioDJ or other broadcasters. |

Plus a mic override (different title shown when the mic is open) and a "push title now" test button. The currently-broadcasting title is shown live in a strip (Simple mode) or chip (Studio mode).

### 🛠️ Reliability & support

|     | Feature                       | Description                                                                              |
| :-: | ----------------------------- | ---------------------------------------------------------------------------------------- |
| 🧪  | **100+ unit tests**           | 100 Rust + 37 TypeScript covering audio resampling, URL framing, BOM detection, presets, validation and i18n parity. |
| 🔒  | **Atomic preset writes**      | Corrupt JSON falls back to defaults — never bricks the app.                              |
| 🪶  | **Lock-free callbacks**       | cpal real-time path uses only atomics + ring buffers. No allocation, no locking.         |
| 📊  | **Persistent rolling log**    | Always-on file logger with rotation. Every stream/mode/error transition is timestamped.  |
| 🩺  | **Diagnostic bundle**         | One click in Settings → Advanced copies a sanitized report (version, OS, active config, last 300 log lines) ready to paste in a bug report. |
| 🌐  | **Deep links**                | `aircast://` URL scheme to share server configurations.                                  |
| 🛡️  | **Isolated dev / prod data**  | Dev builds (`pnpm tauri:dev`) use a separate identifier, so you can iterate locally without touching the production preset file. |

---

## 🏗️ Architecture

```
┌───────────────────────────┐                       ┌──────────────────────────────┐
│  React + TypeScript UI    │ ── tauri::invoke ──►  │  Rust backend (Tauri 2)       │
│  Tailwind v4, FR/EN i18n  │ ◄── tauri events ───  │   audio::capture (cpal)       │
└───────────────────────────┘                       │   studio::mixer + resampler   │
                                                    │   stream::pipeline            │
                                                    │   presets::store              │
                                                    └────────────────┬──────────────┘
                                                                     │ stdin (s16le PCM)
                                                                     ▼
                                                    ┌──────────────────────────────┐
                                                    │  ffmpeg sidecar (subprocess) │
                                                    │   HTTP PUT → Icecast 2.4+    │
                                                    └──────────────────────────────┘
```

Capture is **always-on** as soon as a device is selected. The streaming pipeline taps into the same audio flow without restarting it — switching live → idle never cuts the local monitor.

Full design notes in [`docs/architecture.md`](docs/architecture.md).

---

## 🛠️ Build from Source

Want to hack on it?

```bash
# Prerequisites: Rust (stable), Node 20+, pnpm 9+
git clone https://github.com/Synapsr/Aircast.git
cd Aircast
pnpm install
pnpm fetch-ffmpeg          # downloads the ffmpeg sidecar for your host
pnpm tauri dev             # runs the app in dev mode
```

To produce installable bundles for your host platform:

```bash
pnpm build:bundle          # → src-tauri/target/release/bundle/...
```

CI builds for **macOS (arm64 + x64)**, **Windows** and **Linux** are produced automatically on every git tag — see [`.github/workflows/release.yml`](.github/workflows/release.yml).

---

## 🤝 Contributing

PRs are welcome. Read [`docs/contributing.md`](docs/contributing.md) first — it covers local checks, style and the architecture invariants that came from real bugs. Don't break those without saying why.

The local check suite that CI runs on every PR:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cd ..
pnpm typecheck
pnpm test
pnpm build
```

---

## 📜 License

- **Aircast source** — GPL-3.0-or-later, see [`LICENSE`](LICENSE).
- **Bundled ffmpeg** — LGPL static build. Aircast spawns ffmpeg as a separate subprocess, so under the FSF "mere aggregation" interpretation, ffmpeg's license does not propagate to Aircast's source.

---

<div align="center">

Made with ♥ for radio operators by [Synapsr](https://github.com/Synapsr).

</div>

---

<div align="center">

<img src="public/france2030.svg" alt="France 2030" width="120" />

<sub>Operation supported by the French State as part of the *Territoires Numériques Éducatifs* initiative of the *Programme d'investissements d'avenir*, operated by Caisse des Dépôts.</sub>

<sub>[Discover Suite.Studio](https://suite.studio/) · [Porte-Voix.app](https://porte-voix.app/)</sub>

</div>
