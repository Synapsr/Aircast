use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static FFMPEG_COUNTER: AtomicU64 = AtomicU64::new(0);
static ALIVE_FFMPEGS: AtomicU64 = AtomicU64::new(0);

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use tauri::AppHandle;

use crate::audio::capture::AudioFormat;
use crate::error::{AppError, AppResult};
use crate::presets::{StreamConfig, StreamFormat};
use crate::stream::ffmpeg_path;

const STDIN_QUEUE_CAPACITY: usize = 64;
const STDERR_TAIL_LINES: usize = 20;
const GRACEFUL_SHUTDOWN_TIMEOUT_MS: u64 = 1500;

#[derive(Default)]
pub struct FfmpegStatus {
    pub became_live: AtomicBool,
    pub last_error: Mutex<Option<String>>,
    pub stderr_tail: Mutex<VecDeque<String>>,
}

impl FfmpegStatus {
    pub fn last_error_message(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    pub fn tail(&self) -> String {
        self.stderr_tail
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Where ffmpeg writes the encoded stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderOutput {
    /// ffmpeg opens the connection itself and PUTs to `host:port/mount`.
    /// It owns the transport; we only feed it PCM and read its stderr.
    Icecast,
    /// ffmpeg writes the encoded elementary stream to stdout and we own the
    /// transport. Used by the webcast (WebSocket) transport, since ffmpeg has
    /// no WebSocket muxer.
    Pipe,
}

pub struct FfmpegProcess {
    id: u64,
    child: Child,
    stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
    stdout: Option<tokio::process::ChildStdout>,
    pub status: Arc<FfmpegStatus>,
}

impl Drop for FfmpegProcess {
    fn drop(&mut self) {
        let alive = ALIVE_FFMPEGS
            .fetch_sub(1, Ordering::SeqCst)
            .saturating_sub(1);
        log::info!("ffmpeg #{} dropped (alive: {})", self.id, alive);
    }
}

impl FfmpegProcess {
    pub fn spawn(
        app: &AppHandle,
        config: &StreamConfig,
        format: AudioFormat,
        output: EncoderOutput,
    ) -> AppResult<Self> {
        let binary = ffmpeg_path::resolve(app);
        log::info!("ffmpeg binary: {}", binary.display());
        let mut cmd = Command::new(&binary);
        cmd.args([
            "-hide_banner",
            "-loglevel",
            "info",
            "-nostats",
            "-f",
            "f32le",
            "-ar",
            &format.sample_rate.to_string(),
            "-ac",
            &format.channels.to_string(),
            "-i",
            "pipe:0",
        ]);

        for arg in codec_args(config, output) {
            cmd.arg(arg);
        }

        match output {
            EncoderOutput::Icecast => {
                // We use HTTP PUT to Icecast (2.4+) instead of the `icecast://`
                // protocol. The icecast demuxer in ffmpeg hard-codes a check that
                // rejects path "/" with "No mountpoint specified!", so any user with
                // a root mount couldn't stream. The HTTP protocol has no such check.
                cmd.arg("-method");
                cmd.arg("PUT");
                // `progress=continue` on stderr is what tells us the encoder is
                // actually pushing bytes to the server — see `is_progress_line`.
                cmd.arg("-progress");
                cmd.arg("pipe:2");
                cmd.arg(build_output_url(config));
            }
            EncoderOutput::Pipe => {
                // Deliberately no `-progress` here: in pipe mode ffmpeg produces
                // output as soon as it encodes, which says nothing about whether
                // the socket is healthy. The webcast transport sets `became_live`
                // itself, once a binary frame has actually been accepted.
                cmd.arg("pipe:1");
            }
        }

        cmd.stdin(Stdio::piped())
            .stdout(match output {
                EncoderOutput::Icecast => Stdio::null(),
                EncoderOutput::Pipe => Stdio::piped(),
            })
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::Stream(format_spawn_error(e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Stream("ffmpeg stdin not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Stream("ffmpeg stderr not piped".into()))?;
        let stdout = match output {
            EncoderOutput::Pipe => Some(
                child
                    .stdout
                    .take()
                    .ok_or_else(|| AppError::Stream("ffmpeg stdout not piped".into()))?,
            ),
            EncoderOutput::Icecast => None,
        };

        let (stdin_tx, stdin_rx) = mpsc::channel::<Vec<u8>>(STDIN_QUEUE_CAPACITY);
        let status = Arc::new(FfmpegStatus::default());

        tokio::spawn(stdin_pump(stdin, stdin_rx));
        tokio::spawn(stderr_reader(stderr, status.clone()));

        let id = FFMPEG_COUNTER.fetch_add(1, Ordering::SeqCst);
        let alive = ALIVE_FFMPEGS.fetch_add(1, Ordering::SeqCst) + 1;
        log::info!(
            "ffmpeg #{} spawned (alive: {}, pid: {:?})",
            id,
            alive,
            child.id()
        );
        Ok(Self {
            id,
            child,
            stdin_tx: Some(stdin_tx),
            stdout,
            status,
        })
    }

    pub fn stdin_sender(&self) -> Option<mpsc::Sender<Vec<u8>>> {
        self.stdin_tx.clone()
    }

    /// Hand the encoded-output pipe to the caller. Only `Some` when spawned
    /// with [`EncoderOutput::Pipe`], and only for the first call.
    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.stdout.take()
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    pub async fn shutdown(&mut self) {
        // close stdin → ffmpeg sees EOF and flushes
        self.stdin_tx.take();
        let timeout = Duration::from_millis(GRACEFUL_SHUTDOWN_TIMEOUT_MS);
        match tokio::time::timeout(timeout, self.child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
            }
        }
    }
}

async fn stdin_pump(mut stdin: tokio::process::ChildStdin, mut rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(buf) = rx.recv().await {
        if let Err(e) = stdin.write_all(&buf).await {
            log::warn!("ffmpeg stdin write failed: {e}");
            break;
        }
    }
    let _ = stdin.shutdown().await;
}

async fn stderr_reader(stderr: tokio::process::ChildStderr, status: Arc<FfmpegStatus>) {
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        // ffmpeg outputs many progress lines per second; raw lines stay at
        // debug so production runs don't spam. Errors that we classify get
        // promoted to error-level so they land in the diagnostic log even
        // at the default info filter.
        log::debug!("ffmpeg: {line}");

        {
            let mut tail = status.stderr_tail.lock().unwrap();
            if tail.len() >= STDERR_TAIL_LINES {
                tail.pop_front();
            }
            tail.push_back(line.clone());
        }

        if !status.became_live.load(Ordering::Relaxed) && is_progress_line(&line) {
            status.became_live.store(true, Ordering::Relaxed);
            log::info!("ffmpeg: live (encoder is producing output)");
        }

        if let Some(err) = classify_error(&line) {
            log::error!("ffmpeg classified error: {err} (raw: {line})");
            *status.last_error.lock().unwrap() = Some(err);
        }
    }
    log::info!("ffmpeg stderr stream closed");
}

fn is_progress_line(line: &str) -> bool {
    // ffmpeg `-progress pipe:2` emits key=value lines like:
    //   bitrate=128.0kbits/s
    //   total_size=...
    //   out_time_us=...
    //   progress=continue
    // We treat the first "progress=continue" as the "live" signal.
    line.starts_with("progress=") && line.contains("continue")
}

fn classify_error(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if lower.contains("connection refused") {
        Some("Connection refused — the server isn't accepting connections.".into())
    } else if lower.contains("network is unreachable") || lower.contains("no route to host") {
        Some("Network unreachable — check your internet connection.".into())
    } else if lower.contains("name or service not known") || lower.contains("could not resolve") {
        Some("Server address not found — check the host.".into())
    } else if lower.contains("401") || lower.contains("unauthorized") {
        Some("Authentication failed — check your username and password.".into())
    } else if lower.contains("403") {
        Some("Server refused the connection — the mount may already be in use.".into())
    } else if lower.contains("404") {
        Some("Mount point not found on the server.".into())
    } else if lower.contains("connection timed out")
        // Windows UCRT: ETIMEDOUT = 138 (Linux: 110). ffmpeg prints the raw
        // errno from ff_neterrno() rather than the textual reason on Windows,
        // so we match both the numeric form and the generic TCP-failure line
        // it logs just before (e.g. "[tcp @ ...] Connection to tcp://host:port
        // failed: Error number -138 occurred").
        || lower.contains("error number -138")
        || lower.contains("error number -110")
        || (lower.contains("connection to tcp") && lower.contains("failed"))
    {
        Some(
            "Connection timed out — check your firewall, antivirus, or that the streaming port isn't blocked on this network."
                .into(),
        )
    } else {
        None
    }
}

/// Errors that won't fix themselves on retry (bad credentials, missing mount, …).
/// The pipeline uses this to abort the reconnect loop and surface the issue
/// instead of looping forever.
///
/// This only governs the Icecast transport, where ffmpeg's English stderr is
/// the only signal available. The webcast transport knows the answer exactly —
/// from the close code and the HTTP status — and carries it explicitly rather
/// than encoding it in wording and hoping this matcher agrees.
pub fn is_fatal_error(message: &str) -> bool {
    let m = message.to_lowercase();
    m.contains("authentication failed")
        || m.contains("mount point not found")
        || m.contains("server address not found")
        || m.contains("server refused the connection")
}

/// The MIME type that describes what the encoder produces.
///
/// Doubles as the `mime` field of the webcast hello frame. Liquidsoap's harbor
/// truncates it at the first `;` and uses it purely to select a decoder, then
/// libavformat re-probes the actual bytes — so these two values, which are
/// both registered decoder MIMEs, are all the transport needs. Never send a
/// MIME the harbor does not know: it raises `Unknown_codec` *after* claiming
/// the mount, which can leave the mount unusable until the station restarts.
pub fn mime_for(format: &StreamFormat) -> &'static str {
    match format {
        StreamFormat::Mp3 => "audio/mpeg",
        StreamFormat::Aac => "audio/aac",
    }
}

fn codec_args(config: &StreamConfig, output: EncoderOutput) -> Vec<String> {
    let (codec, container) = match config.format {
        StreamFormat::Mp3 => ("libmp3lame", "mp3"),
        StreamFormat::Aac => ("aac", "adts"),
    };

    let mut args: Vec<String> = vec![
        "-codec:a".into(),
        codec.into(),
        "-b:a".into(),
        format!("{}k", config.bitrate),
    ];

    match output {
        EncoderOutput::Icecast => {
            // `-content_type` is an option of the http muxer, so it is only
            // valid when ffmpeg owns the connection.
            args.push("-content_type".into());
            args.push(mime_for(&config.format).into());
            args.push("-f".into());
            args.push(container.into());
        }
        EncoderOutput::Pipe => {
            args.push("-f".into());
            args.push(container.into());
            if container == "mp3" {
                // Emit a bare MP3 elementary stream: no ID3v2 header and no
                // Xing/LAME frame. Both are written up-front and would be
                // relayed as if they were audio, and the Xing frame in
                // particular is a silent frame that only makes sense for a
                // seekable file.
                args.push("-id3v2_version".into());
                args.push("0".into());
                args.push("-write_xing".into());
                args.push("0".into());
            }
            // Do not let the muxer sit on a packet: this is a live stream and
            // latency matters more than write syscalls.
            args.push("-flush_packets".into());
            args.push("1".into());
        }
    }

    args
}

/// Build the HTTP URL ffmpeg streams to via PUT. Icecast 2.4+ accepts source
/// clients over HTTP PUT with Basic auth — the user:pass embedded in the URL
/// is what ffmpeg's http protocol uses to set the `Authorization` header.
fn build_output_url(config: &StreamConfig) -> String {
    let mount_normalized = if config.mount.starts_with('/') {
        config.mount.clone()
    } else {
        format!("/{}", config.mount)
    };
    format!(
        "http://{}:{}@{}:{}{}",
        urlencode(&config.username),
        urlencode(&config.password),
        config.host.trim(),
        config.port,
        urlencode_path(&mount_normalized),
    )
}

/// Percent-encode the unreserved-set complement (RFC 3986). Used for
/// userinfo (username/password) which mustn't contain `:`, `@`, or `/`.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Percent-encode a URL path while preserving `/` separators. Each segment
/// gets its individual characters encoded. Avoids breaking the URL when the
/// mount contains spaces, `?`, `&`, `#`, or other unsafe characters.
fn urlencode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if is_unreserved(b) || b == b'/' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[inline]
fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

fn format_spawn_error(e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        "ffmpeg binary not found. Install ffmpeg and ensure it's in your PATH.".into()
    } else {
        format!("spawn ffmpeg: {e}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{StreamConfig, StreamFormat};

    fn cfg(host: &str, mount: &str, user: &str, pass: &str, format: StreamFormat) -> StreamConfig {
        StreamConfig {
            device_id: "dev".into(),
            host: host.into(),
            port: 8000,
            mount: mount.into(),
            username: user.into(),
            password: pass.into(),
            bitrate: 128,
            format,
            transport: crate::presets::Transport::Icecast,
        }
    }

    // ───────── classify_error ─────────

    #[test]
    fn classify_connection_refused() {
        assert_eq!(
            classify_error("Server returned 0: Connection refused"),
            Some("Connection refused — the server isn't accepting connections.".into())
        );
    }

    #[test]
    fn classify_401_unauthorized() {
        assert_eq!(
            classify_error("HTTP error 401 Unauthorized"),
            Some("Authentication failed — check your username and password.".into())
        );
    }

    #[test]
    fn classify_403_forbidden() {
        assert_eq!(
            classify_error("Server returned 403"),
            Some("Server refused the connection — the mount may already be in use.".into())
        );
    }

    #[test]
    fn classify_404_not_found() {
        assert_eq!(
            classify_error("Server returned 404 Not Found"),
            Some("Mount point not found on the server.".into())
        );
    }

    #[test]
    fn classify_dns_failure() {
        assert!(classify_error("Could not resolve host: example.invalid").is_some());
        assert!(classify_error("Name or service not known").is_some());
    }

    #[test]
    fn classify_timeout() {
        let expected = "Connection timed out — check your firewall, antivirus, or that the streaming port isn't blocked on this network.";
        assert_eq!(
            classify_error("Connection timed out"),
            Some(expected.into())
        );
    }

    #[test]
    fn classify_windows_tcp_etimedout() {
        // Real-world stderr line from a Windows user with port 8255 filtered.
        let line = "[tcp @ 00000256c8472a00] Connection to tcp://stream.example.com:8255 failed: Error number -138 occurred";
        assert!(classify_error(line).is_some());
    }

    #[test]
    fn classify_linux_tcp_etimedout() {
        let line =
            "[tcp @ 0x12345] Connection to tcp://server:8000 failed: Error number -110 occurred";
        assert!(classify_error(line).is_some());
    }

    #[test]
    fn classify_no_route_to_host() {
        assert_eq!(
            classify_error("No route to host"),
            Some("Network unreachable — check your internet connection.".into())
        );
    }

    #[test]
    fn classify_unknown_returns_none() {
        assert!(classify_error("size=    1024kB time=00:00:01.00 bitrate=...").is_none());
        assert!(classify_error("Press [q] to stop, [?] for help").is_none());
    }

    #[test]
    fn classify_is_case_insensitive() {
        assert!(classify_error("UNAUTHORIZED").is_some());
        assert!(classify_error("connection REFUSED").is_some());
    }

    // ───────── is_fatal_error ─────────

    #[test]
    fn fatal_includes_auth_and_mount_errors() {
        assert!(is_fatal_error(
            "Authentication failed — check your username and password."
        ));
        assert!(is_fatal_error("Mount point not found on the server."));
        assert!(is_fatal_error("Server address not found — check the host."));
        assert!(is_fatal_error(
            "Server refused the connection — the mount may already be in use."
        ));
    }

    #[test]
    fn fatal_excludes_transient_errors() {
        assert!(!is_fatal_error(
            "Connection refused — the server isn't accepting connections."
        ));
        assert!(!is_fatal_error(
            "Connection timed out — the server isn't responding."
        ));
        assert!(!is_fatal_error("Network unreachable."));
    }

    // ───────── urlencode ─────────

    #[test]
    fn urlencode_passes_through_safe_ascii() {
        assert_eq!(urlencode("source"), "source");
        assert_eq!(urlencode("AbC123"), "AbC123");
        assert_eq!(urlencode("a-b.c_d~e"), "a-b.c_d~e");
    }

    #[test]
    fn urlencode_escapes_special_chars() {
        assert_eq!(urlencode("p@ss"), "p%40ss");
        assert_eq!(urlencode("space here"), "space%20here");
        assert_eq!(urlencode("a/b"), "a%2Fb");
        assert_eq!(urlencode("a:b"), "a%3Ab");
        assert_eq!(urlencode("a?b"), "a%3Fb");
        assert_eq!(urlencode("a&b"), "a%26b");
    }

    #[test]
    fn urlencode_handles_utf8() {
        // é is two bytes in UTF-8 (C3 A9)
        assert_eq!(urlencode("café"), "caf%C3%A9");
    }

    // ───────── build_output_url ─────────

    #[test]
    fn output_url_basic_mp3() {
        let c = cfg(
            "example.com",
            "/live.mp3",
            "source",
            "secret",
            StreamFormat::Mp3,
        );
        assert_eq!(
            build_output_url(&c),
            "http://source:secret@example.com:8000/live.mp3"
        );
    }

    #[test]
    fn output_url_normalizes_mount_without_leading_slash() {
        let c = cfg(
            "example.com",
            "live.mp3",
            "source",
            "secret",
            StreamFormat::Mp3,
        );
        assert_eq!(
            build_output_url(&c),
            "http://source:secret@example.com:8000/live.mp3"
        );
    }

    #[test]
    fn output_url_encodes_credentials() {
        let c = cfg(
            "example.com",
            "/live",
            "us er",
            "p@ss/word",
            StreamFormat::Mp3,
        );
        assert_eq!(
            build_output_url(&c),
            "http://us%20er:p%40ss%2Fword@example.com:8000/live"
        );
    }

    #[test]
    fn output_url_empty_password_is_safe() {
        let c = cfg("example.com", "/live", "source", "", StreamFormat::Mp3);
        assert_eq!(build_output_url(&c), "http://source:@example.com:8000/live");
    }

    #[test]
    fn output_url_root_mount_works() {
        // The whole point of HTTP PUT: "/" is a valid path here, unlike
        // ffmpeg's icecast:// protocol which rejects it.
        let c = cfg("example.com", "/", "source", "secret", StreamFormat::Mp3);
        assert_eq!(
            build_output_url(&c),
            "http://source:secret@example.com:8000/"
        );
    }

    #[test]
    fn output_url_encodes_mount_special_chars_but_preserves_slashes() {
        let c = cfg(
            "example.com",
            "/live/my channel?",
            "source",
            "secret",
            StreamFormat::Mp3,
        );
        let url = build_output_url(&c);
        // `/` preserved, space → %20, `?` → %3F
        assert!(url.contains("/live/my%20channel%3F"));
    }

    #[test]
    fn output_url_trims_whitespace_from_host() {
        let c = cfg(
            "  example.com  ",
            "/live",
            "source",
            "secret",
            StreamFormat::Mp3,
        );
        assert_eq!(
            build_output_url(&c),
            "http://source:secret@example.com:8000/live"
        );
    }

    // ───────── codec_args ─────────

    #[test]
    fn codec_args_mp3_uses_libmp3lame() {
        let c = cfg("h", "/m", "u", "p", StreamFormat::Mp3);
        let args = codec_args(&c, EncoderOutput::Icecast);
        assert!(args.contains(&"libmp3lame".to_string()));
        assert!(args.contains(&"audio/mpeg".to_string()));
        assert!(args.contains(&"mp3".to_string()));
        assert!(args.contains(&"128k".to_string()));
    }

    #[test]
    fn codec_args_aac_uses_native_aac() {
        let c = cfg("h", "/m", "u", "p", StreamFormat::Aac);
        let args = codec_args(&c, EncoderOutput::Icecast);
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"audio/aac".to_string()));
        assert!(args.contains(&"adts".to_string()));
    }

    // ───────── is_progress_line ─────────

    #[test]
    fn progress_line_recognizes_continue_marker() {
        assert!(is_progress_line("progress=continue"));
    }

    #[test]
    fn progress_line_rejects_other_lines() {
        assert!(!is_progress_line("progress=end"));
        assert!(!is_progress_line("size=1024kB"));
        assert!(!is_progress_line(""));
    }
}
