/// Sample-and-hold rate converter, driven one target frame at a time.
///
/// The caller supplies a `read_source` closure that pulls one sample at a
/// time from whatever source they have (a streamed decoder, a `Vec<f32>`
/// slice, a network buffer, …). The resampler decides how many source
/// frames to consume for each target frame using a single integer formula:
///
///     desired_source_idx = floor(target_idx × source_rate / target_rate)
///
/// This is correct for any source and target format — mono, stereo,
/// multi-channel, low or high sample rate. The arithmetic uses one `f64`
/// multiply per target frame and otherwise stays in `u64`.
pub struct FrameResampler {
    source_rate: u32,
    source_channels: u16,
    /// Number of source frames fully read into `last_frame` so far.
    source_frames_read: u64,
    /// Number of target frames already emitted since the resampler was
    /// constructed (or since `reset_target` was called).
    target_emitted: u64,
    /// Last fully-consumed source frame, ready to be emitted as long as
    /// the desired source index hasn't moved past it.
    last_frame: Vec<f32>,
}

impl FrameResampler {
    pub fn new(source_rate: u32, source_channels: u16) -> Self {
        Self {
            source_rate,
            source_channels,
            source_frames_read: 0,
            target_emitted: 0,
            last_frame: vec![0.0; source_channels.max(1) as usize],
        }
    }

    pub fn source_rate(&self) -> u32 {
        self.source_rate
    }
    pub fn last_frame(&self) -> &[f32] {
        &self.last_frame
    }
    pub fn source_frames_read(&self) -> u64 {
        self.source_frames_read
    }

    /// Try to emit one target frame. Returns `true` on success (the caller
    /// should now read `last_frame()` and write it to its output) or
    /// `false` when the source ran out mid-frame.
    pub fn step<F>(&mut self, target_rate: u32, mut read_source: F) -> bool
    where
        F: FnMut() -> Option<f32>,
    {
        let chs = self.source_channels.max(1) as usize;
        let step = self.source_rate.max(1) as f64 / target_rate.max(1) as f64;
        // Source frame index that should be heard for this target frame.
        let desired = (self.target_emitted as f64 * step) as u64;

        // Read source frames until we've read past `desired` (so that
        // `last_frame` is the source frame at index `desired`).
        while self.source_frames_read <= desired {
            for c in 0..chs {
                match read_source() {
                    Some(s) => self.last_frame[c] = s,
                    None => return false,
                }
            }
            self.source_frames_read += 1;
        }

        self.target_emitted += 1;
        true
    }
}

/// Write `input` (interleaved at `input.len()` channels) into `output`
/// (interleaved at `output.len()` channels), assigning. Mono → broadcast,
/// multi → mono = average of first two channels, otherwise channel-wise
/// copy with the last source channel duplicated as a fallback.
pub fn map_channels_set(input: &[f32], output: &mut [f32], gain: f32) {
    let in_ch = input.len();
    let out_ch = output.len();
    if in_ch == 0 || out_ch == 0 {
        for o in output.iter_mut() {
            *o = 0.0;
        }
        return;
    }
    if in_ch == 1 {
        let v = input[0] * gain;
        for o in output.iter_mut() {
            *o = v;
        }
        return;
    }
    if out_ch == 1 {
        if in_ch >= 2 {
            output[0] = (input[0] + input[1]) * 0.5 * gain;
        } else {
            output[0] = input[0] * gain;
        }
        return;
    }
    for (i, o) in output.iter_mut().enumerate() {
        *o = input[i.min(in_ch - 1)] * gain;
    }
}

/// Same as [`map_channels_set`] but additive.
pub fn map_channels_add(input: &[f32], output: &mut [f32], gain: f32) {
    let in_ch = input.len();
    let out_ch = output.len();
    if in_ch == 0 || out_ch == 0 {
        return;
    }
    if in_ch == 1 {
        let v = input[0] * gain;
        for o in output.iter_mut() {
            *o += v;
        }
        return;
    }
    if out_ch == 1 {
        if in_ch >= 2 {
            output[0] += (input[0] + input[1]) * 0.5 * gain;
        } else {
            output[0] += input[0] * gain;
        }
        return;
    }
    for (i, o) in output.iter_mut().enumerate() {
        *o += input[i.min(in_ch - 1)] * gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull `count` samples from a slice via the resampler `step` API. Returns
    /// the per-target-frame outputs as `Vec<Vec<f32>>` so we can assert on
    /// the conversion.
    fn drive(
        resampler: &mut FrameResampler,
        target_rate: u32,
        target_chs: usize,
        source: &[f32],
        count: usize,
    ) -> Vec<Vec<f32>> {
        let mut idx = 0;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let mut frame = vec![0.0; target_chs];
            let alive = resampler.step(target_rate, || {
                if idx < source.len() {
                    let v = source[idx];
                    idx += 1;
                    Some(v)
                } else {
                    None
                }
            });
            if !alive {
                break;
            }
            map_channels_set(resampler.last_frame(), &mut frame, 1.0);
            out.push(frame);
        }
        out
    }

    #[test]
    fn step_one_to_one_mono() {
        // source 48k mono → target 48k mono: each source frame heard once.
        let mut r = FrameResampler::new(48_000, 1);
        let src: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let frames = drive(&mut r, 48_000, 1, &src, 10);
        let flat: Vec<f32> = frames.iter().flatten().copied().collect();
        assert_eq!(flat, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    }

    #[test]
    fn step_upsample_2x_repeats_each_source_frame_twice() {
        // source 24k mono → target 48k mono: step=0.5, each source frame heard twice.
        let mut r = FrameResampler::new(24_000, 1);
        let src: Vec<f32> = vec![10.0, 20.0, 30.0, 40.0];
        let frames = drive(&mut r, 48_000, 1, &src, 8);
        let flat: Vec<f32> = frames.iter().flatten().copied().collect();
        // Source idx for target 0..7 with step=0.5: 0,0,1,1,2,2,3,3
        assert_eq!(flat, vec![10.0, 10.0, 20.0, 20.0, 30.0, 30.0, 40.0, 40.0]);
    }

    #[test]
    fn step_downsample_2x_skips_every_other_source_frame() {
        // source 96k mono → target 48k mono: step=2.0, every other source frame heard.
        let mut r = FrameResampler::new(96_000, 1);
        let src: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let frames = drive(&mut r, 48_000, 1, &src, 5);
        let flat: Vec<f32> = frames.iter().flatten().copied().collect();
        // Source idx for target 0..4 with step=2.0: 0,2,4,6,8
        assert_eq!(flat, vec![0.0, 2.0, 4.0, 6.0, 8.0]);
    }

    #[test]
    fn step_44100_to_48000_consumes_about_44100_per_48000_target() {
        // The most common production case (CD-rate music → CoreAudio default).
        let mut r = FrameResampler::new(44_100, 1);
        let src: Vec<f32> = vec![1.0; 50_000];
        let _ = drive(&mut r, 48_000, 1, &src, 48_000);
        // After 48000 target frames at step ≈ 0.91875, we should have read
        // approximately 44100 source frames (±1 due to truncation).
        let read = r.source_frames_read();
        assert!(
            (44_099..=44_101).contains(&read),
            "expected ~44100 source frames consumed, got {read}"
        );
    }

    #[test]
    fn step_returns_false_when_source_exhausted() {
        let mut r = FrameResampler::new(48_000, 1);
        let src: Vec<f32> = vec![1.0, 2.0];
        // Try to pull 5 target frames from only 2 source samples; expect early exit.
        let frames = drive(&mut r, 48_000, 1, &src, 5);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn map_channels_set_mono_to_stereo_broadcasts() {
        let mut out = [0.0f32; 2];
        map_channels_set(&[0.5], &mut out, 1.0);
        assert_eq!(out, [0.5, 0.5]);
    }

    #[test]
    fn map_channels_set_stereo_to_mono_averages() {
        let mut out = [0.0f32; 1];
        map_channels_set(&[0.4, 0.6], &mut out, 1.0);
        // (0.4 + 0.6) / 2 = 0.5
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn map_channels_set_stereo_to_stereo_passes_through() {
        let mut out = [0.0f32; 2];
        map_channels_set(&[0.3, 0.7], &mut out, 1.0);
        assert_eq!(out, [0.3, 0.7]);
    }

    #[test]
    fn map_channels_set_applies_gain() {
        let mut out = [0.0f32; 2];
        map_channels_set(&[1.0], &mut out, 0.25);
        assert_eq!(out, [0.25, 0.25]);
    }

    #[test]
    fn map_channels_set_handles_empty_input() {
        let mut out = [1.0f32, 2.0, 3.0];
        map_channels_set(&[], &mut out, 1.0);
        assert_eq!(out, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn map_channels_add_accumulates() {
        let mut out = [1.0f32, 2.0];
        map_channels_add(&[0.5], &mut out, 1.0);
        // mono → both channels added
        assert_eq!(out, [1.5, 2.5]);
    }

    #[test]
    fn map_channels_add_mono_target_averages_stereo_input_with_gain() {
        let mut out = [1.0f32];
        map_channels_add(&[0.4, 0.6], &mut out, 0.5);
        // ((0.4+0.6)/2) * 0.5 = 0.25 → 1.0 + 0.25 = 1.25
        assert!((out[0] - 1.25).abs() < 1e-6);
    }

    #[test]
    fn map_channels_add_zero_gain_is_noop() {
        let mut out = [1.0f32, 2.0];
        map_channels_add(&[5.0, 5.0], &mut out, 0.0);
        assert_eq!(out, [1.0, 2.0]);
    }

    #[test]
    fn map_channels_set_5_1_to_stereo_falls_back_to_first_two_channels() {
        // Generic case: 6-channel input, 2-channel output
        let mut out = [0.0f32; 2];
        map_channels_set(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6], &mut out, 1.0);
        assert_eq!(out, [0.1, 0.2]);
    }
}
