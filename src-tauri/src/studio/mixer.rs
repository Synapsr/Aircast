use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::audio::playback::{self, MonitorRing};
use crate::studio::atomic_f32::AtomicF32;
use crate::studio::cart::CartBank;
use crate::studio::music::MusicPlayer;

const GAIN_SMOOTHING: f32 = 0.05; // ~50ms convergence at 48kHz with 1024-frame chunks

pub struct Mixer {
    studio_active: AtomicBool,
    mic_gain_target: AtomicF32,
    mic_gain: AtomicF32,
    music_gain_target: AtomicF32,
    music_gain: AtomicF32,
    cart_gain: AtomicF32,
    duck_amount: AtomicF32, // how much to drop music when mic open (0..1)
    pub music: Mutex<MusicPlayer>,
    pub carts: Mutex<CartBank>,
    pub monitor_muted: Arc<AtomicBool>,
    pub monitor_ring: MonitorRing,
}

impl Mixer {
    pub fn new(target_rate: u32, target_channels: u16) -> Self {
        Self {
            studio_active: AtomicBool::new(false),
            mic_gain_target: AtomicF32::new(1.0),
            mic_gain: AtomicF32::new(1.0),
            music_gain_target: AtomicF32::new(1.0),
            music_gain: AtomicF32::new(1.0),
            cart_gain: AtomicF32::new(1.0),
            duck_amount: AtomicF32::new(0.7),
            music: Mutex::new(MusicPlayer::new(target_rate, target_channels)),
            carts: Mutex::new(CartBank::new(target_rate, target_channels)),
            monitor_muted: Arc::new(AtomicBool::new(false)),
            monitor_ring: playback::make_ring(),
        }
    }

    pub fn set_monitor_muted(&self, muted: bool) {
        self.monitor_muted.store(muted, Ordering::Relaxed);
    }

    pub fn is_monitor_muted(&self) -> bool {
        self.monitor_muted.load(Ordering::Relaxed)
    }

    pub fn set_target_format(&self, target_rate: u32, target_channels: u16) {
        if let Ok(mut music) = self.music.lock() {
            music.set_format(target_rate, target_channels);
        }
        if let Ok(mut carts) = self.carts.lock() {
            let _ = carts.set_format(target_rate, target_channels);
        }
    }

    pub fn enable_studio(&self) {
        self.studio_active.store(true, Ordering::Relaxed);
        self.set_mic_open(false); // mic starts closed in studio
    }

    pub fn disable_studio(&self) {
        self.studio_active.store(false, Ordering::Relaxed);
        // reset to passthrough behavior: full mic, no duck
        self.mic_gain_target.store(1.0);
        self.music_gain_target.store(1.0);
        // also stop music + carts so they don't keep mixing
        if let Ok(mut music) = self.music.lock() {
            music.stop();
        }
        if let Ok(mut carts) = self.carts.lock() {
            carts.stop_all();
        }
    }

    #[allow(dead_code)]
    pub fn is_studio(&self) -> bool {
        self.studio_active.load(Ordering::Relaxed)
    }

    pub fn set_crossfade_seconds(&self, seconds: f32) {
        if let Ok(mut music) = self.music.lock() {
            music.set_crossfade_seconds(seconds);
        }
    }

    pub fn set_music_duration_override(&self, track_id: &str, duration_secs: f32) {
        if let Ok(mut music) = self.music.lock() {
            music.set_duration_override(track_id, duration_secs);
        }
    }

    pub fn set_mic_open(&self, open: bool) {
        if open {
            self.mic_gain_target.store(1.0);
            let duck = self.duck_amount.load();
            self.music_gain_target.store((1.0 - duck).clamp(0.0, 1.0));
        } else {
            self.mic_gain_target.store(0.0);
            self.music_gain_target.store(1.0);
        }
    }

    pub fn is_mic_open(&self) -> bool {
        self.mic_gain_target.load() > 0.5
    }

    pub fn set_duck_amount(&self, amount: f32) {
        self.duck_amount.store(amount.clamp(0.0, 1.0));
        // re-evaluate music target if mic is open
        if self.is_mic_open() {
            self.music_gain_target.store((1.0 - amount).clamp(0.0, 1.0));
        }
    }

    /// Mix mic + music + carts into `output`. `output` and `mic` are interleaved
    /// f32 PCM with the same length and channel layout.
    /// `music_scratch` and `cart_scratch` are caller-owned buffers reused across
    /// invocations to avoid per-callback allocation.
    pub fn process(
        &self,
        mic: &[f32],
        output: &mut Vec<f32>,
        music_scratch: &mut Vec<f32>,
        cart_scratch: &mut Vec<f32>,
    ) {
        let n = mic.len();
        output.clear();
        output.resize(n, 0.0);

        // Smooth gains toward their targets
        let mic_gain = step(&self.mic_gain, self.mic_gain_target.load());
        let music_gain = step(&self.music_gain, self.music_gain_target.load());
        let cart_gain = self.cart_gain.load();

        if !self.studio_active.load(Ordering::Relaxed) {
            // passthrough: just copy mic with smoothed mic gain
            for i in 0..n {
                output[i] = (mic[i] * mic_gain).clamp(-1.0, 1.0);
            }
            playback::push_to_ring(&self.monitor_ring, output);
            return;
        }

        music_scratch.clear();
        music_scratch.resize(n, 0.0);
        cart_scratch.clear();
        cart_scratch.resize(n, 0.0);

        if let Ok(mut music) = self.music.try_lock() {
            music.pull(music_scratch);
        }
        if let Ok(mut carts) = self.carts.try_lock() {
            carts.add_to(cart_scratch);
        }

        for i in 0..n {
            let m = mic[i] * mic_gain;
            let mu = music_scratch[i] * music_gain;
            let c = cart_scratch[i] * cart_gain;
            output[i] = (m + mu + c).clamp(-1.0, 1.0);
        }

        playback::push_to_ring(&self.monitor_ring, output);
    }
}

fn step(current: &AtomicF32, target: f32) -> f32 {
    let cur = current.load();
    if (cur - target).abs() < 1e-4 {
        current.store(target);
        return target;
    }
    let next = cur + (target - cur) * GAIN_SMOOTHING;
    current.store(next);
    next
}
