<div align="right">

🌐 **Language**: [Français](README.fr.md) | **English**

</div>

<div align="center">

# 🎙️ Aircast

### The Open-Source Desktop App for Radio Streaming

**Stop paying for proprietary streaming software. Own your radio.**

[![Latest Release](https://img.shields.io/github/v/release/Synapsr/Aircast?style=for-the-badge&logo=github&label=Release)](https://github.com/Synapsr/Aircast/releases/latest)
[![GitHub Stars](https://img.shields.io/github/stars/Synapsr/Aircast?style=for-the-badge&logo=github)](https://github.com/Synapsr/Aircast)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)
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
| MIT licensed. No subscriptions, no per-MB fees, ever. | Local monitor never cuts when going live. | Music queue, carts, ducking, crossfade. |

|        📦 **Self-Contained**         |         🖥️ **Truly Cross-Platform**         |        🌍 **i18n FR / EN**         |
| :----------------------------------: | :-----------------------------------------: | :--------------------------------: |
| ffmpeg bundled. No install required. | Native macOS, Windows, Linux. No Electron.  | French & English. Easy to extend.  |

---

## ⬇️ Quick Start

Download the latest build for your platform from the [Releases](https://github.com/Synapsr/Aircast/releases/latest) page:

| Platform                  | File                                       | Notes                                                      |
| ------------------------- | ------------------------------------------ | ---------------------------------------------------------- |
| 🍎 **macOS** (Apple Silicon) | `Aircast_<version>_aarch64.dmg`           | Drag into Applications. First launch: right-click → Open.  |
| 🍎 **macOS** (Intel)         | `Aircast_<version>_x64.dmg`               | Same as above.                                             |
| 🪟 **Windows** (portable)    | `Aircast-portable-windows-x64.zip`        | **No install, no admin.** Extract, double-click `Aircast.exe`. |
| 🪟 **Windows** (installer)   | `Aircast_<version>_x64-setup.exe`         | NSIS installer. Per-user, no admin prompt.                 |
| 🐧 **Linux**                  | `aircast_<version>_amd64.deb` / `.AppImage` | Standard Debian package or self-contained AppImage.        |

> 🪟 **Windows note**: requires the WebView2 runtime, included by default on Windows 10 1803+ and Windows 11. On older versions, install it from [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/).

That's it. Pick your microphone, enter your Icecast server in **Setup**, click **Go Live**.

---

## ✨ Features

### 📡 Streaming

|     | Feature                  | Description                                                                              |
| :-: | ------------------------ | ---------------------------------------------------------------------------------------- |
| ⚙️  | **Server presets**       | Save and recall as many Icecast servers as you need (host, port, mount, codec, bitrate). |
| 🔌  | **Any input device**     | Built-in mic, USB interface, virtual cable — anything the OS exposes.                    |
| 🎚️  | **Live VU meter**        | Real-time RMS + peak monitoring at 20 Hz.                                                |
| 🔁  | **Auto-reconnect**       | Configurable retry interval; instant fail-over on network blips.                         |
| 🚨  | **Clear errors**         | Auth, mount, network and timeout errors are classified into actionable messages.         |
| 🎧  | **Codecs**               | MP3 (libmp3lame) and AAC (native), 64–320 kbps.                                          |
| 🌐  | **Icecast 2.4+**         | HTTP PUT protocol — supports root mount path `/`, unlike the legacy `icecast://`.        |

### 🎛️ Studio Mode

|     | Feature                  | Description                                                                                  |
| :-: | ------------------------ | -------------------------------------------------------------------------------------------- |
| 🎵  | **Music queue**          | Drag in MP3/WAV/FLAC/OGG. Play, pause, reorder, remove. Stream-decoded, low memory.          |
| 🎚️  | **Cart bank**            | 12 jingle slots, one-shot trigger, pre-decoded for instant playback.                         |
| 🎙️  | **Mic ducking**          | Music ramps down automatically when the mic opens. Configurable level and ramp time.        |
| 🔄  | **Crossfade**            | Smooth transitions between tracks with a configurable duration.                              |
| 🎼  | **Format-agnostic**      | Custom `FrameResampler` handles any source rate (22 / 44.1 / 48 / 96 kHz, mono or stereo).    |

### 🛠️ Reliability

|     | Feature                  | Description                                                                                |
| :-: | ------------------------ | ------------------------------------------------------------------------------------------ |
| 🧪  | **118 unit tests**       | 81 Rust + 37 TypeScript covering audio, networking, presets, validation and i18n parity.   |
| 🔒  | **Atomic preset writes** | Corrupt JSON falls back to defaults — never bricks the app.                                |
| 🪶  | **Lock-free callbacks**  | cpal real-time path uses only atomics + ring buffers. No allocation, no locking.           |
| 📊  | **Structured logging**   | Every component logs through `log` with adjustable level via `RUST_LOG`.                   |
| 🌐  | **Deep links**           | `aircast://` URL scheme to share server configurations.                                    |

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

- **Aircast source** — MIT, see [`LICENSE`](LICENSE).
- **Bundled ffmpeg** — LGPL static build. Aircast spawns ffmpeg as a separate subprocess, so under the FSF "mere aggregation" interpretation, ffmpeg's license does not propagate to Aircast's source.

---

<div align="center">

Made with ♥ for radio operators by [Synapsr](https://github.com/Synapsr).

</div>
