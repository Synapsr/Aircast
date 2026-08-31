//! The "webcast" transport — live audio over a WebSocket instead of the
//! Icecast source protocol.
//!
//! ## Why this exists
//!
//! `SOURCE /mount HTTP/1.0` carries neither `Content-Length` nor
//! `Transfer-Encoding`, so every reverse proxy on the path reads it as a
//! request with no body, forwards it bodyless, and then chokes on the audio
//! bytes that follow. That makes the Icecast source protocol impossible to
//! tunnel through a shared `:443`, which is the only port open on most school
//! and corporate networks.
//!
//! A WebSocket has no such problem: after the `101 Switching Protocols` the
//! connection stops being HTTP and becomes an opaque tunnel that every proxy
//! already knows how to pass through.
//!
//! AzuraCast's own Web DJ uses exactly this path, and the route is generated
//! automatically for every station, so **no server-side change is required**.
//!
//! ## The protocol (Liquidsoap 2.4.5 `input.harbor`)
//!
//! 1. `GET <mount>` with `Upgrade: websocket` and
//!    `Sec-WebSocket-Protocol: webcast`. Both values are compared
//!    **case-sensitively against exact lowercase strings** — anything else
//!    falls through to the plain-HTTP handler and answers 404.
//! 2. One text frame: `{"type":"hello","data":{"mime","user","password"}}`.
//!    All three keys are mandatory.
//! 3. Binary frames carrying the raw encoder output. They are appended to one
//!    buffer and drained byte-wise, so frame boundaries carry no meaning and
//!    arbitrary chunking is safe.
//! 4. Optional text frames: `{"type":"metadata","data":{"title","artist"}}`.
//!
//! ## Things that will bite you
//!
//! - **Success is silent.** The harbor writes nothing at all when the hello is
//!   accepted; the 101 is sent *before* the hello is even read. Failures, on
//!   the other hand, do arrive as real close frames — see [`close_to_message`].
//! - **Never send a zero-length binary frame.** The harbor's read returns 0,
//!   which ocaml-ffmpeg maps to `AVERROR_EOF`: one empty frame ends the
//!   broadcast.
//! - **Never send an unknown MIME.** `Unknown_codec` is raised *after* the
//!   mount has been claimed, and nothing resets it — it can leave the mount
//!   unusable until the station restarts. See [`super::ffmpeg::mime_for`].
//! - **Pace at real time.** There is no backpressure: the harbor's generator
//!   drops the *oldest* audio once its buffer overflows.
//! - **Ping is not a probe.** The harbor discards every control frame and
//!   never answers. Pings are sent only to stop intermediate proxies idling
//!   the connection out.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use futures_util::{Sink, SinkExt, StreamExt};
use tauri::{AppHandle, Manager};
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message, WebSocketConfig};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

use crate::audio::capture::AudioFormat;
use crate::presets::StreamConfig;
use crate::state::{AppState, CaptureContext};
use crate::stream::ffmpeg::{mime_for, EncoderOutput, FfmpegProcess, FfmpegStatus};
use crate::stream::status::{emit, StreamStatus};

/// Target payload per binary frame. ~4 KiB is roughly 250 ms of 128 kbps MP3.
///
/// The browser Web DJ sends 1-second chunks only because `MediaRecorder`
/// cannot do better; there is no reason to copy that. Sending each 417-byte
/// MP3 frame on its own would cost ~2% WebSocket framing plus ~6% TLS record
/// overhead and 38 syscalls a second.
const TARGET_CHUNK_BYTES: usize = 4096;

/// Flush the accumulator at least this often, so a low bitrate does not sit
/// waiting to reach [`TARGET_CHUNK_BYTES`].
const MAX_CHUNK_INTERVAL: Duration = Duration::from_millis(250);

/// Bounded queue between the encoder reader and the socket writer.
/// 64 × 4 KiB ≈ 16 s of 128 kbps audio.
const AUDIO_QUEUE_DEPTH: usize = 64;

/// Cap tungstenite's internal write buffer. It defaults to `usize::MAX`, so a
/// stalled socket would absorb an unbounded backlog of audio in RAM. Past this
/// we want an error and a reconnect, not silent growth.
const MAX_WRITE_BUFFER: usize = 256 * 1024;

/// Sent purely to keep intermediate proxies from idling the connection out
/// during a silent passage (Traefik's `idleTimeout` defaults to 180 s). The
/// harbor never answers, so this is not a liveness probe — but any readable
/// frame does reset the harbor's own 30 s data timeout.
const KEEPALIVE: Duration = Duration::from_secs(20);

/// How long the whole attempt may take to reach the live state.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Budget for getting one frame out onto the wire.
///
/// This is load-bearing, not belt-and-braces. `SinkExt::send` is
/// poll_ready + start_send + poll_flush-to-completion, and tungstenite maps a
/// `WouldBlock` from the socket into `Poll::Pending`. A peer that ACKs but
/// stops reading — a stalled proxy in front of the harbor, exactly the
/// deployment this transport targets — produces no RTO and no error, so an
/// unbounded `send` pends forever. Because the sends happen inside
/// `select!` arm *bodies*, that would park the entire relay loop, including
/// the branch that watches for Stop: the user's Stop button would hang and
/// take the stream mutex with it.
///
/// At 128 kbps a 4 KiB frame flushes in milliseconds, so anything near this
/// budget means the link is gone.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// The whole close sequence — drain, flush, close frame, handshake — gets one
/// budget. Every step of it can pend on a socket that will not drain.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

/// If the encoder produces nothing for this long the session is broken, and
/// pinging would only keep the harbor holding the mount while listeners hear
/// dead air. Must stay below the harbor's own 30 s data timeout so we notice
/// first and can reconnect.
const AUDIO_STALL_TIMEOUT: Duration = Duration::from_secs(15);

/// The harbor sends nothing when the hello is accepted, so — exactly like
/// AzuraCast's own Web DJ — we wait a moment before believing it. A rejection
/// arrives well inside this window.
const HELLO_GRACE: Duration = Duration::from_millis(1000);

/// Shared slot holding the metadata sender of the currently-connected session.
///
/// The metadata updater is deliberately decoupled from the stream: it holds a
/// long-lived `PushTarget` and knows nothing about connection lifecycles. This
/// slot bridges the two — the transport fills it when the socket comes up and
/// clears it when the socket goes down, mirroring how
/// [`CaptureContext::stream_tx`] bridges the capture callback to the encoder.
#[derive(Clone, Default)]
pub struct MetadataSink(Arc<Mutex<Option<mpsc::Sender<String>>>>);

impl MetadataSink {
    pub fn new() -> Self {
        Self::default()
    }

    fn set(&self, tx: Option<mpsc::Sender<String>>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = tx;
        }
    }

    /// Queue a title for the live session. `Err` when no session is connected
    /// or the queue is full — both are "try again next tick", not failures
    /// worth surfacing.
    pub fn try_send(&self, title: &str) -> Result<(), String> {
        let guard = self
            .0
            .lock()
            .map_err(|_| "metadata sink poisoned".to_string())?;
        let tx = guard
            .as_ref()
            .ok_or_else(|| "no live webcast session".to_string())?;
        tx.try_send(title.to_string())
            .map_err(|e| format!("webcast metadata queue: {e}"))
    }
}

impl std::fmt::Debug for MetadataSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let connected = self.0.lock().map(|g| g.is_some()).unwrap_or(false);
        f.debug_struct("MetadataSink")
            .field("connected", &connected)
            .finish()
    }
}

/// How one connection attempt ended. Mirrors the pipeline's own outcome type.
pub enum SessionEnd {
    Stopped,
    Failed {
        message: String,
        details: Option<String>,
        /// Whether retrying is pointless.
        ///
        /// The Icecast path infers this by substring-matching English prose
        /// from ffmpeg. That is fine there — ffmpeg's messages are the only
        /// signal available — but here we know the answer exactly, from the
        /// close code and the HTTP status, so we say it rather than encoding
        /// it in wording and hoping the matcher agrees.
        fatal: bool,
    },
}

/// A message plus whether retrying it could ever help.
#[derive(Debug)]
pub(crate) struct Failure {
    message: String,
    fatal: bool,
}

impl Failure {
    fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: false,
        }
    }
    fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: true,
        }
    }
    fn into_end(self, details: Option<String>) -> SessionEnd {
        SessionEnd::Failed {
            message: self.message,
            details,
            fatal: self.fatal,
        }
    }
}

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Run one full attempt: spawn the encoder, open the socket, relay until
/// something ends it.
pub async fn run_attempt(
    app: &AppHandle,
    config: &StreamConfig,
    format: AudioFormat,
    capture_ctx: &CaptureContext,
    meta_sink: &MetadataSink,
    stop_rx: &mut oneshot::Receiver<()>,
) -> SessionEnd {
    let mut ffmpeg = match FfmpegProcess::spawn(app, config, format, EncoderOutput::Pipe) {
        Ok(f) => f,
        Err(e) => {
            // A missing or unspawnable ffmpeg will not fix itself.
            return Failure::fatal(e.to_string()).into_end(None);
        }
    };

    let stdin_tx = match ffmpeg.stdin_sender() {
        Some(tx) => tx,
        None => return Failure::fatal("ffmpeg stdin missing").into_end(None),
    };
    let stdout = match ffmpeg.take_stdout() {
        Some(o) => o,
        None => return Failure::fatal("ffmpeg stdout missing").into_end(None),
    };
    let status = ffmpeg.status.clone();

    let url = config.webcast_url();
    log::info!("webcast: connecting to {url}");

    // Deliberately connect BEFORE attaching the encoder to the capture loop.
    // Nothing drains ffmpeg's stdout until `pump_encoder` starts, so any audio
    // encoded during the handshake would sit in the 64 KiB stdout pipe and be
    // burst onto the socket the moment the pump begins — the harbor plays that
    // out at real time, so the whole broadcast would run permanently behind by
    // however long the handshake took.
    let socket = tokio::select! {
        _ = &mut *stop_rx => {
            ffmpeg.shutdown().await;
            return SessionEnd::Stopped;
        }
        result = connect(config, &url) => match result {
            Ok(s) => s,
            Err(failure) => {
                let tail = status.tail();
                ffmpeg.shutdown().await;
                return failure.into_end(if tail.is_empty() { None } else { Some(tail) });
            }
        }
    };

    log::info!("webcast: upgraded, hello sent");

    let (audio_tx, audio_rx) = mpsc::channel::<Bytes>(AUDIO_QUEUE_DEPTH);
    let (meta_tx, meta_rx) = mpsc::channel::<String>(8);
    meta_sink.set(Some(meta_tx));

    let pump = tokio::spawn(pump_encoder(stdout, audio_tx));
    capture_ctx.set_stream_tx(Some(stdin_tx));

    let outcome = relay(
        app,
        socket,
        audio_rx,
        meta_rx,
        &status,
        &mut ffmpeg,
        stop_rx,
    )
    .await;

    capture_ctx.set_stream_tx(None);
    pump.abort();
    meta_sink.set(None);
    let tail = status.tail();
    ffmpeg.shutdown().await;

    match outcome {
        SessionEnd::Failed {
            message,
            details,
            fatal,
        } => SessionEnd::Failed {
            message,
            details: details.or(if tail.is_empty() { None } else { Some(tail) }),
            fatal,
        },
        other => other,
    }
}

/// Open the socket and send the hello frame.
pub(crate) async fn connect(config: &StreamConfig, url: &str) -> Result<Socket, Failure> {
    let request = build_request(url)?;

    let ws_config = WebSocketConfig::default()
        // We coalesce chunks ourselves, so tungstenite must not add a second
        // layer of buffering delay.
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_WRITE_BUFFER)
        // The harbor never sends anything but close frames, so nothing large
        // should ever arrive.
        .max_message_size(Some(64 * 1024))
        .max_frame_size(Some(64 * 1024));

    let connector = Connector::NativeTls(
        native_tls::TlsConnector::builder()
            .build()
            .map_err(|e| Failure::fatal(format!("Could not initialise TLS: {e}")))?,
    );

    let connect_fut =
        connect_async_tls_with_config(request, Some(ws_config), true, Some(connector));

    let (mut socket, _response) = match tokio::time::timeout(CONNECT_TIMEOUT, connect_fut).await {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => return Err(handshake_failure(e)),
        Err(_) => {
            return Err(Failure::retryable(
                "Connection timed out — check your firewall, antivirus, or that the streaming port isn't blocked on this network.",
            ))
        }
    };

    // All three keys are mandatory: the harbor does a plain lookup for each and
    // answers Close(1002, "Invalid hello.") if any is missing.
    let hello = serde_json::json!({
        "type": "hello",
        "data": {
            "mime": mime_for(&config.format),
            "user": config.username.trim(),
            "password": config.password,
        }
    });

    send_bounded(&mut socket, Message::Text(hello.to_string().into()), false)
        .await
        .map_err(|f| Failure {
            message: format!("Could not send the handshake: {}", f.message),
            fatal: f.fatal,
        })?;

    Ok(socket)
}

/// Build the client handshake. The subprotocol is the whole trick.
fn build_request(url: &str) -> Result<Request, Failure> {
    let mut request = url.into_client_request().map_err(|e| {
        // A URL the client cannot even parse is a configuration mistake; no
        // amount of retrying turns it into a valid one.
        Failure::fatal(format!("Invalid server address ({url}): {e}"))
    })?;

    // Liquidsoap compares this value with an exact, case-sensitive string
    // match. A subprotocol *list* ("webcast, chat") is rejected, and the
    // request then falls through to the plain-HTTP handler and 404s.
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_static("webcast"),
    );
    request.headers_mut().insert(
        "User-Agent",
        HeaderValue::from_static(concat!("Aircast/", env!("CARGO_PKG_VERSION"))),
    );
    Ok(request)
}

/// Send one message with a deadline.
///
/// See [`WRITE_TIMEOUT`] for why an unbounded `send` is not an option here.
async fn send_bounded<S>(sink: &mut S, message: Message, live: bool) -> Result<(), Failure>
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    match tokio::time::timeout(WRITE_TIMEOUT, sink.send(message)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(transport_failure(&e, live)),
        Err(_) => Err(Failure::retryable(format!(
            "The connection stalled — nothing could be sent for {}s.",
            WRITE_TIMEOUT.as_secs()
        ))),
    }
}

/// Read the encoder's stdout, coalesce it into frame-sized chunks, and hand
/// them to the socket writer over a bounded channel.
///
/// Backpressure policy: never block, never grow without bound. If the socket
/// falls behind we drop, because the wall clock does not stop — audio we could
/// not send three seconds ago is worthless now. Blocking here would be far
/// worse than dropping: ffmpeg's 64 KiB stdout pipe would fill, ffmpeg would
/// stop draining its stdin, and the cpal callback writing into that stdin
/// would block — glitching capture, monitoring and the local mix, not just the
/// broadcast.
async fn pump_encoder(mut stdout: tokio::process::ChildStdout, tx: mpsc::Sender<Bytes>) {
    let mut acc = BytesMut::with_capacity(TARGET_CHUNK_BYTES * 2);
    let mut read_buf = vec![0u8; 8192];
    let mut ticker = tokio::time::interval(MAX_CHUNK_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut dropped: u64 = 0;

    loop {
        let should_flush = tokio::select! {
            biased;
            read = stdout.read(&mut read_buf) => match read {
                Ok(0) => {
                    // Encoder exited. Flush the tail; the relay loop notices
                    // the process died and classifies it.
                    if !acc.is_empty() {
                        let _ = tx.try_send(acc.split().freeze());
                    }
                    return;
                }
                Ok(n) => {
                    acc.extend_from_slice(&read_buf[..n]);
                    acc.len() >= TARGET_CHUNK_BYTES
                }
                Err(e) => {
                    log::warn!("webcast: reading encoder output failed: {e}");
                    return;
                }
            },
            _ = ticker.tick() => !acc.is_empty(),
        };

        if !should_flush {
            continue;
        }

        // Never send an empty binary frame: the harbor maps a zero-length read
        // to EOF and ends the broadcast.
        let chunk = acc.split().freeze();
        if chunk.is_empty() {
            continue;
        }

        match tx.try_send(chunk) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                dropped += 1;
                if dropped % 16 == 1 {
                    log::warn!("webcast: socket behind, dropped {dropped} chunk(s)");
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => return,
        }
    }
}

/// The socket loop: audio out, metadata out, keepalive, and — crucially — the
/// read half, which is the only place close frames and transport errors
/// surface.
#[allow(clippy::too_many_arguments)]
async fn relay(
    app: &AppHandle,
    socket: Socket,
    mut audio_rx: mpsc::Receiver<Bytes>,
    mut meta_rx: mpsc::Receiver<String>,
    status: &Arc<FfmpegStatus>,
    ffmpeg: &mut FfmpegProcess,
    stop_rx: &mut oneshot::Receiver<()>,
) -> SessionEnd {
    let (mut sink, mut stream) = socket.split();
    let mut keepalive = tokio::time::interval(KEEPALIVE);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick of a tokio interval fires immediately; we do not want a
    // ping before any audio.
    keepalive.tick().await;

    let started = Instant::now();
    let mut last_audio = Instant::now();
    let mut live = false;
    let mut sent_any = false;
    let mut live_check = tokio::time::interval(Duration::from_millis(200));
    live_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;

            _ = &mut *stop_rx => {
                // One budget for the whole close sequence. Every step of it —
                // drain, flush, close frame, handshake — can pend on a socket
                // that will not drain, and Stop must always return.
                let _ = tokio::time::timeout(
                    CLOSE_TIMEOUT,
                    graceful_close(&mut sink, &mut audio_rx),
                )
                .await;
                return SessionEnd::Stopped;
            }

            // The harbor stays silent on success, but this is how
            // Close(1002/1011) and transport errors reach us — and it is what
            // lets tungstenite flush its automatic Pong replies.
            inbound = stream.next() => match inbound {
                Some(Ok(Message::Close(frame))) => {
                    return close_failure(frame.as_ref()).into_end(None);
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => return transport_failure(&e, live).into_end(None),
                None => return abrupt_end_failure(live).into_end(None),
            },

            // The encoder dying is a distinct failure from the socket dying,
            // and its stderr usually says exactly why.
            exit = ffmpeg.wait() => {
                let message = status.last_error_message().unwrap_or_else(|| {
                    format!("The encoder stopped unexpectedly ({:?}).", exit.ok())
                });
                return Failure::retryable(message).into_end(None);
            }

            Some(title) = meta_rx.recv() => {
                // Every value must be a JSON string: the harbor calls a
                // string-only accessor on each one, and a number would raise
                // inside the socket read loop and drop the broadcast.
                //
                // Only `title` and `artist` survive AzuraCast's allow-list, and
                // the four ID fields it also accepts are client-injectable — a
                // bogus `media_id` makes it reject the update outright. So we
                // send exactly these two, with the composed string as the title
                // (same shape as the Icecast `song=` field).
                let frame = serde_json::json!({
                    "type": "metadata",
                    "data": { "title": title, "artist": "" }
                });
                if let Err(f) = send_bounded(&mut sink, Message::Text(frame.to_string().into()), live).await {
                    return f.into_end(None);
                }
            }

            Some(chunk) = audio_rx.recv() => {
                if let Err(f) = send_bounded(&mut sink, Message::Binary(chunk), live).await {
                    return f.into_end(None);
                }
                sent_any = true;
                last_audio = Instant::now();
            }

            _ = keepalive.tick() => {
                // A ping resets the harbor's 30 s data timeout just as audio
                // does. Pinging through an audio drought would therefore hold
                // the mount open while listeners hear nothing — so check the
                // encoder is still producing before reaching for the keepalive.
                if last_audio.elapsed() > AUDIO_STALL_TIMEOUT {
                    return Failure::retryable(format!(
                        "The encoder stopped producing audio for {}s.",
                        AUDIO_STALL_TIMEOUT.as_secs()
                    ))
                    .into_end(None);
                }
                if let Err(f) = send_bounded(&mut sink, Message::Ping(Bytes::new()), live).await {
                    return f.into_end(None);
                }
            }

            _ = live_check.tick() => {
                if !live && sent_any && started.elapsed() >= HELLO_GRACE {
                    live = true;
                    status.became_live.store(true, Ordering::Relaxed);
                    log::info!("webcast: live (audio accepted, no rejection)");
                    // Clear any leftover error now that we're healthy again,
                    // mirroring what the Icecast path does in `pipeline`.
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(mut slot) = state.last_stream_error.lock() {
                            *slot = None;
                        }
                    }
                    emit(app, StreamStatus::Live);
                }
                if !live && started.elapsed() >= CONNECT_TIMEOUT {
                    return Failure::retryable(format!(
                        "Connection timed out — the server didn't accept the stream within {}s.",
                        CONNECT_TIMEOUT.as_secs()
                    ))
                    .into_end(None);
                }
            }
        }
    }
}

/// Drain what is already queued, then close cleanly. The close frame makes
/// Liquidsoap fire `on_disconnect` immediately rather than waiting for its own
/// timeout, so the station falls back to AutoDJ without a gap of silence.
///
/// Every await here can pend forever on a dead socket, so the caller runs the
/// whole thing under one timeout.
async fn graceful_close<S>(sink: &mut S, audio_rx: &mut mpsc::Receiver<Bytes>)
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    // Bound the drain up front: the pump may still be producing, and we must
    // not let it keep this loop alive.
    let queued = audio_rx.len();
    for _ in 0..queued {
        match audio_rx.try_recv() {
            Ok(chunk) => {
                if sink.feed(Message::Binary(chunk)).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = sink.flush().await;
    let _ = sink
        .send(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: "".into(),
        })))
        .await;
    let _ = sink.close().await;
}

/// Map a close frame to a message and a retry decision.
fn close_failure(frame: Option<&CloseFrame>) -> Failure {
    let Some(frame) = frame else {
        return Failure::retryable("The server closed the stream without saying why.");
    };
    let reason = frame.reason.as_str();

    if reason.contains("Authentication failed") {
        Failure::fatal("Authentication failed — check your username and password.")
    } else if reason.contains("Invalid hello") {
        Failure::fatal(
            "The server rejected the connection request — this looks like a bug in Aircast, please report it.",
        )
    } else if reason.contains("mountpoint") {
        // Retryable on purpose: the harbor allows one source per mount, and on
        // a reconnect the server may not have noticed the previous socket died
        // yet. Giving up here would strand a DJ who simply reconnected too fast.
        Failure::retryable(
            "The mount point is busy or unavailable — another DJ may already be connected.",
        )
    } else if reason.is_empty() {
        Failure::retryable(format!(
            "The server closed the stream (code {}).",
            u16::from(frame.code)
        ))
    } else {
        Failure::retryable(format!(
            "The server closed the stream: {reason} (code {}).",
            u16::from(frame.code)
        ))
    }
}

/// The socket ended without a close frame.
///
/// Before we ever went live, this is the harbor refusing the stream *after*
/// accepting the hello — either the mount was already held by another source,
/// or the audio format was rejected. Both are raised from a code path that
/// sends no close frame, so a bare disconnect is all we get and the two are
/// indistinguishable on the wire.
///
/// It stays retryable, because the far more common cause by a wide margin is a
/// busy mount — typically our own previous session not yet torn down. Aircast
/// only ever offers the harbor two MIME types, both verified to decode, so the
/// format branch is not reachable from a correctly built client.
fn abrupt_end_failure(live: bool) -> Failure {
    if live {
        Failure::retryable("The connection to the server was lost.")
    } else {
        Failure::retryable(
            "The server accepted the login but then closed the stream — the mount point may already be in use.",
        )
    }
}

fn transport_failure(e: &WsError, live: bool) -> Failure {
    match e {
        // A peer that vanishes mid-stream reaches us as one of these three,
        // depending on whether it sent a FIN, an RST, or nothing at all. They
        // must be classified together — routing only `ConnectionClosed` here
        // would leave the common "mount is busy" case surfacing as a raw
        // protocol string.
        WsError::ConnectionClosed | WsError::AlreadyClosed => abrupt_end_failure(live),
        WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => {
            abrupt_end_failure(live)
        }
        WsError::Io(io)
            if matches!(
                io.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::UnexpectedEof
            ) =>
        {
            abrupt_end_failure(live)
        }
        WsError::WriteBufferFull(_) => Failure::retryable(
            "The connection is too slow to carry the stream — try a lower bitrate.",
        ),
        WsError::Io(io) => match io.kind() {
            std::io::ErrorKind::TimedOut => Failure::retryable(
                "Connection timed out — check your firewall, antivirus, or that the streaming port isn't blocked on this network.",
            ),
            std::io::ErrorKind::ConnectionRefused => {
                Failure::retryable("Connection refused — the server isn't accepting connections.")
            }
            _ => Failure::retryable(format!("Network error: {io}")),
        },
        other => Failure::retryable(format!("Connection error: {other}")),
    }
}

/// Map a failed handshake to a message and a retry decision.
fn handshake_failure(e: WsError) -> Failure {
    match e {
        // The upgrade was answered with a normal HTTP response. This is the
        // most likely failure for a mistyped path, because the harbor answers
        // 404 for any request it does not recognise as a webcast upgrade.
        WsError::Http(response) => {
            let status = response.status();
            if status == 404 {
                Failure::fatal("Mount point not found on the server — check the path, and that this station has streamers enabled.")
            } else if status == 401 || status == 403 {
                Failure::fatal("Authentication failed — check your username and password.")
            } else if status.is_server_error()
                || status == 408
                || status == 425
                || status == 429
            {
                // A restarting station, a proxy hiccup or a rate limit. Giving
                // up here would end a broadcast that would have recovered on
                // its own a few seconds later.
                Failure::retryable(format!(
                    "The server is temporarily unavailable (HTTP {status}) — it may be restarting."
                ))
            } else {
                Failure::fatal(format!("The server refused the connection (HTTP {status})."))
            }
        }
        // native-tls reports everything from the ClientHello onward as a TLS
        // error, including a socket that simply dropped mid-handshake. Only a
        // genuine trust failure is worth giving up on, so look for an I/O
        // cause underneath before blaming the certificate.
        WsError::Tls(tls) => {
            if let Some(io) = io_source(&tls) {
                Failure::retryable(format!("The secure connection dropped: {io}"))
            } else {
                Failure::fatal(format!(
                    "The server's TLS certificate could not be verified: {tls}. On a school or company network, the IT team may need to install their certificate on this machine."
                ))
            }
        }
        WsError::Io(io) => match io.kind() {
            std::io::ErrorKind::TimedOut => Failure::retryable(
                "Connection timed out — check your firewall, antivirus, or that the streaming port isn't blocked on this network.",
            ),
            std::io::ErrorKind::ConnectionRefused => {
                Failure::retryable("Connection refused — the server isn't accepting connections.")
            }
            std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => {
                Failure::fatal(format!("Server address not found — check the host. ({io})"))
            }
            _ => Failure::retryable(format!("Could not reach the server: {io}")),
        },
        WsError::Url(url) => Failure::fatal(format!("Invalid server address: {url}")),
        other => Failure::retryable(format!("Could not open the connection: {other}")),
    }
}

/// Walk an error's source chain looking for an underlying I/O failure, which
/// distinguishes "the socket died" from "the certificate is not trusted".
fn io_source(e: &(dyn std::error::Error + 'static)) -> Option<String> {
    let mut source = e.source();
    while let Some(err) = source {
        if let Some(io) = err.downcast_ref::<std::io::Error>() {
            return Some(io.to_string());
        }
        source = err.source();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{StreamFormat, Transport};

    fn close(code: u16, reason: &str) -> CloseFrame {
        CloseFrame {
            code: CloseCode::from(code),
            reason: reason.to_string().into(),
        }
    }

    fn cfg(host: &str, port: u16, mount: &str) -> StreamConfig {
        StreamConfig {
            device_id: "dev".into(),
            host: host.into(),
            port,
            mount: mount.into(),
            username: "loan".into(),
            password: "secret".into(),
            bitrate: 128,
            format: StreamFormat::Mp3,
            transport: Transport::Webcast,
        }
    }

    // ───────── URL construction ─────────

    #[test]
    fn url_uses_wss_and_hides_default_port() {
        let c = cfg("stream.radios.bzh", 443, "/webdj/porte-voix/");
        assert_eq!(c.webcast_url(), "wss://stream.radios.bzh/webdj/porte-voix/");
    }

    #[test]
    fn url_keeps_non_default_port() {
        let c = cfg("stream.radios.bzh", 8443, "/webdj/porte-voix/");
        assert_eq!(
            c.webcast_url(),
            "wss://stream.radios.bzh:8443/webdj/porte-voix/"
        );
    }

    #[test]
    fn url_allows_plaintext_only_on_loopback() {
        assert!(cfg("localhost", 8000, "/webdj/x/")
            .webcast_url()
            .starts_with("ws://"));
        assert!(cfg("127.0.0.1", 8000, "/webdj/x/")
            .webcast_url()
            .starts_with("ws://"));
        // Anything that leaves the machine must be TLS.
        assert!(cfg("example.com", 8000, "/webdj/x/")
            .webcast_url()
            .starts_with("wss://"));
    }

    #[test]
    fn url_normalizes_missing_leading_slash() {
        let c = cfg("example.com", 443, "webdj/x/");
        assert_eq!(c.webcast_url(), "wss://example.com/webdj/x/");
    }

    // ───────── handshake request ─────────

    #[test]
    fn request_carries_the_exact_subprotocol() {
        let req = build_request("wss://example.com/webdj/x/").unwrap();
        // Liquidsoap compares this value byte-for-byte; a list would 404.
        assert_eq!(
            req.headers().get("Sec-WebSocket-Protocol").unwrap(),
            "webcast"
        );
    }

    // ───────── close-frame classification ─────────
    //
    // These assert on `Failure::fatal`, which is what the pipeline now acts on.
    // The old design inferred it by substring-matching the English message,
    // which is exactly how a "server refused the connection (HTTP 502)" ended
    // up permanently killing a broadcast during a station restart.

    #[test]
    fn auth_failure_is_fatal() {
        let f = close_failure(Some(&close(1011, "Authentication failed.")));
        assert!(f.fatal, "bad credentials must not be retried");
        assert!(f.message.contains("Authentication failed"));
    }

    #[test]
    fn invalid_hello_is_fatal() {
        assert!(close_failure(Some(&close(1002, "Invalid hello."))).fatal);
    }

    #[test]
    fn busy_mount_is_retryable() {
        let f = close_failure(Some(&close(1011, "This mountpoint isn't available.")));
        assert!(
            !f.fatal,
            "a reconnect can race the previous session being torn down, so this must retry"
        );
    }

    #[test]
    fn unknown_close_is_retryable() {
        let f = close_failure(Some(&close(1001, "Going away")));
        assert!(!f.fatal);
        assert!(f.message.contains("Going away"));
    }

    #[test]
    fn missing_close_frame_is_retryable() {
        let f = close_failure(None);
        assert!(!f.fatal);
        assert!(!f.message.is_empty());
    }

    // ───────── abrupt disconnect ─────────

    #[test]
    fn a_bare_drop_reaches_the_right_classifier() {
        // Liquidsoap drops the TCP connection with no close frame when the
        // mount is already held. That surfaces as one of these three variants
        // depending on FIN vs RST vs nothing — all must produce the mount-busy
        // message, not a raw protocol string.
        for e in [
            WsError::ConnectionClosed,
            WsError::AlreadyClosed,
            WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake),
            WsError::Io(std::io::Error::from(std::io::ErrorKind::ConnectionReset)),
            WsError::Io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)),
        ] {
            let f = transport_failure(&e, false);
            assert!(
                f.message.contains("mount point may already be in use"),
                "unclassified bare drop: {:?} => {}",
                e,
                f.message
            );
            assert!(!f.fatal, "a busy mount clears on its own, so it must retry");
        }
    }

    #[test]
    fn a_lost_link_mid_broadcast_is_retryable() {
        let f = transport_failure(&WsError::ConnectionClosed, true);
        assert!(!f.fatal);
        assert!(f.message.contains("lost"));
    }

    // ───────── handshake classification ─────────

    fn http_failure(status: u16) -> Failure {
        let response = tokio_tungstenite::tungstenite::http::Response::builder()
            .status(status)
            .body(None)
            .unwrap();
        handshake_failure(WsError::Http(Box::new(response)))
    }

    #[test]
    fn a_restarting_server_is_retryable() {
        // The whole point of the reconnect loop. A 502 from a proxy whose
        // backend is restarting used to match "server refused the connection"
        // and permanently end the broadcast.
        for status in [500u16, 502, 503, 504, 408, 429] {
            let f = http_failure(status);
            assert!(
                !f.fatal,
                "HTTP {status} must be retryable, got: {}",
                f.message
            );
        }
    }

    #[test]
    fn a_client_mistake_is_fatal() {
        for status in [400u16, 401, 403, 404, 410] {
            assert!(http_failure(status).fatal, "HTTP {status} should be fatal");
        }
    }

    #[test]
    fn a_missing_path_names_the_likely_cause() {
        let f = http_failure(404);
        assert!(f.message.contains("streamers enabled"));
    }

    #[test]
    fn an_unparseable_url_is_fatal() {
        // Retrying a malformed address forever would just spin the encoder.
        let f = build_request("not a url").unwrap_err();
        assert!(f.fatal);
    }

    // ───────── live server ─────────
    //
    // Ignored by default: needs the network and a real AzuraCast station.
    //
    //   AIRCAST_WEBCAST_HOST=stream.example.com \\
    //   AIRCAST_WEBCAST_MOUNT=/webdj/my-station/ \\
    //   cargo test --lib webcast -- --ignored --nocapture
    //
    // Without credentials it asserts the server rejects the login, which
    // proves the whole path up to auth. Add AIRCAST_WEBCAST_USER/_PASS and it
    // asserts the opposite: a socket that stays open, i.e. the stream is live.

    fn env_cfg() -> Option<StreamConfig> {
        let host = std::env::var("AIRCAST_WEBCAST_HOST").ok()?;
        let mount = std::env::var("AIRCAST_WEBCAST_MOUNT").unwrap_or_else(|_| "/".to_string());
        let port: u16 = std::env::var("AIRCAST_WEBCAST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(443);
        let mut c = cfg(&host, port, &mount);
        c.username = std::env::var("AIRCAST_WEBCAST_USER").unwrap_or_else(|_| "nobody".into());
        c.password =
            std::env::var("AIRCAST_WEBCAST_PASS").unwrap_or_else(|_| "definitely-wrong".into());
        Some(c)
    }

    #[tokio::test]
    #[ignore = "requires the network and a live AzuraCast station"]
    async fn live_handshake_and_auth() {
        let Some(config) = env_cfg() else {
            panic!("set AIRCAST_WEBCAST_HOST (and optionally _MOUNT/_PORT/_USER/_PASS)");
        };
        let url = config.webcast_url();
        eprintln!("connecting to {url}");

        // Getting here at all proves the 101 upgrade succeeded: the harbor 404s
        // anything whose Upgrade / Sec-WebSocket-Protocol values do not match
        // byte for byte, which tungstenite surfaces as Http(404).
        let mut socket = match connect(&config, &url).await {
            Ok(s) => s,
            Err(f) => panic!("handshake failed: {}", f.message),
        };
        eprintln!("upgraded, hello sent as {}", mime_for(&config.format));

        let expect_success = std::env::var("AIRCAST_WEBCAST_USER").is_ok();

        // A rejection arrives well inside a second; success is silent.
        match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
            Err(_) => {
                assert!(
                    expect_success,
                    "server stayed silent with bogus credentials — expected a close frame"
                );
                eprintln!("silent for 5s => credentials accepted, the stream would go live");
            }
            Ok(Some(Ok(Message::Close(frame)))) => {
                let f = close_failure(frame.as_ref());
                eprintln!("server closed: {} (fatal={})", f.message, f.fatal);
                assert!(
                    !expect_success,
                    "expected the stream to be accepted, but the server closed it: {}",
                    f.message
                );
                assert!(
                    f.message.contains("Authentication failed"),
                    "expected an auth rejection, got: {}",
                    f.message
                );
                assert!(f.fatal, "bad credentials must be classified fatal");
            }
            Ok(other) => panic!("unexpected first frame from the harbor: {other:?}"),
        }
    }

    /// The real thing: push actual MP3 through the transport and confirm the
    /// station goes live. Everything else stops at the handshake.
    ///
    ///   AIRCAST_WEBCAST_HOST=stream.example.com \\
    ///   AIRCAST_WEBCAST_MOUNT=/webdj/my-station/ \\
    ///   AIRCAST_WEBCAST_USER=dj AIRCAST_WEBCAST_PASS=... \\
    ///   AIRCAST_WEBCAST_BROADCAST=1 \\
    ///   cargo test --lib webcast -- --ignored --nocapture
    ///
    /// This BROADCASTS to a real station — it interrupts whatever is playing.
    #[tokio::test]
    #[ignore = "broadcasts real audio to a live station"]
    async fn live_broadcast_end_to_end() {
        if std::env::var("AIRCAST_WEBCAST_BROADCAST").is_err() {
            eprintln!("set AIRCAST_WEBCAST_BROADCAST=1 to actually go on air; skipping");
            return;
        }
        let config = env_cfg().expect("set AIRCAST_WEBCAST_HOST/_MOUNT/_USER/_PASS");
        let secs: u64 = std::env::var("AIRCAST_WEBCAST_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        let ffmpeg_bin = std::env::var("AIRCAST_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
        let nonce =
            std::env::var("AIRCAST_WEBCAST_NONCE").unwrap_or_else(|_| "Aircast probe".into());

        let url = config.webcast_url();
        eprintln!("connecting to {url}");
        let socket = connect(&config, &url)
            .await
            .unwrap_or_else(|f| panic!("{}", f.message));
        eprintln!("upgraded, hello sent as {}", mime_for(&config.format));

        // A 440 Hz tone at -20 dBFS, encoded exactly as the app does and paced
        // at real time by `-re`, which is what the harbor's generator expects.
        let mut enc = tokio::process::Command::new(&ffmpeg_bin)
            .args(["-hide_banner", "-loglevel", "error"])
            .args([
                "-re",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100",
            ])
            .args(["-af", "volume=-20dB"])
            .args(["-c:a", "libmp3lame", "-b:a", "128k"])
            .args(["-f", "mp3", "-id3v2_version", "0", "-write_xing", "0"])
            .args(["-flush_packets", "1", "pipe:1"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn ffmpeg");
        let stdout = enc.stdout.take().unwrap();

        let (audio_tx, mut audio_rx) = mpsc::channel::<Bytes>(AUDIO_QUEUE_DEPTH);
        let pump = tokio::spawn(pump_encoder(stdout, audio_tx));

        let (mut sink, mut stream) = socket.split();
        let deadline = Instant::now() + Duration::from_secs(secs);
        let mut sent = 0usize;
        let mut bytes = 0usize;
        let mut meta_sent = false;

        while Instant::now() < deadline {
            tokio::select! {
                biased;
                inbound = stream.next() => match inbound {
                    Some(Ok(Message::Close(f))) => panic!("server closed: {}", close_failure(f.as_ref()).message),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => panic!("transport error: {}", transport_failure(&e, true).message),
                    None => panic!("socket closed unexpectedly after {sent} frames"),
                },
                chunk = audio_rx.recv() => {
                    let Some(chunk) = chunk else { break };
                    assert!(!chunk.is_empty(), "a zero-length frame would end the broadcast");
                    bytes += chunk.len();
                    send_bounded(&mut sink, Message::Binary(chunk), true)
                        .await
                        .unwrap_or_else(|f| panic!("send failed after {sent} frames: {}", f.message));
                    sent += 1;
                    if !meta_sent && sent > 20 {
                        meta_sent = true;
                        let frame = serde_json::json!({
                            "type": "metadata",
                            "data": { "title": nonce, "artist": "" }
                        });
                        send_bounded(&mut sink, Message::Text(frame.to_string().into()), true)
                            .await
                            .expect("metadata send");
                        eprintln!("metadata sent: {nonce}");
                    }
                }
            }
        }

        eprintln!("sent {sent} frames / {bytes} bytes over {secs}s");
        assert!(
            sent > 10,
            "the encoder barely produced anything: {sent} frames"
        );
        // ~128 kbps => ~16 KiB/s. Well under means we were not pacing at real time.
        let expected = (secs as usize) * 16_000;
        assert!(
            bytes > expected / 2,
            "only {bytes} bytes in {secs}s — expected around {expected}"
        );

        pump.abort();
        let _ = tokio::time::timeout(CLOSE_TIMEOUT, graceful_close(&mut sink, &mut audio_rx)).await;
        eprintln!("closed cleanly");
    }

    // ───────── metadata sink ─────────

    #[test]
    fn metadata_sink_reports_no_session() {
        let sink = MetadataSink::new();
        assert!(sink.try_send("hello").is_err());
    }

    #[test]
    fn metadata_sink_delivers_when_connected() {
        let sink = MetadataSink::new();
        let (tx, mut rx) = mpsc::channel::<String>(4);
        sink.set(Some(tx));
        assert!(sink.try_send("Artist — Title").is_ok());
        assert_eq!(rx.try_recv().unwrap(), "Artist — Title");
        sink.set(None);
        assert!(sink.try_send("later").is_err());
    }
}
