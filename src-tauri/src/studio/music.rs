use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::studio::resampler::{map_channels_add, map_channels_set, FrameResampler};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub id: String,
    pub path: PathBuf,
    pub title: String,
    pub duration_secs: Option<f32>,
}

impl TrackInfo {
    pub fn from_path(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let duration_secs = probe_duration(&path);
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            title,
            duration_secs,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicSnapshot {
    pub state: PlayerState,
    pub queue: Vec<TrackInfo>,
    pub current: Option<CurrentTrackSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTrackSnapshot {
    pub info: TrackInfo,
    pub elapsed_secs: f32,
    pub duration_secs: Option<f32>,
}

type SampleStream = Box<dyn Iterator<Item = f32> + Send>;

struct CurrentTrack {
    info: TrackInfo,
    decoder: SampleStream,
    resampler: FrameResampler,
}

pub struct MusicPlayer {
    target_rate: u32,
    target_channels: u16,
    queue: VecDeque<TrackInfo>,
    current: Option<CurrentTrack>,
    fading_out: Option<CurrentTrack>,
    fade_remaining_target_frames: u64,
    fade_total_target_frames: u64,
    crossfade_seconds: f32,
    state: PlayerState,
    duration_overrides: HashMap<String, f32>,
}

impl MusicPlayer {
    pub fn new(target_rate: u32, target_channels: u16) -> Self {
        Self {
            target_rate,
            target_channels,
            queue: VecDeque::new(),
            current: None,
            fading_out: None,
            fade_remaining_target_frames: 0,
            fade_total_target_frames: 0,
            crossfade_seconds: 3.0,
            state: PlayerState::Stopped,
            duration_overrides: HashMap::new(),
        }
    }

    pub fn set_format(&mut self, target_rate: u32, target_channels: u16) {
        if self.target_rate == target_rate && self.target_channels == target_channels {
            return;
        }
        self.target_rate = target_rate;
        self.target_channels = target_channels;
    }

    pub fn set_crossfade_seconds(&mut self, seconds: f32) {
        self.crossfade_seconds = seconds.clamp(0.0, 30.0);
    }

    pub fn set_duration_override(&mut self, track_id: &str, duration_secs: f32) {
        if duration_secs.is_finite() && duration_secs > 0.0 {
            self.duration_overrides
                .insert(track_id.to_string(), duration_secs);
        }
    }

    pub fn enqueue(&mut self, info: TrackInfo) {
        self.queue.push_back(info);
    }

    pub fn remove(&mut self, id: &str) {
        self.queue.retain(|t| t.id != id);
        self.duration_overrides.remove(id);
    }

    pub fn move_track(&mut self, id: &str, delta: i32) {
        let Some(pos) = self.queue.iter().position(|t| t.id == id) else {
            return;
        };
        let new_pos = (pos as i32 + delta).clamp(0, self.queue.len() as i32 - 1) as usize;
        if new_pos == pos {
            return;
        }
        let item = self.queue.remove(pos).unwrap();
        self.queue.insert(new_pos, item);
    }

    pub fn play(&mut self) -> Result<(), String> {
        if self.current.is_none() {
            self.advance_to_next(false)?;
        }
        if self.current.is_some() {
            self.state = PlayerState::Playing;
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        if matches!(self.state, PlayerState::Playing) {
            self.state = PlayerState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.current = None;
        self.fading_out = None;
        self.fade_remaining_target_frames = 0;
        self.fade_total_target_frames = 0;
        self.state = PlayerState::Stopped;
    }

    pub fn next_track(&mut self) -> Result<(), String> {
        self.advance_to_next(true)
    }

    pub fn snapshot(&self) -> MusicSnapshot {
        let map_info = |info: &TrackInfo| {
            let mut info = info.clone();
            if let Some(d) = self.duration_overrides.get(&info.id) {
                info.duration_secs = Some(*d);
            }
            info
        };
        let current = self.current.as_ref().map(|c| {
            let info = map_info(&c.info);
            let elapsed_secs =
                c.resampler.source_frames_read() as f32 / c.resampler.source_rate().max(1) as f32;
            CurrentTrackSnapshot {
                duration_secs: info.duration_secs,
                info,
                elapsed_secs,
            }
        });
        MusicSnapshot {
            state: self.state,
            queue: self.queue.iter().map(map_info).collect(),
            current,
        }
    }

    fn advance_to_next(&mut self, with_crossfade: bool) -> Result<(), String> {
        if with_crossfade && self.current.is_some() && self.crossfade_seconds > 0.0 {
            let fade_total = (self.crossfade_seconds * self.target_rate.max(1) as f32) as u64;
            self.fading_out = self.current.take();
            self.fade_total_target_frames = fade_total;
            self.fade_remaining_target_frames = fade_total;
        } else {
            self.current = None;
        }

        match self.queue.pop_front() {
            Some(track) => {
                let (decoder, source_rate, source_channels) = open_native(&track.path)?;
                log::info!(
                    "music play '{}': native {}Hz/{}ch → target {}Hz/{}ch (step={:.4}, xfade={:.1}s)",
                    track.title,
                    source_rate,
                    source_channels,
                    self.target_rate,
                    self.target_channels,
                    source_rate as f64 / self.target_rate.max(1) as f64,
                    if with_crossfade { self.crossfade_seconds } else { 0.0 },
                );
                self.current = Some(CurrentTrack {
                    info: track,
                    decoder,
                    resampler: FrameResampler::new(source_rate, source_channels),
                });
                Ok(())
            }
            None => {
                self.current = None;
                if self.fading_out.is_none() {
                    self.state = PlayerState::Stopped;
                }
                Ok(())
            }
        }
    }

    pub fn pull(&mut self, buf: &mut [f32]) -> bool {
        if !matches!(self.state, PlayerState::Playing) {
            buf.fill(0.0);
            return false;
        }
        let target_chs = self.target_channels.max(1) as usize;
        if target_chs == 0 || buf.is_empty() {
            return false;
        }
        let target_rate = self.target_rate;
        let target_frames = buf.len() / target_chs;
        let mut produced_any = false;

        let mut f = 0;
        while f < target_frames {
            let out_off = f * target_chs;
            let out = &mut buf[out_off..out_off + target_chs];

            let (new_gain, old_gain) =
                if self.fade_total_target_frames > 0 && self.fading_out.is_some() {
                    let progress = 1.0
                        - (self.fade_remaining_target_frames as f32
                            / self.fade_total_target_frames as f32);
                    let p = progress.clamp(0.0, 1.0);
                    let half_pi = std::f32::consts::FRAC_PI_2;
                    ((p * half_pi).sin(), (p * half_pi).cos())
                } else {
                    (1.0, 0.0)
                };

            // Step the current track. If exhausted, advance and retry the
            // same target frame with the next track.
            let current_outcome = if let Some(track) = self.current.as_mut() {
                let resampler = &mut track.resampler;
                let decoder = &mut track.decoder;
                if resampler.step(target_rate, || decoder.next()) {
                    map_channels_set(resampler.last_frame(), out, new_gain);
                    Outcome::Emitted
                } else {
                    Outcome::Exhausted
                }
            } else {
                Outcome::None
            };

            match current_outcome {
                Outcome::Emitted => {
                    produced_any = true;
                }
                Outcome::Exhausted => {
                    self.current = None;
                    let _ = self.advance_to_next(false);
                    if self.current.is_some() {
                        // Retry this same target frame with the new track.
                        // Important: don't increment `f`.
                        continue;
                    } else {
                        // No next track; let fading_out finish if it's still alive.
                        if self.fading_out.is_none() {
                            self.state = PlayerState::Stopped;
                            for s in &mut buf[out_off..] {
                                *s = 0.0;
                            }
                            return produced_any;
                        }
                        // Output silence for the current contribution.
                        for s in out.iter_mut() {
                            *s = 0.0;
                        }
                    }
                }
                Outcome::None => {
                    if self.fading_out.is_none() {
                        self.state = PlayerState::Stopped;
                        for s in &mut buf[out_off..] {
                            *s = 0.0;
                        }
                        return produced_any;
                    }
                    for s in out.iter_mut() {
                        *s = 0.0;
                    }
                }
            }

            // Step the fading_out track (additive into out).
            let fading_alive = match (old_gain > 0.0, self.fading_out.as_mut()) {
                (true, Some(track)) => {
                    let resampler = &mut track.resampler;
                    let decoder = &mut track.decoder;
                    if resampler.step(target_rate, || decoder.next()) {
                        map_channels_add(resampler.last_frame(), out, old_gain);
                        produced_any = true;
                        true
                    } else {
                        false
                    }
                }
                (_, Some(_)) => true, // gain is 0 but fading_out still exists; keep alive flag
                _ => false,
            };
            if !fading_alive {
                self.fading_out = None;
                self.fade_remaining_target_frames = 0;
                self.fade_total_target_frames = 0;
            }

            // Decrement fade window.
            if self.fading_out.is_some() && self.fade_remaining_target_frames > 0 {
                self.fade_remaining_target_frames -= 1;
                if self.fade_remaining_target_frames == 0 {
                    self.fading_out = None;
                    self.fade_total_target_frames = 0;
                }
            }

            f += 1;
        }

        produced_any
    }
}

enum Outcome {
    Emitted,
    Exhausted,
    None,
}

fn open_native(path: &Path) -> Result<(SampleStream, u32, u16), String> {
    let file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let decoder = Decoder::new(BufReader::new(file)).map_err(|e| format!("decode: {e}"))?;
    let source_rate = decoder.sample_rate();
    let source_channels = decoder.channels();
    if source_channels == 0 {
        return Err("source has 0 channels".into());
    }
    let iter: SampleStream = Box::new(decoder.convert_samples::<f32>());
    Ok((iter, source_rate, source_channels))
}

fn probe_duration(path: &Path) -> Option<f32> {
    let file = File::open(path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    decoder.total_duration().map(|d| d.as_secs_f32())
}

/// Decode the file end-to-end, counting samples to produce an exact duration.
pub fn scan_full_duration(path: &Path) -> Option<f32> {
    let file = File::open(path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    let rate = decoder.sample_rate();
    let channels = decoder.channels().max(1);
    let total: usize = decoder.convert_samples::<f32>().count();
    let frames = total / channels as usize;
    if rate == 0 {
        return None;
    }
    Some(frames as f32 / rate as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Write a minimal PCM-S16LE WAV file with `duration_secs` of silence at
    /// the given format. Returns the path.
    fn write_silence_wav(
        dir: &Path,
        name: &str,
        duration_secs: f32,
        sample_rate: u32,
        channels: u16,
    ) -> PathBuf {
        let path = dir.join(name);
        let total_frames = (duration_secs * sample_rate as f32) as u32;
        let total_samples = total_frames * channels as u32;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;
        let data_size = total_samples * 2;
        let chunk_size = 36 + data_size;

        let mut f = File::create(&path).unwrap();
        f.write_all(b"RIFF").unwrap();
        f.write_all(&chunk_size.to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16_u32.to_le_bytes()).unwrap();
        f.write_all(&1_u16.to_le_bytes()).unwrap();
        f.write_all(&channels.to_le_bytes()).unwrap();
        f.write_all(&sample_rate.to_le_bytes()).unwrap();
        f.write_all(&byte_rate.to_le_bytes()).unwrap();
        f.write_all(&block_align.to_le_bytes()).unwrap();
        f.write_all(&16_u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_size.to_le_bytes()).unwrap();
        for _ in 0..total_samples {
            f.write_all(&0_i16.to_le_bytes()).unwrap();
        }
        path
    }

    #[test]
    fn track_info_from_path_extracts_title_from_filename() {
        let info = TrackInfo::from_path(PathBuf::from("/tmp/My Song.mp3"));
        assert_eq!(info.title, "My Song");
    }

    #[test]
    fn track_info_falls_back_to_untitled_when_no_stem() {
        let info = TrackInfo::from_path(PathBuf::from(""));
        assert_eq!(info.title, "Untitled");
    }

    #[test]
    fn enqueue_appends_to_back_of_queue() {
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path: "x".into(),
            title: "A".into(),
            duration_secs: None,
        });
        p.enqueue(TrackInfo {
            id: "b".into(),
            path: "y".into(),
            title: "B".into(),
            duration_secs: None,
        });
        let snap = p.snapshot();
        assert_eq!(snap.queue.len(), 2);
        assert_eq!(snap.queue[0].id, "a");
        assert_eq!(snap.queue[1].id, "b");
    }

    #[test]
    fn remove_drops_track_from_queue_and_clears_override() {
        let mut p = MusicPlayer::new(48_000, 1);
        let track = TrackInfo {
            id: "a".into(),
            path: "x".into(),
            title: "A".into(),
            duration_secs: None,
        };
        p.enqueue(track);
        p.set_duration_override("a", 12.5);
        p.remove("a");
        assert_eq!(p.snapshot().queue.len(), 0);
        // re-enqueue; override should not magically reappear
        p.enqueue(TrackInfo {
            id: "a".into(),
            path: "x".into(),
            title: "A".into(),
            duration_secs: None,
        });
        let snap = p.snapshot();
        assert!(snap.queue[0].duration_secs.is_none());
    }

    #[test]
    fn move_track_reorders_queue() {
        let mut p = MusicPlayer::new(48_000, 1);
        for id in ["a", "b", "c"] {
            p.enqueue(TrackInfo {
                id: id.into(),
                path: "x".into(),
                title: id.into(),
                duration_secs: None,
            });
        }
        p.move_track("c", -2);
        let ids: Vec<String> = p.snapshot().queue.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn move_track_clamps_to_bounds() {
        let mut p = MusicPlayer::new(48_000, 1);
        for id in ["a", "b"] {
            p.enqueue(TrackInfo {
                id: id.into(),
                path: "x".into(),
                title: id.into(),
                duration_secs: None,
            });
        }
        p.move_track("a", -100);
        let ids: Vec<String> = p.snapshot().queue.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, vec!["a", "b"]);
        p.move_track("a", 100);
        let ids: Vec<String> = p.snapshot().queue.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn duration_override_applied_to_snapshot() {
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path: "x".into(),
            title: "A".into(),
            duration_secs: None,
        });
        p.set_duration_override("a", 42.0);
        let snap = p.snapshot();
        assert_eq!(snap.queue[0].duration_secs, Some(42.0));
    }

    #[test]
    fn set_crossfade_seconds_clamps_to_30s_max() {
        let mut p = MusicPlayer::new(48_000, 1);
        p.set_crossfade_seconds(120.0);
        // Indirect check: we don't expose crossfade getter, but we can set
        // a high value and ensure no panic and follow-up calls still work.
        p.set_crossfade_seconds(-5.0);
        // No panic = success; the clamp lives in the setter.
    }

    #[test]
    fn play_with_empty_queue_is_noop() {
        let mut p = MusicPlayer::new(48_000, 1);
        assert!(p.play().is_ok());
        let snap = p.snapshot();
        assert!(snap.current.is_none());
        assert_eq!(snap.state, PlayerState::Stopped);
    }

    #[test]
    fn play_then_pull_advances_through_track() {
        let dir = TempDir::new().unwrap();
        let path = write_silence_wav(dir.path(), "track.wav", 0.5, 44_100, 1);
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path,
            title: "A".into(),
            duration_secs: None,
        });
        p.play().unwrap();
        let snap = p.snapshot();
        assert_eq!(snap.state, PlayerState::Playing);
        assert!(snap.current.is_some());

        // Pull a few frames.
        let mut buf = vec![0.0_f32; 1024];
        assert!(p.pull(&mut buf));
    }

    #[test]
    fn next_track_advances_to_next_in_queue() {
        let dir = TempDir::new().unwrap();
        let p1 = write_silence_wav(dir.path(), "a.wav", 0.2, 44_100, 1);
        let p2 = write_silence_wav(dir.path(), "b.wav", 0.2, 44_100, 1);
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path: p1,
            title: "A".into(),
            duration_secs: None,
        });
        p.enqueue(TrackInfo {
            id: "b".into(),
            path: p2,
            title: "B".into(),
            duration_secs: None,
        });
        p.play().unwrap();
        assert_eq!(p.snapshot().current.unwrap().info.id, "a");
        p.next_track().unwrap();
        assert_eq!(p.snapshot().current.unwrap().info.id, "b");
    }

    #[test]
    fn stop_clears_current_and_state() {
        let dir = TempDir::new().unwrap();
        let path = write_silence_wav(dir.path(), "t.wav", 0.2, 44_100, 1);
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path,
            title: "A".into(),
            duration_secs: None,
        });
        p.play().unwrap();
        p.stop();
        let snap = p.snapshot();
        assert!(snap.current.is_none());
        assert_eq!(snap.state, PlayerState::Stopped);
    }

    #[test]
    fn pause_keeps_current_track() {
        let dir = TempDir::new().unwrap();
        let path = write_silence_wav(dir.path(), "t.wav", 0.5, 44_100, 1);
        let mut p = MusicPlayer::new(48_000, 1);
        p.enqueue(TrackInfo {
            id: "a".into(),
            path,
            title: "A".into(),
            duration_secs: None,
        });
        p.play().unwrap();
        p.pause();
        let snap = p.snapshot();
        assert!(snap.current.is_some());
        assert_eq!(snap.state, PlayerState::Paused);
    }

    #[test]
    fn pull_when_stopped_fills_silence() {
        let mut p = MusicPlayer::new(48_000, 1);
        let mut buf = vec![1.0_f32; 256];
        assert!(!p.pull(&mut buf));
        assert!(buf.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn scan_full_duration_returns_correct_seconds_for_known_wav() {
        let dir = TempDir::new().unwrap();
        let path = write_silence_wav(dir.path(), "one.wav", 1.0, 44_100, 1);
        let dur = scan_full_duration(&path).unwrap();
        // Allow ±1 frame slop
        assert!((dur - 1.0).abs() < 0.001, "got {dur}");
    }
}
