# Contributing to Aircast

Thanks for considering a contribution. Aircast is a small, opinionated tool
for radio operators — bug reports, fixes, and well-scoped features are all
welcome.

## Quick start

```bash
# 1. Install Rust (stable) and Node 20+ with pnpm 9+
pnpm install

# 2. Fetch the ffmpeg sidecar for your host platform
pnpm fetch-ffmpeg

# 3. Run the app in dev mode
pnpm tauri dev
```

You'll need a Tauri toolchain set up for your OS. See
[tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/) — the
short version is: a working C toolchain plus, on Linux, the `webkit2gtk-4.1`
and `libasound2` development packages.

## Running checks locally

CI runs the same six commands. Run them before opening a PR:

```bash
# Rust
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib

# Frontend
cd ..
pnpm typecheck
pnpm test
pnpm build
```

`cargo clippy -- -D warnings` is enforced — new clippy warnings will fail CI.
If clippy flags something you genuinely disagree with, prefer
`#[allow(clippy::lint_name)]` with a one-line justification over disabling
the lint globally.

## Style

- **Rust**: stable toolchain, MSRV is `1.83` (set in `Cargo.toml`). Format
  with `cargo fmt`, check with `cargo clippy -- -D warnings`. No `unsafe`
  unless there's a comment explaining why and what invariant it relies on.
- **TypeScript**: strict mode is on. Match the surrounding code — no Prettier
  config is enforced, but reviewers will ask for tabs/spaces consistent with
  the file you're editing. Avoid adding new dependencies for things React
  can do natively.
- **Comments**: only when the *why* is non-obvious. Don't restate what the
  code does. The architecture is documented in
  [`docs/architecture.md`](architecture.md), not inline.
- **No emojis** in source or commit messages unless they were already there.

## Testing

- Rust tests live next to the code in `#[cfg(test)] mod tests`. Use
  `tempfile::TempDir` for anything that needs the filesystem; never write
  into the user's `app_data_dir` from a test.
- Frontend tests live next to the code as `*.test.ts` and run under
  vitest+jsdom. Pure-logic helpers in `src/lib/` are the easiest things to
  cover.
- Adding a new translation key? The i18n test
  ([`src/i18n/i18n.test.ts`](../src/i18n/i18n.test.ts)) will fail until both
  dictionaries have it.

## Architecture invariants

These came from real bugs. Don't break them without saying why in the PR:

- **cpal callbacks must not allocate, lock, or block.** Use `AtomicF32` for
  gains, `AtomicBool` for flags, lock-free rings for samples.
- **The stop-flag check at the top of every cpal callback stays.** It's the
  workaround for macOS not synchronously stopping the audio thread when a
  stream is dropped. Removing it brings back audio doubling on Go-Live cycles.
- **`FrameResampler` is the only sample-rate converter.** Music and cart
  both go through it. A second converter creates two places to fix when
  format-edge bugs appear.
- **The pipeline emits `Error` before `Reconnecting`.** The frontend depends
  on this order to keep `lastStreamError` visible across reconnect attempts.
- **Preset writes are atomic** (write-temp + rename) and the loader falls
  back to defaults on corrupt JSON. A corrupted file shouldn't brick the app.

## Pull requests

- Keep PRs focused. A bug fix and an unrelated refactor belong in two PRs.
- Describe what changed and *why*. The why is what reviewers can't see in
  the diff.
- Link the issue you're fixing if one exists.
- If you change runtime behaviour, mention how you tested it. CI doesn't
  cover audio output or live streaming — those still need a human.

## Reporting bugs

Audio bugs are notoriously hard to reproduce remotely. When filing one,
include:

- OS + version, audio interface, sample rate of the input device;
- the streaming format (MP3 / AAC) and bitrate;
- the contents of the dialog if the app showed an error;
- if relevant, the last few lines of the log file (the path is shown in
  Settings → About).

For everything that isn't an audio bug, the usual things help: steps to
reproduce, what you expected, what happened.

## License

Aircast is GPL-3.0-or-later licensed. By contributing, you agree your contribution will be
released under the same license.
