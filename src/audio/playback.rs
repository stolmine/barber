use std::fmt;
use std::sync::{Arc, Mutex};

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, IOType};

use crate::audio::decode::AudioBuffer;
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
    pub edit_list: EditList,
    pub buffer: Arc<AudioBuffer>,
}

pub struct PlaybackEngine {
    state: Arc<Mutex<PlaybackState>>,
    _audio_unit: AudioUnit,
}

impl PlaybackEngine {
    pub fn new(buffer: Arc<AudioBuffer>, edit_list: EditList) -> Result<Self, PlaybackError> {
        let state = Arc::new(Mutex::new(PlaybackState {
            playing: false,
            position: 0,
            edit_list,
            buffer: buffer.clone(),
        }));

        let mut audio_unit = AudioUnit::new(IOType::DefaultOutput)?;

        let stream_format = audio_unit.output_stream_format()?;
        let device_channels = stream_format.channels as usize;
        let device_sample_rate = stream_format.sample_rate;
        let source_sample_rate = buffer.sample_rate as f64;
        let source_channels = buffer.channels as usize;

        let callback_state = Arc::clone(&state);

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

            for i in 0..num_frames {
                let pos = guard.position;
                if pos >= total {
                    guard.playing = false;
                    for (ch_idx, channel) in data.channels_mut().enumerate() {
                        if ch_idx < device_channels {
                            channel[i] = 0.0;
                        }
                    }
                    continue;
                }

                let source_frame = guard.edit_list.resolve(pos);

                for (ch_idx, channel) in data.channels_mut().enumerate() {
                    if ch_idx < device_channels {
                        channel[i] = match source_frame {
                            Some(sf) => {
                                let src_ch = ch_idx.min(source_channels - 1);
                                guard.buffer.samples[src_ch]
                                    .get(sf)
                                    .copied()
                                    .unwrap_or(0.0)
                            }
                            None => 0.0,
                        };
                    }
                }

                // Advance position, accounting for sample rate difference
                if rate_ratio <= 1.0 {
                    guard.position += 1;
                } else {
                    guard.position += rate_ratio as usize;
                }
            }

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
        }
    }

    pub fn position(&self) -> usize {
        self.state.lock().map(|s| s.position).unwrap_or(0)
    }

    pub fn is_playing(&self) -> bool {
        self.state.lock().map(|s| s.playing).unwrap_or(false)
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
