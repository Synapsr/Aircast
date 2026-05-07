use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};

use crate::studio::resampler::{map_channels_add, FrameResampler};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartSlot {
    pub slot: u8,
    pub name: String,
    pub path: PathBuf,
    pub duration_secs: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartSnapshot {
    pub slot: u8,
    pub name: String,
    pub duration_secs: f32,
    pub elapsed_secs: f32,
    pub playing: bool,
}

struct LoadedCart {
    info: CartSlot,
    samples: Arc<Vec<f32>>,
    source_rate: u32,
    source_channels: u16,
    /// Index of the next sample to read from `samples` (linear, in samples
    /// not frames — use source_channels for the stride).
    read_idx: usize,
    resampler: FrameResampler,
    playing: bool,
}

pub struct CartBank {
    target_rate: u32,
    target_channels: u16,
    slots: HashMap<u8, LoadedCart>,
}

impl CartBank {
    pub fn new(target_rate: u32, target_channels: u16) -> Self {
        Self {
            target_rate,
            target_channels,
            slots: HashMap::new(),
        }
    }

    pub fn set_format(&mut self, target_rate: u32, target_channels: u16) -> Result<(), String> {
        // Format change is now a metadata-only update — the resampler picks
        // up the new ratio at the next `step` call, no re-decoding required.
        self.target_rate = target_rate;
        self.target_channels = target_channels;
        Ok(())
    }

    pub fn assign(&mut self, slot: u8, name: String, path: PathBuf) -> Result<CartSlot, String> {
        let (samples, source_rate, source_channels, source_duration) = decode_native(&path)?;
        let total_frames = samples.len() / source_channels.max(1) as usize;
        let duration_secs =
            source_duration.unwrap_or_else(|| total_frames as f32 / source_rate.max(1) as f32);
        let info = CartSlot {
            slot,
            name,
            path,
            duration_secs,
        };
        log::info!(
            "cart slot {} '{}' loaded: {} samples at native {}Hz/{}ch ({:.2}s) — bank target {}Hz/{}ch",
            slot,
            info.name,
            samples.len(),
            source_rate,
            source_channels,
            duration_secs,
            self.target_rate,
            self.target_channels,
        );
        self.slots.insert(
            slot,
            LoadedCart {
                info: info.clone(),
                samples: Arc::new(samples),
                source_rate,
                source_channels,
                read_idx: 0,
                resampler: FrameResampler::new(source_rate, source_channels),
                playing: false,
            },
        );
        Ok(info)
    }

    pub fn remove(&mut self, slot: u8) {
        self.slots.remove(&slot);
    }

    pub fn play(&mut self, slot: u8) {
        if let Some(cart) = self.slots.get_mut(&slot) {
            cart.read_idx = 0;
            cart.resampler = FrameResampler::new(cart.source_rate, cart.source_channels);
            cart.playing = true;
            log::info!(
                "cart play slot {} '{}': native {}Hz/{}ch → target {}Hz/{}ch (step={:.4})",
                slot,
                cart.info.name,
                cart.source_rate,
                cart.source_channels,
                self.target_rate,
                self.target_channels,
                cart.source_rate as f64 / self.target_rate.max(1) as f64,
            );
        }
    }

    pub fn stop(&mut self, slot: u8) {
        if let Some(cart) = self.slots.get_mut(&slot) {
            cart.playing = false;
        }
    }

    pub fn stop_all(&mut self) {
        for cart in self.slots.values_mut() {
            cart.playing = false;
        }
    }

    pub fn add_to(&mut self, buf: &mut [f32]) {
        let target_chs = self.target_channels.max(1) as usize;
        if target_chs == 0 || buf.is_empty() {
            return;
        }
        let target_rate = self.target_rate;
        let target_frames = buf.len() / target_chs;

        for cart in self.slots.values_mut() {
            if !cart.playing {
                continue;
            }
            for f in 0..target_frames {
                if !cart.playing {
                    break;
                }

                // Borrow disjoint fields so the closure can read from
                // `samples`/`read_idx` while the resampler is mutably borrowed.
                let resampler = &mut cart.resampler;
                let samples = &cart.samples;
                let mut idx = cart.read_idx;
                let advanced = resampler.step(target_rate, || {
                    if idx < samples.len() {
                        let v = samples[idx];
                        idx += 1;
                        Some(v)
                    } else {
                        None
                    }
                });
                cart.read_idx = idx;

                if !advanced {
                    cart.playing = false;
                    break;
                }

                let out_off = f * target_chs;
                map_channels_add(
                    resampler.last_frame(),
                    &mut buf[out_off..out_off + target_chs],
                    1.0,
                );
            }
        }
    }

    pub fn snapshot(&self) -> Vec<CartSnapshot> {
        let mut out: Vec<_> = self
            .slots
            .values()
            .map(|cart| {
                let total_frames = cart.samples.len() / cart.source_channels.max(1) as usize;
                let pos_frames = cart.resampler.source_frames_read().min(total_frames as u64);
                let elapsed_secs = pos_frames as f32 / cart.source_rate.max(1) as f32;
                let total_secs = total_frames as f32 / cart.source_rate.max(1) as f32;
                CartSnapshot {
                    slot: cart.info.slot,
                    name: cart.info.name.clone(),
                    duration_secs: total_secs,
                    elapsed_secs,
                    playing: cart.playing,
                }
            })
            .collect();
        out.sort_by_key(|c| c.slot);
        out
    }

    pub fn persisted(&self) -> Vec<CartSlot> {
        let mut out: Vec<_> = self.slots.values().map(|c| c.info.clone()).collect();
        out.sort_by_key(|c| c.slot);
        out
    }
}

fn decode_native(path: &Path) -> Result<(Vec<f32>, u32, u16, Option<f32>), String> {
    let file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let decoder = Decoder::new(BufReader::new(file)).map_err(|e| format!("decode: {e}"))?;
    let source_rate = decoder.sample_rate();
    let source_channels = decoder.channels();
    let source_duration = decoder.total_duration().map(|d| d.as_secs_f32());
    let samples: Vec<f32> = decoder.convert_samples::<f32>().collect();
    if source_channels == 0 {
        return Err("source has 0 channels".into());
    }
    Ok((samples, source_rate, source_channels, source_duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Write a tiny PCM-S16LE WAV with a constant sample value (0 = silence,
    /// non-zero = something we can detect after add_to).
    fn write_wav(
        dir: &Path,
        name: &str,
        duration_secs: f32,
        sample_rate: u32,
        channels: u16,
        sample_value: i16,
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
            f.write_all(&sample_value.to_le_bytes()).unwrap();
        }
        path
    }

    #[test]
    fn assign_creates_loaded_cart_with_correct_metadata() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.5, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 2);
        let info = bank.assign(1, "test".into(), path).unwrap();
        assert_eq!(info.slot, 1);
        assert_eq!(info.name, "test");
        // Duration is approximately 0.5s (within 1 frame slop)
        assert!(
            (info.duration_secs - 0.5).abs() < 0.01,
            "got {}",
            info.duration_secs
        );
    }

    #[test]
    fn assign_replaces_previous_slot() {
        let dir = TempDir::new().unwrap();
        let p1 = write_wav(dir.path(), "a.wav", 0.2, 44_100, 1, 0);
        let p2 = write_wav(dir.path(), "b.wav", 0.4, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "first".into(), p1).unwrap();
        bank.assign(1, "second".into(), p2).unwrap();
        let snap = bank.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].name, "second");
    }

    #[test]
    fn remove_drops_slot() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(3, "test".into(), path).unwrap();
        bank.remove(3);
        assert!(bank.snapshot().is_empty());
    }

    #[test]
    fn play_marks_cart_playing_and_resets_pos() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "t".into(), path).unwrap();
        bank.play(1);
        let snap = bank.snapshot();
        assert!(snap[0].playing);
        assert_eq!(snap[0].elapsed_secs, 0.0);
    }

    #[test]
    fn stop_marks_cart_not_playing() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "t".into(), path).unwrap();
        bank.play(1);
        bank.stop(1);
        let snap = bank.snapshot();
        assert!(!snap[0].playing);
    }

    #[test]
    fn stop_all_stops_every_cart() {
        let dir = TempDir::new().unwrap();
        let p1 = write_wav(dir.path(), "1.wav", 0.2, 44_100, 1, 0);
        let p2 = write_wav(dir.path(), "2.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "1".into(), p1).unwrap();
        bank.assign(2, "2".into(), p2).unwrap();
        bank.play(1);
        bank.play(2);
        bank.stop_all();
        for c in bank.snapshot() {
            assert!(!c.playing);
        }
    }

    #[test]
    fn add_to_does_nothing_when_no_carts_playing() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "t".into(), path).unwrap();
        // not playing
        let mut buf = vec![1.0_f32; 256];
        bank.add_to(&mut buf);
        // Buf untouched
        assert!(buf.iter().all(|&v| v == 1.0));
    }

    #[test]
    fn add_to_consumes_samples_and_marks_done_at_end_of_cart() {
        let dir = TempDir::new().unwrap();
        // Half-second cart at 44100Hz mono = 22050 source frames.
        let path = write_wav(dir.path(), "c.wav", 0.5, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "t".into(), path).unwrap();
        bank.play(1);

        // Pull enough target frames to exhaust the cart at step ≈ 0.918:
        // 22050 source frames ÷ 0.918 ≈ 24014 target frames needed.
        let mut buf = vec![0.0_f32; 32_000];
        bank.add_to(&mut buf);

        let snap = bank.snapshot();
        assert!(!snap[0].playing, "cart should auto-stop at end");
    }

    #[test]
    fn add_to_is_additive_does_not_clobber() {
        let dir = TempDir::new().unwrap();
        // Cart with constant non-zero samples (i16::MAX/4 → ~0.25 in f32).
        let path = write_wav(dir.path(), "c.wav", 0.05, 44_100, 1, 8192);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(1, "t".into(), path).unwrap();
        bank.play(1);

        let mut buf = vec![0.5_f32; 256];
        bank.add_to(&mut buf);
        // Buf was 0.5; cart adds ~0.25 → buf > 0.5 (additive)
        assert!(buf[0] > 0.5);
    }

    #[test]
    fn snapshot_is_sorted_by_slot() {
        let dir = TempDir::new().unwrap();
        let p1 = write_wav(dir.path(), "1.wav", 0.1, 44_100, 1, 0);
        let p2 = write_wav(dir.path(), "2.wav", 0.1, 44_100, 1, 0);
        let p3 = write_wav(dir.path(), "3.wav", 0.1, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 1);
        bank.assign(5, "5".into(), p1).unwrap();
        bank.assign(2, "2".into(), p2).unwrap();
        bank.assign(8, "8".into(), p3).unwrap();
        let slots: Vec<u8> = bank.snapshot().iter().map(|c| c.slot).collect();
        assert_eq!(slots, vec![2, 5, 8]);
    }

    #[test]
    fn set_format_only_updates_target_metadata_no_redecode() {
        let dir = TempDir::new().unwrap();
        let path = write_wav(dir.path(), "c.wav", 0.2, 44_100, 1, 0);
        let mut bank = CartBank::new(48_000, 2);
        bank.assign(1, "t".into(), path).unwrap();
        // The actual decode happened at native (44.1k mono). set_format
        // should NOT re-decode; cart samples remain at native format.
        let len_before = {
            let snap = bank.snapshot();
            snap[0].duration_secs
        };
        bank.set_format(96_000, 1).unwrap();
        let len_after = bank.snapshot()[0].duration_secs;
        assert_eq!(
            len_before, len_after,
            "duration_secs is in source time, format change shouldn't alter it"
        );
    }
}
