use std::io;
use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::*;
use super::{engine::*, resample::*, source::*};
use crate::shared::kernel::app::mutex;

pub(super) fn fill(shared: &Shared, data: &mut [f32], channels: usize) {
    let volume = if shared.muted.load(Ordering::Relaxed) {
        0.0
    } else {
        shared.volume()
    };

    if !shared.playing.load(Ordering::Relaxed) {
        data.fill(0.0);
        return;
    }

    let mut buffer = match shared.buffer.try_lock() {
        Ok(buffer) => buffer,
        Err(_) => {
            data.fill(0.0);
            return;
        }
    };

    let available = buffer.len().min(data.len());
    for (sample, value) in data.iter_mut().zip(buffer.drain(..available)) {
        *sample = value * volume;
    }

    if available < data.len() {
        data[available..].fill(0.0);
    }

    shared
        .played
        .fetch_add(available as u64 / channels.max(1) as u64, Ordering::Relaxed);

    if buffer.is_empty() && shared.eos.load(Ordering::Relaxed) {
        shared.ended.store(true, Ordering::SeqCst);
        shared.playing.store(false, Ordering::SeqCst);
    }
}

pub(super) fn run_decoder(shared: Arc<Shared>, commands: mpsc::Receiver<Command>) {
    loop {
        let command = match commands.recv() {
            Ok(command) => command,
            Err(_) => return,
        };

        match command {
            Command::Load {
                url,
                agent,
                start,
                epoch,
            } => {
                if let Err(error) = decode(&shared, &commands, &url, &agent, start, epoch) {
                    log::warn!("playback stopped: {error}");
                    shared.eos.store(true, Ordering::SeqCst);
                }
            }
            Command::Stop => {
                mutex::lock(&shared.buffer).clear();
                shared.seeking.store(false, Ordering::SeqCst);
            }
            Command::Seek(_) => {
                shared.seeking.store(false, Ordering::SeqCst);
            }
        }
    }
}

pub(super) fn decode(
    shared: &Arc<Shared>,
    commands: &mpsc::Receiver<Command>,
    url: &str,
    agent: &str,
    start: f64,
    epoch: u64,
) -> Result<(), String> {
    FIRST_PACKET.store(false, Ordering::Relaxed);
    let source = HttpSource::new(url, agent)?;
    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("m4a");

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|error| format!("cannot probe stream: {error}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "stream has no audio track".to_owned())?;
    let track_id = track.id;
    let source_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "stream has no sample rate".to_owned())?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| format!("unsupported codec: {error}"))?;

    let dst_rate = shared.rate.load(Ordering::Relaxed);
    let dst_channels = shared.channels.load(Ordering::Relaxed) as usize;
    let mut resampler = Resampler::new(source_rate, dst_rate, dst_channels);
    let capacity = (dst_rate as f32 * dst_channels as f32 * BUFFER_SECONDS) as usize;

    shared
        .base_ms
        .store((start.max(0.0) * 1000.0) as u64, Ordering::SeqCst);
    shared.played.store(0, Ordering::SeqCst);
    shared.eos.store(false, Ordering::SeqCst);
    shared.ended.store(false, Ordering::SeqCst);
    mutex::lock(&shared.buffer).clear();

    if start > 0.2 {
        let seek = symphonia::core::formats::SeekTo::Time {
            time: symphonia::core::units::Time::from(start),
            track_id: Some(track_id),
        };

        if let Err(error) = format.seek(symphonia::core::formats::SeekMode::Accurate, seek) {
            log::warn!("seek failed: {error}");
        }
    }

    loop {
        let mut pending_seek = None;
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Stop | Command::Load { .. } => {
                    shared.seeking.store(false, Ordering::SeqCst);
                    return Ok(());
                }
                Command::Seek(position) => pending_seek = Some(position),
            }
        }

        if let Some(position) = pending_seek {
            let seek = symphonia::core::formats::SeekTo::Time {
                time: symphonia::core::units::Time::from(position),
                track_id: Some(track_id),
            };

            let started = std::time::Instant::now();
            match format.seek(symphonia::core::formats::SeekMode::Coarse, seek) {
                Ok(_) => {
                    log::debug!("audio: seek to {position:.1}s took {:?}", started.elapsed());
                    mutex::lock(&shared.buffer).clear();
                    resampler.reset();
                    shared
                        .base_ms
                        .store((position.max(0.0) * 1000.0) as u64, Ordering::SeqCst);
                    shared.played.store(0, Ordering::SeqCst);
                    shared.eos.store(false, Ordering::SeqCst);
                    shared.ended.store(false, Ordering::SeqCst);
                }
                Err(error) => {
                    log::warn!("seek failed: {error}");
                    shared.eos.store(true, Ordering::SeqCst);
                }
            }

            shared.seeking.store(false, Ordering::SeqCst);
        }

        if mutex::lock(&shared.buffer).len() >= capacity {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                shared.eos.store(true, Ordering::SeqCst);
                return Ok(());
            }
            Err(SymphoniaError::ResetRequired) => return Ok(()),
            Err(error) => return Err(format!("read error: {error}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => {
                if !FIRST_PACKET.swap(true, Ordering::Relaxed) {
                    log::debug!("audio: first packet decoded");
                }

                decoded
            }
            Err(SymphoniaError::DecodeError(error)) => {
                log::warn!("decode error: {error}");
                continue;
            }
            Err(error) => return Err(format!("decode error: {error}")),
        };

        let spec = *decoded.spec();
        let frames = decoded.frames();
        let mut samples = SampleBuffer::<f32>::new(frames as u64, spec);
        samples.copy_interleaved_ref(decoded);

        // A stop or a new load may have landed while this packet was decoding:
        // never let the old stream seed the buffer that feeds `started`.
        if shared.epoch.load(Ordering::SeqCst) != epoch {
            return Ok(());
        }

        let mut buffer = mutex::lock(&shared.buffer);
        resampler.push(samples.samples(), spec.channels.count(), &mut buffer);
        drop(buffer);
    }
}
