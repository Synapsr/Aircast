# Aircast architecture

This document describes how Aircast is wired together: the layers, the audio
flow, the threading model, and the key invariants. Read it before making any
non-trivial change to the audio pipeline or the streaming code — the constraints
here are not obvious from the source files alone.

## High-level layers

```
┌──────────────────────────────────────────────────────────────┐
│ React + TypeScript frontend (src/)                           │
│   - components/, hooks/, i18n/, lib/                         │
│   - talks to the backend through Tauri commands + events     │
└──────────────────────────────────────────────────────────────┘
                          ▲   ▲
                  invoke  │   │ events  (status, vu, errors)
                          ▼   ▼
┌──────────────────────────────────────────────────────────────┐
│ Tauri command surface (src-tauri/src/commands.rs)            │
│   - thin: validates input, mutates state, returns AppResult  │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ Domain modules                                               │
│   audio/    – cpal capture + monitor playback                │
│   studio/   – mixer, music player, cart bank, resampler      │
│   stream/   – ffmpeg sidecar pipeline + status events        │
│   presets/  – on-disk preset store (atomic JSON)             │
│   state.rs  – shared AppState (Arc<Mutex<…>>)                │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ External                                                     │
│   cpal       – cross-platform audio I/O                      │
│   ffmpeg     – subprocess sidecar (encoder + Icecast client) │
│   Icecast    – the user's streaming server                   │
└──────────────────────────────────────────────────────────────┘
```

The frontend never touches audio directly — every frame is captured, mixed and
encoded in the Rust backend. Frontend state is derived from events emitted by
the backend (`status`, `vu`, `error`, …) plus the values returned from
commands.

## Audio data flow

A single live cycle pulls samples from three sources, mixes them, and writes
the result both to the monitor speaker (so the operator can hear it) and to
ffmpeg's stdin (so the listeners hear it).

```
                               ┌────────────────────┐
   mic input device  ─cpal──▶  │  CaptureStream     │ ─┐
                               └────────────────────┘  │
                                                       │
   queued tracks  ─rodio─▶ FrameResampler ─▶  MusicPlayer  ─┤
                                                       │
   loaded files  ─decode─▶ CartBank slots  ─▶  CartBank ────┤
                                                       │   ▼
                                                  ┌──────────┐
                                                  │  Mixer   │  ── monitor (cpal output)
                                                  │ + ducking│
                                                  │ + gains  │  ── ffmpeg stdin (s16le PCM)
                                                  └──────────┘
                                                                                │
                                                                                ▼
                                                                          Icecast server
```

### Sample format

Internally everything is **f32 PCM**, interleaved, at the **monitor output's
native rate and channel count**. ffmpeg receives the same stream converted to
s16le on stdin; ffmpeg owns the resample-to-encoder-rate step (it's already
inside ffmpeg, no need to do it twice).

### Why a custom resampler

`rodio`'s `UniformSourceIterator` re-bootstraps its internal converter at every
MP3 frame boundary, which dropped enough samples to play music at ~7× speed in
early prototypes. `studio/resampler.rs` replaces it with a deliberately simple
sample-and-hold converter:

```text
desired_source_idx = floor(target_emitted × source_rate / target_rate)
```

It is driven one target frame at a time, which keeps it format-agnostic — same
code path handles mono/stereo, 22.05 / 44.1 / 48 / 96 kHz sources. The
converter is the single source of truth: both `MusicPlayer` and `CartBank`
delegate to it via `FrameResampler::step`, so any improvement is shared.

This is intentionally not a high-quality resampler. It introduces aliasing on
extreme ratios. For radio-grade speech and music it is inaudible after the MP3
encoder's lowpass; if a higher-quality resampler is ever needed, the place to
upgrade is `step()`.

## Threading model

There are several independent threads, and the boundaries are deliberate.

| Thread                     | Owner                         | Real-time? | Notes                                                                 |
|----------------------------|-------------------------------|------------|-----------------------------------------------------------------------|
| Tauri main / command       | `tauri::Builder`              | No         | Mutates `AppState` under `Arc<Mutex<…>>`, never blocks for long.      |
| cpal capture callback      | `audio/capture.rs`            | **Yes**    | Pushes f32 frames to a ring; only atomic ops + a bounded copy.        |
| cpal monitor callback      | `audio/playback.rs`           | **Yes**    | Pulls from the mixer; only atomic ops + arithmetic.                   |
| ffmpeg writer task         | `stream/pipeline.rs` (tokio)  | No         | Pulls mixed s16le PCM and writes to ffmpeg stdin.                     |
| ffmpeg reader task         | `stream/pipeline.rs` (tokio)  | No         | Reads stderr, classifies errors, emits status events.                 |
| Music decode thread        | `studio/music.rs`             | No         | Decodes a `rodio::Decoder` into a bounded ring; back-pressured.       |
| VU meter timer             | `vu.rs`                       | No         | Reads atomic peak/RMS and emits an event every ~50 ms.                |

### Real-time discipline (callbacks)

cpal callbacks **must not allocate, lock, or block**. Specifically:

- gains are read via [`AtomicF32`](../src-tauri/src/studio/atomic_f32.rs) (an
  `AtomicU32` reinterpreted as f32);
- the stop signal is a single `AtomicBool`;
- frames flow through pre-allocated lock-free ring buffers;
- if a music/cart frame isn't ready, the mixer outputs silence for that
  channel rather than waiting.

### macOS stream-drop quirk

On macOS, dropping a `cpal::Stream` does **not** synchronously stop the audio
thread — it can keep firing callbacks for several hundred milliseconds. If a
new live cycle starts during that window you get audio doubling.

The fix lives in `audio/playback.rs` and `audio/capture.rs`: every callback
checks an `Arc<AtomicBool>` "stop flag" first and returns early once set. The
stream is kept alive until cpal genuinely tears it down, but the callback
becomes a no-op as soon as we call `stop()`. **Do not remove this check** —
the bug it fixes only reproduces on macOS hardware, not in tests or under the
debugger.

## Mixer

`studio/mixer.rs` is the convergence point. It owns:

- `mic_gain`, `music_gain`, `monitor_gain`, `master_gain` — `AtomicF32`
  values written by commands and read by the monitor / ffmpeg callbacks;
- a ducking state machine (when the mic opens, music gain ramps down to
  `ducking_level` over `ducking_ms`; when the mic closes, it ramps back);
- the stop flag.

All mixing is additive in f32 (`map_channels_add` from
[`resampler.rs`](../src-tauri/src/studio/resampler.rs)) and the final result is
written into the monitor buffer with `map_channels_set` so we don't accumulate
on top of whatever cpal happened to leave there.

Channel mapping rules:

- mono → multi: broadcast the single channel;
- multi → mono: average of the first two channels (close enough for VU and
  monitor purposes; the encoder side uses ffmpeg's downmix);
- otherwise: channel-wise copy with the last source channel duplicated as a
  fallback.

## Stream pipeline

`stream/pipeline.rs` spawns the ffmpeg sidecar and manages its lifecycle:

1. resolve the binary path via `stream/ffmpeg_path.rs` (next to the exe →
   resource dir → PATH);
2. build the icecast URL with `stream/ffmpeg.rs::build_icecast_url`, which
   percent-encodes the credentials and the mount path while leaving the host
   alone;
3. spawn ffmpeg with the right codec args (`codec_args` chooses MP3 or AAC);
4. push s16le PCM into ffmpeg's stdin from the mixer;
5. parse ffmpeg's stderr line-by-line, classify it (`classify_error`,
   `is_fatal_error`) and emit `status` / `error` events.

A fatal error triggers the reconnect loop. The pipeline emits **Error first,
then Reconnecting**, so the frontend can record `lastStreamError` and keep the
dialog open even after the status flips back to "connecting".

## State management

`state.rs` owns the runtime state for a single app instance:

- `AppState { stream, music, cart, mixer, capture, monitor, current_format }`
- everything is wrapped in `Arc<Mutex<…>>` and held under a single
  `tauri::State<AppState>`;
- commands lock briefly, mutate or read, and return.

The on-disk state lives in `presets/store.rs` (`aircast.json` in
`app_data_dir`). Writes are atomic (write-to-temp + rename) and the loader
falls back to defaults on corrupt JSON rather than crashing — a corrupt
preset file should never brick the app.

## Errors

`error.rs` defines `AppError` and `AppResult<T>`. Everything that crosses the
Tauri boundary is `AppResult<T>`; `commands.rs` is the only place that maps
errors to user-visible messages.

`stream/ffmpeg.rs::classify_error` matches a small, deliberately conservative
set of patterns (refused / 401 / 403 / 404 / DNS / timeout) to translatable
keys. Anything we can't classify is forwarded verbatim to the UI — operators
need the real ffmpeg message when something exotic breaks.

## Frontend conventions

- React + Tailwind v4, no global CSS framework on top;
- one hook per slice of backend state (`useStreamStatus`, `useMicOpen`,
  `useMusic`, …) — each subscribes to the relevant event and exposes a
  React-friendly snapshot;
- `lib/api.ts` is the only file that calls `invoke` directly;
- `lib/validation.ts` runs client-side checks before invoking, so the user
  sees per-field errors before any backend call;
- i18n lives in `src/i18n/` — a `translate()` helper plus FR/EN nested
  dictionaries (`locales/en.json`, `locales/fr.json`). The unit test enforces
  that both dictionaries have identical leaf-key sets.

## Testing

- **Rust unit tests** live next to the code in `#[cfg(test)] mod tests`. The
  resampler, mixer, ffmpeg URL builder, music player, cart bank and preset
  store are all covered. `tempfile::TempDir` is used wherever we need real
  WAV/JSON files on disk.
- **TypeScript unit tests** live next to the code as `*.test.ts` and run
  under vitest+jsdom. They cover deep-link parsing, validation and i18n.
- **No integration tests against a real Icecast server.** The protocol is
  ffmpeg's responsibility, and mocking ffmpeg's behaviour against icecast
  has historically diverged from production. We test the URL we build and
  the error classification, not the wire protocol.

CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
`pnpm typecheck`, `pnpm test`, and `pnpm build` on every push. The bundle
matrix (macOS / Windows / Linux) runs on `main`.

## Where to add things

| Want to…                                | Touch                                                  |
|-----------------------------------------|--------------------------------------------------------|
| Add a new Tauri command                 | `commands.rs` + the relevant module + `lib/api.ts`     |
| Add a new mixer source                  | `studio/mixer.rs` (gain) + own module under `studio/`  |
| Change how ffmpeg is invoked            | `stream/ffmpeg.rs` (`codec_args`, `build_icecast_url`) |
| Change error classification             | `stream/ffmpeg.rs` (`classify_error`, `is_fatal_error`)|
| Add a new preset field                  | `presets/mod.rs` + `presets/store.rs` (migration-free) |
| Add a new translation                   | `src/i18n/index.ts` (both `fr` and `en` dictionaries)  |
