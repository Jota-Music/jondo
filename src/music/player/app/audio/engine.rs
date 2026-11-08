use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::decoder::*;
use super::*;
use crate::shared::kernel::app::mutex;

pub(super) enum Command {
    Load {
        url: String,
        agent: String,
        start: f64,
        epoch: u64,
    },
    Seek(f64),
    Stop,
}

pub(super) struct Shared {
    pub(super) buffer: Mutex<VecDeque<f32>>,
    pub(super) playing: AtomicBool,
    pub(super) volume: AtomicU32,
    pub(super) muted: AtomicBool,
    pub(super) base_ms: AtomicU64,
    pub(super) played: AtomicU64,
    pub(super) rate: AtomicU32,
    pub(super) channels: AtomicU32,
    pub(super) eos: AtomicBool,
    pub(super) ended: AtomicBool,
    pub(super) seeking: AtomicBool,
    pub(super) epoch: AtomicU64,
}

impl Shared {
    pub(super) fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
}

pub struct Engine {
    shared: Arc<Shared>,
    commands: mpsc::Sender<Command>,
    _stream: cpal::Stream,
}

impl Engine {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no audio output device".to_owned())?;
        let config = pick_config(&device)?;
        let rate = config.sample_rate();
        let channels = config.channels();
        let sample_format = config.sample_format();
        let shared = Arc::new(Shared {
            buffer: Mutex::new(VecDeque::with_capacity(
                (rate as f32 * channels as f32 * BUFFER_SECONDS) as usize,
            )),
            playing: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            muted: AtomicBool::new(false),
            base_ms: AtomicU64::new(0),
            played: AtomicU64::new(0),
            rate: AtomicU32::new(rate),
            channels: AtomicU32::new(channels as u32),
            eos: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            seeking: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
        });

        let stream = build_stream(&device, &config, sample_format, shared.clone())?;
        stream.play().map_err(|error| error.to_string())?;

        let (commands, receiver) = mpsc::channel();
        let decoder_shared = shared.clone();
        thread::Builder::new()
            .name("jondo-decoder".to_owned())
            .spawn(move || run_decoder(decoder_shared, receiver))
            .map_err(|error| error.to_string())?;

        Ok(Self {
            shared,
            commands,
            _stream: stream,
        })
    }

    pub fn load(&self, url: &str, agent: &str, start: f64) {
        let _ = self.commands.send(Command::Stop);
        let epoch = self.shared.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        mutex::lock(&self.shared.buffer).clear();
        self.shared
            .base_ms
            .store((start.max(0.0) * 1000.0) as u64, Ordering::SeqCst);
        self.shared.played.store(0, Ordering::SeqCst);
        self.shared.eos.store(false, Ordering::SeqCst);
        self.shared.ended.store(false, Ordering::SeqCst);
        self.shared.seeking.store(false, Ordering::SeqCst);
        self.shared.playing.store(true, Ordering::SeqCst);
        let _ = self.commands.send(Command::Load {
            url: url.to_owned(),
            agent: agent.to_owned(),
            start,
            epoch,
        });
    }

    pub fn load_paused(&self, url: &str, agent: &str, start: f64) {
        self.load(url, agent, start);
        self.shared.playing.store(false, Ordering::SeqCst);
    }

    pub fn play(&self, playing: bool) {
        self.shared.playing.store(playing, Ordering::SeqCst);
    }

    pub fn seek(&self, seconds: f64) {
        let seconds = seconds.max(0.0);
        self.shared.seeking.store(true, Ordering::SeqCst);
        self.shared
            .base_ms
            .store((seconds * 1000.0) as u64, Ordering::SeqCst);
        self.shared.played.store(0, Ordering::SeqCst);
        self.shared.eos.store(false, Ordering::SeqCst);
        self.shared.ended.store(false, Ordering::SeqCst);
        mutex::lock(&self.shared.buffer).clear();
        let _ = self.commands.send(Command::Seek(seconds));
    }

    pub fn stop(&self) {
        let _ = self.commands.send(Command::Stop);
        self.shared.epoch.fetch_add(1, Ordering::SeqCst);
        self.shared.played.store(0, Ordering::SeqCst);
        self.shared.playing.store(false, Ordering::SeqCst);
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        self.shared
            .volume
            .store(super::volume::gain(volume).to_bits(), Ordering::Relaxed);
        self.shared.muted.store(muted, Ordering::Relaxed);
    }

    /// Whether the output callback has already consumed samples, so audio is
    /// actually coming out instead of only queued.
    pub fn started(&self) -> bool {
        self.shared.played.load(Ordering::Relaxed) > 0
    }

    pub fn position(&self) -> f64 {
        let base = self.shared.base_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        if self.shared.seeking.load(Ordering::Relaxed) {
            return base;
        }

        let rate = self.shared.rate.load(Ordering::Relaxed) as f64;
        let played = self.shared.played.load(Ordering::Relaxed) as f64;
        base + played / rate
    }

    pub fn take_ended(&self) -> bool {
        self.shared.ended.swap(false, Ordering::SeqCst)
    }
}

pub(super) fn pick_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    let default = device
        .default_output_config()
        .map_err(|error| error.to_string())?;
    if default.sample_format() == cpal::SampleFormat::F32 {
        return Ok(default);
    }

    if let Ok(configs) = device.supported_output_configs() {
        for config in configs {
            if config.sample_format() == cpal::SampleFormat::F32 {
                return Ok(config.with_max_sample_rate());
            }
        }
    }

    Err("audio device does not accept f32 samples".to_owned())
}

pub(super) fn build_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    format: cpal::SampleFormat,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, String> {
    let stream_config: cpal::StreamConfig = (*config).into();
    let channels = config.channels() as usize;
    let error_shared = shared.clone();
    let on_error = move |error| {
        log::error!("audio stream error: {error}");
        error_shared.playing.store(false, Ordering::SeqCst);
    };

    if format != cpal::SampleFormat::F32 {
        return Err("audio device does not accept f32 samples".to_owned());
    }

    device
        .build_output_stream(
            stream_config,
            move |data: &mut [f32], _| fill(&shared, data, channels),
            on_error,
            None,
        )
        .map_err(|error| error.to_string())
}
