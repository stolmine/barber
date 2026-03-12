use std::fmt;
use hound::{SampleFormat, WavSpec, WavWriter};
use crate::audio::decode::AudioBuffer;
use crate::edit::EditList;

pub enum ExportError {
    Io(hound::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "WAV export error: {}", e),
        }
    }
}

impl From<hound::Error> for ExportError {
    fn from(e: hound::Error) -> Self {
        ExportError::Io(e)
    }
}

pub fn export_wav(
    path: &std::path::Path,
    buffer: &AudioBuffer,
    edit_list: &EditList,
) -> Result<(), ExportError> {
    let spec = WavSpec {
        channels: buffer.channels,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    for maybe_frame in edit_list.iter_source_frames(0, edit_list.total_frames()) {
        for ch in 0..buffer.channels as usize {
            let pcm = match maybe_frame {
                Some((source_frame, gain, dc_offset)) => {
                    let sample = (buffer.samples[ch][source_frame] - dc_offset) * gain;
                    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                }
                None => 0i16,
            };
            writer.write_sample(pcm)?;
        }
    }

    writer.finalize()?;
    Ok(())
}
