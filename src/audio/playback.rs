use std::fmt;
use std::sync::{Arc, Mutex};

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, IOType};

use crate::audio::decode::AudioBuffer;
use crate::audio::levels::AudioLevels;
use crate::edit::EditList;

#[derive(Debug)]
pub enum PlaybackError {
    CoreAudio(coreaudio::Error),
}

impl fmt::Display for PlaybackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaybackError::CoreAudio(e) => write!(f, "CoreAudio error: {:?}", e),
        }
    }
}

impl From<coreaudio::Error> for PlaybackError {
    fn from(e: coreaudio::Error) -> Self {
        PlaybackError::CoreAudio(e)
    }
}

pub struct PlaybackState {
    pub playing: bool,
    pub position: usize,
    pub position_frac: f64,
    pub edit_list: EditList,
    pub buffer: Arc<AudioBuffer>,
    pub loop_enabled: bool,
    pub loop_start: usize,
    pub loop_end: Option<usize>,
    pub stop_at: Option<usize>,
}

pub struct PlaybackEngine {
    state: Arc<Mutex<PlaybackState>>,
    _audio_unit: AudioUnit,
}

impl PlaybackEngine {
    pub fn new(buffer: Arc<AudioBuffer>, edit_list: EditList, levels: AudioLevels) -> Result<Self, PlaybackError> {
        let state = Arc::new(Mutex::new(PlaybackState {
            playing: false,
            position: 0,
            position_frac: 0.0,
            edit_list,
            buffer: buffer.clone(),
            loop_enabled: false,
            loop_start: 0,
            loop_end: None,
            stop_at: None,
        }));

        let mut audio_unit = AudioUnit::new(IOType::DefaultOutput)?;

        let stream_format = audio_unit.output_stream_format()?;
        let device_channels = stream_format.channels as usize;
        let device_sample_rate = stream_format.sample_rate;
        let source_sample_rate = buffer.sample_rate as f64;
        let source_channels = buffer.channels as usize;

        let callback_state = Arc::clone(&state);
        let callback_levels = levels;

        type Args = render_callback::Args<data::NonInterleaved<f32>>;
        audio_unit.set_render_callback(move |args: Args| {
            let Args {
                num_frames,
                mut data,
                ..
            } = args;

            let mut guard = match callback_state.try_lock().ok().filter(|g| g.playing) {
                Some(g) => g,
                None => {
                    for channel in data.channels_mut() {
                        for sample in channel.iter_mut() {
                            *sample = 0.0;
                        }
                    }
                    return Ok(());
                }
            };

            let total = guard.edit_list.total_frames();
            let rate_ratio = source_sample_rate / device_sample_rate;
            let vol = callback_levels.volume();
            let mut meter_l: f32 = 0.0;
            let mut meter_r: f32 = 0.0;

            for i in 0..num_frames {
                let pos = guard.position;
                let boundary = guard.stop_at.unwrap_or(
                    guard.loop_end.filter(|_| guard.loop_enabled).unwrap_or(total)
                );
                if pos >= boundary {
                    if guard.loop_enabled && guard.stop_at.is_none() {
                        guard.position = guard.loop_start;
                        guard.position_frac = 0.0;
                        continue;
                    }
                    guard.playing = false;
                    for (ch_idx, channel) in data.channels_mut().enumerate() {
                        if ch_idx < device_channels {
                            channel[i] = 0.0;
                        }
                    }
                    continue;
                }

                let resolved = guard.edit_list.resolve(pos);

                for (ch_idx, channel) in data.channels_mut().enumerate() {
                    if ch_idx < device_channels {
                        let pre_fader = match resolved {
                            Some((sf, gain, dc_offset)) => {
                                let src_ch = ch_idx.min(source_channels - 1);
                                (guard.buffer.samples[src_ch]
                                    .get(sf)
                                    .copied()
                                    .unwrap_or(0.0)
                                    - dc_offset)
                                    * gain
                            }
                            None => 0.0,
                        };
                        let abs = pre_fader.abs();
                        if ch_idx == 0 { meter_l = meter_l.max(abs); }
                        else if ch_idx == 1 { meter_r = meter_r.max(abs); }
                        channel[i] = pre_fader * vol;
                    }
                }

                guard.position_frac += rate_ratio;
                let advance = guard.position_frac as usize;
                guard.position += advance;
                guard.position_frac -= advance as f64;
            }

            if source_channels == 1 { meter_r = meter_l; }
            callback_levels.set_peaks(meter_l, meter_r);

            Ok(())
        })?;

        audio_unit.start()?;

        Ok(PlaybackEngine {
            state,
            _audio_unit: audio_unit,
        })
    }

    pub fn play(&self) {
        if let Ok(mut s) = self.state.lock() {
            if s.position >= s.edit_list.total_frames() {
                s.position = 0;
            }
            s.stop_at = None;
            s.playing = true;
        }
    }

    pub fn pause(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.playing = false;
        }
    }

    pub fn stop(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.playing = false;
            s.position = 0;
        }
    }

    pub fn seek(&self, frame: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.position = frame;
            s.position_frac = 0.0;
        }
    }

    pub fn position(&self) -> usize {
        self.state.lock().map(|s| s.position).unwrap_or(0)
    }

    pub fn is_playing(&self) -> bool {
        self.state.lock().map(|s| s.playing).unwrap_or(false)
    }

    pub fn set_loop(&self, enabled: bool, start: usize, end: Option<usize>) {
        if let Ok(mut s) = self.state.lock() {
            s.loop_enabled = enabled;
            s.loop_start = start;
            s.loop_end = end;
        }
    }

    pub fn set_stop_at(&self, frame: Option<usize>) {
        if let Ok(mut s) = self.state.lock() {
            s.stop_at = frame;
        }
    }

    pub fn set_edit_list(&self, edit_list: EditList) {
        if let Ok(mut s) = self.state.lock() {
            s.edit_list = edit_list;
            if s.position >= s.edit_list.total_frames() {
                s.position = 0;
                s.playing = false;
            }
        }
    }
}
