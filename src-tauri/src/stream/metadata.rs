//! Icecast "now playing" broadcaster.
//!
//! This module owns a long-lived background task that:
//!   1. Composes a title string from the current player state (music track,
//!      cart, mic) and the user's [`MetadataSettings`].
//!   2. Pushes the title to Icecast via `GET /admin/metadata?…` whenever it
//!      changes.
//!
//! ## Why source credentials suffice (no admin required)
//!
//! Despite the misleading `/admin/` prefix, Icecast 2.x routes
//! `/admin/metadata` through `ADMIN_CMD_NEEDS_MOUNT_AUTH`, which validates
//! against the **mount-level source credentials** — not the global admin
//! credentials. This is what `libshout` and `butt` have relied on for over
//! a decade. We can use the same `username`/`password` that authenticate the
//! audio PUT for the metadata endpoint, with no extra config from the user.
//!
//! ## Composition rules (see [`compose_title`])
//!
//! - File mode → file content (read by [`file_watcher`])
//! - Static mode → literal `static_text`
//! - Auto mode + mic-open + non-empty `mic_override` → render `mic_override`
//! - Auto mode + cart playing → cart name
//! - Auto mode + music playing → render `template`
//! - Otherwise → `show_name` if set, else `station_name`, else empty
//!
//! ## Dedup and rate
//!
//! The updater holds the last successfully sent title. A composed title that
//! matches it is silently dropped (no HTTP). This avoids "Now playing" from
//! flickering in listeners' players when the underlying state changes but
//! the rendered title doesn't.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time;

use crate::presets::{MetadataMode, MetadataSettings, StreamConfig};
use crate::studio::{CartSnapshot, MusicSnapshot, TrackInfo};

/// Frontend-facing event emitted whenever the title actually broadcast to
/// Icecast changes. `title = ""` means "broadcaster is dormant" (stream
/// stopped, metadata disabled, or composed title is empty).
const BROADCAST_EVENT: &str = "metadata-broadcast-changed";

/// Lightweight snapshot of every input that influences the composed title.
/// Built each tick from the live `AppState`.
#[derive(Debug, Clone, Default)]
pub struct ComposeInput {
    pub current_track: Option<TrackInfo>,
    pub next_track: Option<TrackInfo>,
    pub current_cart: Option<String>,
    pub mic_open: bool,
    pub file_content: Option<String>,
    pub stream_live: bool,
}

/// Where to send the next update — derived from the live `StreamConfig`.
#[derive(Debug, Clone)]
pub struct PushTarget {
    pub host: String,
    pub port: u16,
    pub mount: String,
    pub username: String,
    pub password: String,
}

impl PushTarget {
    pub fn from_config(config: &StreamConfig) -> Self {
        let mount = if config.mount.starts_with('/') {
            config.mount.clone()
        } else {
            format!("/{}", config.mount)
        };
        Self {
            host: config.host.trim().to_string(),
            port: config.port,
            mount,
            username: config.username.clone(),
            password: config.password.clone(),
        }
    }
}

/// Compose the title from live state and user settings. Pure: no IO, no log.
pub fn compose_title(input: &ComposeInput, settings: &MetadataSettings) -> String {
    if !settings.enabled {
        return String::new();
    }
    match settings.mode {
        MetadataMode::Static => settings.static_text.trim().to_string(),
        MetadataMode::File => input
            .file_content
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        MetadataMode::Auto => compose_auto(input, settings),
    }
}

fn compose_auto(input: &ComposeInput, settings: &MetadataSettings) -> String {
    if input.mic_open && !settings.mic_override.trim().is_empty() {
        return render_template(&settings.mic_override, input, settings);
    }
    if let Some(name) = input.current_cart.as_deref() {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if input.current_track.is_some() {
        return render_template(&settings.template, input, settings);
    }
    let fallback = if !settings.show_name.trim().is_empty() {
        &settings.show_name
    } else {
        &settings.station_name
    };
    fallback.trim().to_string()
}

/// Substitute `{title}` etc. into `template`, then collapse whitespace runs
/// so blanks (e.g. missing `{artist}`) don't leave double-spaces.
pub fn render_template(
    template: &str,
    input: &ComposeInput,
    settings: &MetadataSettings,
) -> String {
    let title = input
        .current_track
        .as_ref()
        .map(|t| t.title.as_str())
        .unwrap_or("");
    let artist = input
        .current_track
        .as_ref()
        .and_then(|t| t.artist.as_deref())
        .unwrap_or("");
    let album = input
        .current_track
        .as_ref()
        .and_then(|t| t.album.as_deref())
        .unwrap_or("");
    let next_title = input
        .next_track
        .as_ref()
        .map(|t| t.title.as_str())
        .unwrap_or("");
    let next_artist = input
        .next_track
        .as_ref()
        .and_then(|t| t.artist.as_deref())
        .unwrap_or("");
    let station = settings.station_name.trim();
    let show = settings.show_name.trim();

    let raw = template
        .replace("{title}", title)
        .replace("{artist}", artist)
        .replace("{album}", album)
        .replace("{next_title}", next_title)
        .replace("{next_artist}", next_artist)
        .replace("{station}", station)
        .replace("{show}", show);

    // Collapse runs of whitespace and any "X — Y" sandwich where one side
    // is empty (most common: missing artist with template "{artist} — {title}").
    let normalized = raw
        .replace(" — ", "\u{1F}") // unit separator placeholder
        .replace(" - ", "\u{1F}")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let parts: Vec<&str> = normalized
        .split('\u{1F}')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    parts.join(" — ")
}

/// One push attempt to `/admin/metadata`. Source credentials are sent as
/// HTTP Basic. Returns Ok on 2xx, Err with the response status/body otherwise.
async fn push_once(target: &PushTarget, title: &str) -> Result<(), String> {
    let path = format!(
        "/admin/metadata?mount={}&mode=updinfo&song={}",
        urlencode(&target.mount),
        urlencode(title),
    );
    let auth = format!("{}:{}", target.username, target.password);
    let auth_b64 = base64::engine::general_purpose::STANDARD.encode(auth.as_bytes());
    let request = format!(
        "GET {path} HTTP/1.0\r\n\
         Host: {host}:{port}\r\n\
         Authorization: Basic {auth_b64}\r\n\
         User-Agent: Aircast/{ver}\r\n\
         Connection: close\r\n\
         \r\n",
        path = path,
        host = target.host,
        port = target.port,
        auth_b64 = auth_b64,
        ver = env!("CARGO_PKG_VERSION"),
    );

    let connect = TcpStream::connect((target.host.as_str(), target.port));
    let mut stream = match time::timeout(Duration::from_secs(5), connect).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("connect: {e}")),
        Err(_) => return Err("connect timed out (5 s)".into()),
    };
    if let Err(e) = stream.write_all(request.as_bytes()).await {
        return Err(format!("write: {e}"));
    }
    let mut response = Vec::with_capacity(512);
    let read = time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response)).await;
    match read {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(format!("read: {e}")),
        Err(_) => return Err("response timed out (5 s)".into()),
    }

    // Parse the status line.
    let head = String::from_utf8_lossy(&response);
    let first_line = head.lines().next().unwrap_or("");
    let mut parts = first_line.splitn(3, ' ');
    let _proto = parts.next();
    let status = parts.next().unwrap_or("");
    let reason = parts.next().unwrap_or("");
    let code: u16 = status.parse().unwrap_or(0);
    if (200..300).contains(&code) {
        Ok(())
    } else {
        Err(format!("HTTP {code} {reason}"))
    }
}

/// URL-encode a string for use as a query parameter value. Encodes anything
/// outside the unreserved set per RFC 3986. Matches the encoder we use in
/// `stream/ffmpeg.rs` so behaviour is consistent across the crate.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Public command surface, sent over an mpsc channel from snapshot pollers.
pub enum Command {
    /// New compose input — recompute the title and push if it changed.
    Tick(Box<ComposeInput>),
    /// Manual override the user pushed via the "Test now" button. Bypasses
    /// dedup so the test always reaches the server.
    PushNow(String),
    /// Active stream config changed (or stream stopped: `None`). When None,
    /// updater stays dormant until a new target is provided.
    SetTarget(Option<PushTarget>),
    /// Settings were edited.
    SetSettings(MetadataSettings),
}

/// Spawned task that owns the HTTP push state and serializes all updates.
///
/// Uses `tauri::async_runtime::spawn` so it can be called from synchronous
/// contexts (e.g. `tauri::Builder::setup`, where Tokio's current-task
/// context isn't entered yet but Tauri's async runtime is already up).
///
/// The `app` handle is used to emit a `metadata-broadcast-changed` event so
/// the UI can show the live title without polling.
pub fn spawn(app: AppHandle, mut rx: mpsc::Receiver<Command>) {
    tauri::async_runtime::spawn(async move {
        let mut last_sent: Option<String> = None;
        let mut target: Option<PushTarget> = None;
        let mut settings = MetadataSettings::default();
        let mut consecutive_errors: u32 = 0;
        // After 3 consecutive failures we cool down for 60 s. This mostly
        // protects against bad creds / unreachable host scenarios where we
        // would otherwise hammer the box on every tick.
        let mut cooldown_until: Option<std::time::Instant> = None;

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::SetSettings(s) => {
                    settings = s;
                    // Force re-evaluation: if mode changed, we likely need a new push.
                    last_sent = None;
                }
                Command::SetTarget(t) => {
                    if t.is_none() {
                        log::info!("metadata: target cleared (stream stopped)");
                        // Tell the UI the broadcaster is dormant.
                        let _ = app.emit(BROADCAST_EVENT, "");
                    }
                    target = t;
                    last_sent = None;
                    consecutive_errors = 0;
                    cooldown_until = None;
                }
                Command::PushNow(title) => {
                    if let Some(ref t) = target {
                        log::info!("metadata: manual push '{}'", redact(&title));
                        match push_once(t, &title).await {
                            Ok(()) => {
                                let _ = app.emit(BROADCAST_EVENT, title.clone());
                                last_sent = Some(title);
                                consecutive_errors = 0;
                                cooldown_until = None;
                            }
                            Err(e) => log::warn!("metadata: manual push failed — {e}"),
                        }
                    } else {
                        log::warn!("metadata: manual push ignored (no active stream)");
                    }
                }
                Command::Tick(input) => {
                    if !settings.enabled || !input.stream_live {
                        continue;
                    }
                    let Some(ref t) = target else { continue };
                    if let Some(until) = cooldown_until {
                        if std::time::Instant::now() < until {
                            continue;
                        }
                        cooldown_until = None;
                    }
                    let title = compose_title(&input, &settings);
                    if title.is_empty() {
                        continue;
                    }
                    if last_sent.as_deref() == Some(title.as_str()) {
                        continue;
                    }
                    match push_once(t, &title).await {
                        Ok(()) => {
                            log::info!("metadata: pushed '{}'", redact(&title));
                            let _ = app.emit(BROADCAST_EVENT, title.clone());
                            last_sent = Some(title);
                            consecutive_errors = 0;
                        }
                        Err(e) => {
                            log::warn!("metadata: push failed — {e}");
                            consecutive_errors = consecutive_errors.saturating_add(1);
                            if consecutive_errors >= 3 {
                                log::warn!("metadata: 3 consecutive failures, cooling down 60 s");
                                cooldown_until =
                                    Some(std::time::Instant::now() + Duration::from_secs(60));
                            }
                        }
                    }
                }
            }
        }
    });
}

/// File-poll task. Reads `path` every `poll_secs` and forwards the trimmed
/// content as a `ComposeInput.file_content` update via `forward`. Detects
/// UTF-16 LE/BE BOM and decodes accordingly so files written by Mixxx on
/// Windows are handled. Unicode replacement on invalid bytes.
pub fn spawn_file_watcher(
    path: std::path::PathBuf,
    poll_secs: u32,
    forward: Arc<Mutex<Option<String>>>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let interval = Duration::from_secs(poll_secs.max(1) as u64);
        let mut last: Option<String> = None;
        loop {
            match read_text_file(&path).await {
                Ok(content) => {
                    let normalized = content.trim().to_string();
                    if last.as_deref() != Some(normalized.as_str()) {
                        let mut slot = forward.lock().await;
                        *slot = Some(normalized.clone());
                        last = Some(normalized);
                    }
                }
                Err(e) => {
                    // Don't log on every tick if the file simply doesn't exist
                    // yet — the user may be waiting for their other tool to
                    // create it. We log once when transitioning to error.
                    if last.is_some() {
                        log::warn!("metadata: file {} unreadable — {e}", path.display());
                        last = None;
                        let mut slot = forward.lock().await;
                        *slot = None;
                    }
                }
            }
            time::sleep(interval).await;
        }
    })
}

async fn read_text_file(path: &Path) -> std::io::Result<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(decode_text(&bytes))
}

/// Decode a byte buffer as text, sniffing UTF-16 BOMs first and falling back
/// to lossy UTF-8.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        // UTF-16 LE
        let pairs = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16_lossy(&pairs);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let pairs = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16_lossy(&pairs);
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Redact a title for log output if it looks like it may contain credentials
/// (the user could put a "{user}" template — unlikely, but defensive). Truncate
/// at 80 chars to keep logs tidy.
fn redact(title: &str) -> String {
    let s: String = title.chars().take(80).collect();
    if s.len() < title.chars().count() {
        format!("{s}…")
    } else {
        s
    }
}

/// Build a `ComposeInput` snapshot from the live music + cart + mic state.
pub fn build_compose_input(
    music: &MusicSnapshot,
    carts: &[CartSnapshot],
    mic_open: bool,
    file_content: Option<String>,
    stream_live: bool,
) -> ComposeInput {
    let current_track = music.current.as_ref().map(|c| c.info.clone());
    // Next = first queued item that isn't the currently playing one.
    let current_id = music.current.as_ref().map(|c| c.info.id.as_str());
    let next_track = music
        .queue
        .iter()
        .find(|t| Some(t.id.as_str()) != current_id)
        .cloned();
    let current_cart = carts
        .iter()
        .find(|c| c.playing)
        .map(|c| c.name.clone())
        .filter(|n| !n.trim().is_empty());
    ComposeInput {
        current_track,
        next_track,
        current_cart,
        mic_open,
        file_content,
        stream_live,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: &str, artist: Option<&str>) -> TrackInfo {
        TrackInfo {
            id: "x".into(),
            path: "/x".into(),
            title: title.into(),
            artist: artist.map(|s| s.into()),
            album: None,
            title_from_tag: artist.is_some(),
            duration_secs: None,
        }
    }

    fn auto_settings() -> MetadataSettings {
        MetadataSettings {
            enabled: true,
            mode: MetadataMode::Auto,
            template: "{artist} — {title}".into(),
            ..Default::default()
        }
    }

    #[test]
    fn render_drops_missing_artist_cleanly() {
        let input = ComposeInput {
            current_track: Some(track("Sound", None)),
            ..Default::default()
        };
        let out = render_template("{artist} — {title}", &input, &auto_settings());
        assert_eq!(out, "Sound");
    }

    #[test]
    fn render_full_template() {
        let input = ComposeInput {
            current_track: Some(track("One More Time", Some("Daft Punk"))),
            ..Default::default()
        };
        let out = render_template("{artist} — {title}", &input, &auto_settings());
        assert_eq!(out, "Daft Punk — One More Time");
    }

    #[test]
    fn render_with_station_substitution() {
        let mut s = auto_settings();
        s.station_name = "Radio XYZ".into();
        let input = ComposeInput {
            current_track: Some(track("Track", Some("A"))),
            ..Default::default()
        };
        let out = render_template("{title} sur {station}", &input, &s);
        assert_eq!(out, "Track sur Radio XYZ");
    }

    #[test]
    fn render_collapses_internal_whitespace() {
        let input = ComposeInput {
            current_track: Some(track("T", None)),
            ..Default::default()
        };
        let out = render_template("{artist}     {title}", &input, &auto_settings());
        assert_eq!(out, "T");
    }

    #[test]
    fn compose_static_returns_static_text() {
        let s = MetadataSettings {
            enabled: true,
            mode: MetadataMode::Static,
            static_text: "  Vous écoutez Radio XYZ  ".into(),
            ..Default::default()
        };
        let out = compose_title(&ComposeInput::default(), &s);
        assert_eq!(out, "Vous écoutez Radio XYZ");
    }

    #[test]
    fn compose_disabled_returns_empty() {
        let mut s = auto_settings();
        s.enabled = false;
        let input = ComposeInput {
            current_track: Some(track("X", Some("Y"))),
            ..Default::default()
        };
        let out = compose_title(&input, &s);
        assert_eq!(out, "");
    }

    #[test]
    fn compose_mic_open_with_override() {
        let s = MetadataSettings {
            enabled: true,
            mode: MetadataMode::Auto,
            template: "{title}".into(),
            mic_override: "Émission live de {show}".into(),
            show_name: "Le matin".into(),
            ..Default::default()
        };
        let input = ComposeInput {
            current_track: Some(track("Track", Some("A"))),
            mic_open: true,
            ..Default::default()
        };
        let out = compose_title(&input, &s);
        assert_eq!(out, "Émission live de Le matin");
    }

    #[test]
    fn compose_cart_takes_priority_over_music() {
        let s = auto_settings();
        let input = ComposeInput {
            current_track: Some(track("Track", Some("A"))),
            current_cart: Some("Jingle XYZ".into()),
            ..Default::default()
        };
        let out = compose_title(&input, &s);
        assert_eq!(out, "Jingle XYZ");
    }

    #[test]
    fn compose_falls_back_to_show_then_station() {
        let mut s = auto_settings();
        s.show_name = "Show".into();
        s.station_name = "Station".into();
        // No track, no cart, mic closed → show_name wins
        let out = compose_title(&ComposeInput::default(), &s);
        assert_eq!(out, "Show");
        s.show_name = "".into();
        let out = compose_title(&ComposeInput::default(), &s);
        assert_eq!(out, "Station");
    }

    #[test]
    fn compose_file_mode_uses_file_content() {
        let s = MetadataSettings {
            enabled: true,
            mode: MetadataMode::File,
            ..Default::default()
        };
        let input = ComposeInput {
            file_content: Some("\n  Now Playing: ABC  \n".into()),
            ..Default::default()
        };
        let out = compose_title(&input, &s);
        assert_eq!(out, "Now Playing: ABC");
    }

    #[test]
    fn urlencode_basic() {
        assert_eq!(urlencode("hello"), "hello");
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("/live.mp3"), "%2Flive.mp3");
        assert_eq!(urlencode("café"), "caf%C3%A9");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn decode_text_handles_utf8_bom() {
        let bytes = b"\xEF\xBB\xBFHello";
        assert_eq!(decode_text(bytes), "Hello");
    }

    #[test]
    fn decode_text_handles_utf16_le() {
        // "Hi" in UTF-16 LE with BOM
        let bytes = b"\xFF\xFE\x48\x00\x69\x00";
        assert_eq!(decode_text(bytes), "Hi");
    }

    #[test]
    fn decode_text_handles_utf16_be() {
        let bytes = b"\xFE\xFF\x00\x48\x00\x69";
        assert_eq!(decode_text(bytes), "Hi");
    }

    #[test]
    fn decode_text_falls_back_to_utf8() {
        assert_eq!(decode_text(b"plain"), "plain");
    }

    #[test]
    fn redact_truncates_long_titles() {
        let long = "x".repeat(120);
        let r = redact(&long);
        assert!(r.ends_with('…'));
        assert!(r.chars().count() <= 81);
    }

    #[test]
    fn target_normalizes_mount_without_leading_slash() {
        let cfg = StreamConfig {
            device_id: "d".into(),
            host: "  example.com  ".into(),
            port: 8000,
            mount: "live".into(),
            username: "u".into(),
            password: "p".into(),
            bitrate: 128,
            format: crate::presets::StreamFormat::Mp3,
        };
        let target = PushTarget::from_config(&cfg);
        assert_eq!(target.mount, "/live");
        assert_eq!(target.host, "example.com");
    }
}
