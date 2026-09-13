use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::logging;

pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
    started: Instant,
}
pub fn encode_wav(samples: &[i16], spec: hound::WavSpec) -> Result<Vec<u8>, String> {
    let sr = spec.sample_rate;
    let ch = spec.channels as u32;
    let bits = spec.bits_per_sample as u16;
    let block_align = (ch * bits as u32 / 8) as u16;
    let byte_rate = sr * ch * bits as u32 / 8;
    let data_len = (samples.len() * (bits as usize / 8)) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&(ch as u16).to_le_bytes());
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    Ok(out)
}

pub fn start() -> Result<Recorder, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no microphone found".to_string())?;
    let cfg = device
        .default_input_config()
        .map_err(|e| format!("cannot query mic config: {e}"))?;
    let sample_rate = cfg.sample_rate().0;
    let channels = cfg.channels();
    let sample_format = cfg.sample_format();

    let buffer = Arc::new(Mutex::new(Vec::<i16>::new()));
    let buf = buffer.clone();
    let err_fn = |err: cpal::StreamError| logging::log(&format!("audio stream error: {err}"));

    let stream = match sample_format {
        cpal::SampleFormat::I16 => device.build_input_stream(
            &cfg.clone().into(),
            move |d: &[i16], _| {
                if let Ok(mut b) = buf.lock() {
                    b.extend_from_slice(d);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &cfg.clone().into(),
            move |d: &[u16], _| {
                if let Ok(mut b) = buf.lock() {
                    b.extend(d.iter().map(|s| (*s as i32 - 32768) as i16));
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::F32 => device.build_input_stream(
            &cfg.clone().into(),
            move |d: &[f32], _| {
                if let Ok(mut b) = buf.lock() {
                    b.extend(d.iter().map(|s| (s * 32767.0) as i16));
                }
            },
            err_fn,
            None,
        ),
        other => return Err(format!("unsupported mic sample format: {other:?}")),
    }
    .map_err(|e| format!("failed to open mic stream: {e}"))?;

    stream.play().map_err(|e| format!("failed to start mic: {e}"))?;

    Ok(Recorder {
        stream,
        buffer,
        sample_rate,
        channels,
        started: Instant::now(),
    })
}

impl Recorder {
    /// Stop capture, encode captured PCM as 16-bit WAV bytes.
    pub fn stop(self) -> Result<(Vec<u8>, Duration), String> {
        drop(self.stream);
        let samples = self
            .buffer
            .lock()
            .map_err(|e| format!("buffer poisoned: {e}"))?
            .clone();
        let dur = self.started.elapsed();
        if samples.is_empty() {
            return Err("no audio captured".into());
        }
        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        Ok((encode_wav(&samples, spec)?, dur))
    }
}
