use std::fmt;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioBuffer {
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub num_frames: usize,
}

#[derive(Debug)]
pub enum DecodeError {
    Io(std::io::Error),
    Symphonia(SymphoniaError),
    NoAudioTrack,
    UnsupportedFormat,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Io(e) => write!(f, "IO error: {}", e),
            DecodeError::Symphonia(e) => write!(f, "Symphonia error: {}", e),
            DecodeError::NoAudioTrack => write!(f, "No audio track found"),
            DecodeError::UnsupportedFormat => write!(f, "Unsupported audio format"),
        }
    }
}

impl From<std::io::Error> for DecodeError {
    fn from(e: std::io::Error) -> Self {
        DecodeError::Io(e)
    }
}

impl From<SymphoniaError> for DecodeError {
    fn from(e: SymphoniaError) -> Self {
        DecodeError::Symphonia(e)
    }
}

pub fn decode_file(path: &Path) -> Result<AudioBuffer, DecodeError> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(DecodeError::Symphonia)?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(DecodeError::NoAudioTrack)?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let channels = codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    let sample_rate = codec_params.sample_rate.unwrap_or(44100);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(DecodeError::Symphonia)?;

    let mut channel_samples: Vec<Vec<f32>> = vec![Vec::new(); channels as usize];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(DecodeError::Symphonia(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(DecodeError::Symphonia(e)),
        };

        append_samples(&decoded, &mut channel_samples);
    }

    let num_frames = channel_samples.first().map(|c| c.len()).unwrap_or(0);

    Ok(AudioBuffer {
        samples: channel_samples,
        sample_rate,
        channels,
        num_frames,
    })
}

fn append_samples(decoded: &AudioBufferRef<'_>, channel_samples: &mut Vec<Vec<f32>>) {
    match decoded {
        AudioBufferRef::F32(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend_from_slice(buf.chan(ch));
                }
            }
        }
        AudioBufferRef::F64(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(buf.chan(ch).iter().map(|&s| s as f32));
                }
            }
        }
        AudioBufferRef::S32(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(buf.chan(ch).iter().map(|&s| s as f32 / i32::MAX as f32));
                }
            }
        }
        AudioBufferRef::S24(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(buf.chan(ch).iter().map(|&s| s.inner() as f32 / 8_388_607.0));
                }
            }
        }
        AudioBufferRef::S16(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(buf.chan(ch).iter().map(|&s| s as f32 / i16::MAX as f32));
                }
            }
        }
        AudioBufferRef::S8(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(buf.chan(ch).iter().map(|&s| s as f32 / i8::MAX as f32));
                }
            }
        }
        AudioBufferRef::U32(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(
                        buf.chan(ch)
                            .iter()
                            .map(|&s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0),
                    );
                }
            }
        }
        AudioBufferRef::U24(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(
                        buf.chan(ch)
                            .iter()
                            .map(|&s| (s.inner() as f32 / 16_777_215.0) * 2.0 - 1.0),
                    );
                }
            }
        }
        AudioBufferRef::U16(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(
                        buf.chan(ch)
                            .iter()
                            .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                    );
                }
            }
        }
        AudioBufferRef::U8(buf) => {
            for (ch, dst) in channel_samples.iter_mut().enumerate() {
                if ch < buf.spec().channels.count() {
                    dst.extend(
                        buf.chan(ch)
                            .iter()
                            .map(|&s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0),
                    );
                }
            }
        }
    }
}
