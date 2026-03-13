use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioLevels {
    volume: Arc<AtomicU32>,
    peak_l: Arc<AtomicU32>,
    peak_r: Arc<AtomicU32>,
    display_l: Arc<AtomicU32>,
    display_r: Arc<AtomicU32>,
}

impl AudioLevels {
    pub fn new() -> Self {
        Self {
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            peak_l: Arc::new(AtomicU32::new(0u32)),
            peak_r: Arc::new(AtomicU32::new(0u32)),
            display_l: Arc::new(AtomicU32::new(0u32)),
            display_r: Arc::new(AtomicU32::new(0u32)),
        }
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Relaxed))
    }

    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.to_bits(), Relaxed);
    }

    pub fn set_peaks(&self, l: f32, r: f32) {
        self.peak_l.store(l.to_bits(), Relaxed);
        self.peak_r.store(r.to_bits(), Relaxed);
    }

    /// Returns smoothed peak values with exponential decay. Call once per UI frame.
    pub fn smoothed_peaks(&self) -> (f32, f32) {
        let raw_l = f32::from_bits(self.peak_l.load(Relaxed));
        let raw_r = f32::from_bits(self.peak_r.load(Relaxed));
        let prev_l = f32::from_bits(self.display_l.load(Relaxed));
        let prev_r = f32::from_bits(self.display_r.load(Relaxed));
        let l = raw_l.max(prev_l * 0.85);
        let r = raw_r.max(prev_r * 0.85);
        self.display_l.store(l.to_bits(), Relaxed);
        self.display_r.store(r.to_bits(), Relaxed);
        (l, r)
    }
}
