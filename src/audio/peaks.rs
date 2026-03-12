use rayon::prelude::*;

use super::decode::AudioBuffer;

const BASE_BLOCK_SIZE: usize = 256;
const NUM_LEVELS: usize = 14;

pub struct PeakData {
    levels: Vec<Vec<Vec<(f32, f32)>>>,
    base_block_size: usize,
}

impl PeakData {
    pub fn compute(buffer: &AudioBuffer) -> Self {
        let channels = buffer.channels as usize;
        let mut levels: Vec<Vec<Vec<(f32, f32)>>> = Vec::with_capacity(NUM_LEVELS);

        let level0: Vec<Vec<(f32, f32)>> = (0..channels)
            .map(|ch| {
                buffer.samples[ch]
                    .par_chunks(BASE_BLOCK_SIZE)
                    .map(|chunk| {
                        let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                        let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        (min, max)
                    })
                    .collect()
            })
            .collect();

        levels.push(level0);

        for level_idx in 1..NUM_LEVELS {
            let prev = &levels[level_idx - 1];
            let next: Vec<Vec<(f32, f32)>> = (0..channels)
                .map(|ch| {
                    prev[ch]
                        .chunks(2)
                        .map(|pair| {
                            let min = pair.iter().map(|&(mn, _)| mn).fold(f32::INFINITY, f32::min);
                            let max = pair
                                .iter()
                                .map(|&(_, mx)| mx)
                                .fold(f32::NEG_INFINITY, f32::max);
                            (min, max)
                        })
                        .collect()
                })
                .collect();

            if next.iter().all(|ch| ch.is_empty()) {
                break;
            }
            levels.push(next);
        }

        PeakData {
            levels,
            base_block_size: BASE_BLOCK_SIZE,
        }
    }

    pub fn get_peaks(
        &self,
        channel: usize,
        start_frame: usize,
        end_frame: usize,
        num_pixels: usize,
    ) -> Vec<(f32, f32)> {
        if num_pixels == 0 || end_frame <= start_frame {
            return vec![(0.0, 0.0); num_pixels];
        }

        let frame_span = end_frame - start_frame;
        let ideal_block_size = frame_span.max(1) / num_pixels.max(1);

        let level = self.select_level(ideal_block_size);
        let level_block_size = self.base_block_size << level;
        let level_data = &self.levels[level][channel];

        (0..num_pixels)
            .map(|px| {
                let frame_start = start_frame + (frame_span * px) / num_pixels;
                let frame_end = start_frame + (frame_span * (px + 1)) / num_pixels;

                let block_start = frame_start / level_block_size;
                let block_end = (frame_end + level_block_size - 1) / level_block_size;

                let block_start = block_start.min(level_data.len());
                let block_end = block_end.min(level_data.len());

                if block_start >= block_end {
                    return (0.0, 0.0);
                }

                let min = level_data[block_start..block_end]
                    .iter()
                    .map(|&(mn, _)| mn)
                    .fold(f32::INFINITY, f32::min);
                let max = level_data[block_start..block_end]
                    .iter()
                    .map(|&(_, mx)| mx)
                    .fold(f32::NEG_INFINITY, f32::max);

                if min == f32::INFINITY {
                    (0.0, 0.0)
                } else {
                    (min, max)
                }
            })
            .collect()
    }

    pub fn channels(&self) -> usize {
        self.levels.first().map(|l| l.len()).unwrap_or(0)
    }

    fn select_level(&self, ideal_block_size: usize) -> usize {
        let num_levels = self.levels.len();
        if ideal_block_size <= self.base_block_size {
            return 0;
        }
        for lvl in 1..num_levels {
            let block_size = self.base_block_size << lvl;
            if block_size >= ideal_block_size {
                return lvl;
            }
        }
        num_levels - 1
    }
}
