use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const EMIT_INTERVAL_MS: u128 = 50; // ~20 Hz

#[derive(Debug, Clone, Serialize)]
pub struct VuPayload {
    pub level: f32,
}

pub struct VuEmitter {
    app: AppHandle,
    last_emit: Instant,
    accumulated_max: f32,
}

impl VuEmitter {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            last_emit: Instant::now(),
            accumulated_max: 0.0,
        }
    }

    pub fn push(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }

        let mut peak: f32 = 0.0;
        let mut sum_sq: f32 = 0.0;
        for &s in samples {
            let abs = s.abs();
            if abs > peak {
                peak = abs;
            }
            sum_sq += s * s;
        }
        let rms = (sum_sq / samples.len() as f32).sqrt();
        // blend RMS (perceived loudness) with peak (responsiveness)
        let level = (rms * 0.7 + peak * 0.3).min(1.0);

        if level > self.accumulated_max {
            self.accumulated_max = level;
        }

        if self.last_emit.elapsed().as_millis() >= EMIT_INTERVAL_MS {
            let _ = self.app.emit(
                "vu-meter",
                VuPayload {
                    level: self.accumulated_max,
                },
            );
            self.last_emit = Instant::now();
            self.accumulated_max = 0.0;
        }
    }
}

pub fn emit_zero(app: &AppHandle) {
    let _ = app.emit("vu-meter", VuPayload { level: 0.0 });
}
