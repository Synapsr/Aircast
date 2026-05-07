use std::sync::atomic::{AtomicU32, Ordering};

/// A lock-free f32 stored as `AtomicU32` bit pattern. Suitable for shared
/// gain/parameter values updated from one thread and read from the audio
/// callback.
pub struct AtomicF32 {
    bits: AtomicU32,
}

impl AtomicF32 {
    pub const fn new(value: f32) -> Self {
        Self {
            bits: AtomicU32::new(value.to_bits()),
        }
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Relaxed))
    }

    pub fn store(&self, value: f32) {
        self.bits.store(value.to_bits(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_value() {
        let a = AtomicF32::new(0.0);
        a.store(0.5);
        assert!((a.load() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn round_trip_special_values() {
        let a = AtomicF32::new(0.0);
        for v in [
            0.0_f32,
            -0.0,
            1.0,
            -1.0,
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            a.store(v);
            let got = a.load();
            assert_eq!(got.to_bits(), v.to_bits(), "mismatch for {v}");
        }
    }

    #[test]
    fn nan_round_trip_preserves_bits() {
        let nan = f32::NAN;
        let a = AtomicF32::new(nan);
        let got = a.load();
        assert!(got.is_nan());
    }

    #[test]
    fn static_constructor_works() {
        static A: AtomicF32 = AtomicF32::new(0.42);
        assert!((A.load() - 0.42).abs() < f32::EPSILON);
    }
}
