use std::time::{Duration, Instant};

use crate::music::player::app::audio::Engine;
use crate::music::player::domain::queue::{Queue, Repeat};
use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::youtube::Stream;

/// How long the read position may stand still while the player believes it is
/// playing before the stream counts as dead. The decoder keeps a window of
/// audio in memory ahead of the output, so this is silence, not buffering.
const STALL: Duration = Duration::from_secs(4);

pub struct Player {
    pub queue: Queue,
    pub repeat: Repeat,
    pub progress: f64,
    pub duration: f64,
    pub playing: bool,
    pub loading: bool,
    /// The user paused while the stream was still on its way. The load keeps
    /// running; this only hides the spinner and keeps `start` from playing.
    pub paused: bool,
    pub stopped: bool,
    pub volume: f32,
    pub muted: bool,
    pub error: Option<String>,
    pub seek_preview: Option<f64>,
    pub loaded: bool,
    pub agent: Option<String>,
    pub(crate) retried: bool,
    still: Still,
    engine: Option<Engine>,
}

/// Watches the read position. A stream that stops moving is dead, not
/// buffering: the window ahead of the decoder is seconds of audio, so a frozen
/// position for this long means the network went away.
#[derive(Default)]
struct Still {
    since: Option<(f64, Instant)>,
}

impl Still {
    /// Returns true once the position has not moved for the whole window.
    fn froze(&mut self, position: f64, now: Instant) -> bool {
        match self.since {
            Some((seen, _)) if (position - seen).abs() > 0.05 => {
                self.since = Some((position, now));
                false
            }
            Some((_, since)) => now.duration_since(since) >= STALL,
            None => {
                self.since = Some((position, now));
                false
            }
        }
    }

    /// A new stream, a seek or a resumed one is not a stall, and the clock may
    /// not have ticked while the player was paused.
    fn reset(&mut self) {
        self.since = None;
    }
}

/// What one playback tick saw. A broken stream is not the end of the track:
/// the queue only moves on when the audio really ran out.
pub enum Tick {
    Idle,
    Ended,
    Failed(String),
}

impl Player {
    pub fn new(volume: f32, muted: bool) -> Self {
        let (engine, error) = match Engine::new() {
            Ok(engine) => (Some(engine), None),
            Err(error) => {
                log::warn!("audio disabled: {error}");
                (None, Some(error))
            }
        };

        if let Some(engine) = &engine {
            engine.set_volume(volume, muted);
        }

        Self {
            queue: Queue::default(),
            repeat: Repeat::Off,
            progress: 0.0,
            duration: 0.0,
            playing: false,
            loading: false,
            paused: false,
            stopped: false,
            volume,
            muted,
            error,
            seek_preview: None,
            loaded: false,
            agent: None,
            retried: false,
            still: Still::default(),
            engine,
        }
    }

    pub fn current(&self) -> Option<&Song> {
        self.queue.current()
    }

    pub fn start(&mut self, stream: Stream, start: f64) {
        let Some(engine) = &self.engine else {
            self.error = Some("no audio output".to_owned());
            self.loading = false;
            return;
        };

        self.duration = stream.duration;
        self.progress = start;
        self.agent = Some(stream.agent.clone());
        self.still.reset();
        // A pause pressed while the stream was resolving still wins: it loads
        // and buffers, but stays paused until the user asks for sound.
        if self.paused {
            engine.load_paused(&stream.url, &stream.agent, start);
            self.loaded = true;
            self.playing = false;
            self.loading = false;
        } else {
            engine.load(&stream.url, &stream.agent, start);
            self.loaded = true;
            self.playing = true;
            self.loading = true;
        }
        self.stopped = false;
        self.error = None;
        self.seek_preview = None;
        if let Some(song) = self.queue.current_mut() {
            song.duration = stream.duration.max(0.0) as u32;
        }
    }

    /// Clears the loader only once the engine is really playing, so the button
    /// never flashes the pause icon while the stream is still buffering.
    pub fn settle(&mut self) -> bool {
        if !self.loading || !self.engine.as_ref().is_some_and(|engine| engine.started()) {
            return false;
        }

        self.loading = false;
        true
    }

    // A room loads a track paused and starts it on the relay's shared instant.
    pub fn prepare(&mut self, stream: Stream, start: f64) {
        let Some(engine) = &self.engine else {
            self.error = Some("no audio output".to_owned());
            self.loading = false;
            return;
        };

        self.duration = stream.duration;
        self.progress = start;
        self.agent = Some(stream.agent.clone());
        self.still.reset();
        engine.load_paused(&stream.url, &stream.agent, start);
        self.loaded = true;
        self.playing = false;
        self.loading = false;
        self.paused = false;
        self.stopped = false;
        self.error = None;
        self.seek_preview = None;
        if let Some(song) = self.queue.current_mut() {
            song.duration = stream.duration.max(0.0) as u32;
        }
    }

    pub fn resume(&mut self) {
        if !self.loaded {
            return;
        }

        self.paused = false;
        self.playing = true;
        self.still.reset();
        if let Some(engine) = &self.engine {
            engine.play(true);
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
        self.paused = self.loading;
        if let Some(engine) = &self.engine {
            engine.play(false);
        }
    }

    /// A spinner only belongs to a load the user still wants. Pausing hides
    /// it while the decoder keeps filling the buffer in the background.
    pub fn busy(&self) -> bool {
        self.loading && !self.paused
    }

    pub fn toggle(&mut self) -> bool {
        if self.queue.current().is_none() {
            return false;
        }

        // Paused while loading. Dropping the intent is enough: `start` loads
        // the pending stream paused instead of playing it.
        if self.paused {
            self.paused = false;
            if !self.loaded {
                return true;
            }

            if self.duration > 0.0 && self.progress >= self.duration - 0.2 {
                return false;
            }

            self.playing = true;
            self.still.reset();
            if let Some(engine) = &self.engine {
                engine.play(true);
            }

            return true;
        }

        if self.playing {
            self.playing = false;
            self.paused = self.loading;
            if let Some(engine) = &self.engine {
                engine.play(false);
            }

            return true;
        }

        if self.loading {
            self.paused = true;
            return true;
        }

        if !self.loaded {
            return false;
        }

        if self.duration > 0.0 && self.progress < self.duration - 0.2 {
            self.playing = true;
            self.still.reset();
            if let Some(engine) = &self.engine {
                engine.play(true);
            }

            return true;
        }

        false
    }

    pub fn load_position(&mut self, position: f64) {
        self.progress = position;
    }

    pub fn seek(&mut self, seconds: f64) {
        let seconds = seconds.clamp(0.0, self.duration.max(0.0));
        self.progress = seconds;
        self.still.reset();
        if let Some(engine) = &self.engine {
            engine.seek(seconds);
        }
    }

    pub fn set_volume(&mut self, volume: f32, muted: bool) {
        self.volume = volume.clamp(0.0, 1.0);
        self.muted = muted;
        if let Some(engine) = &self.engine {
            engine.set_volume(self.volume, self.muted);
        }
    }

    pub fn tick(&mut self) -> Tick {
        let Some(engine) = &self.engine else {
            return Tick::Idle;
        };

        if let Some(error) = engine.take_failure() {
            return Tick::Failed(error);
        }

        if self.playing {
            let position = engine.position();
            if position > 0.0 {
                self.progress = position;
            }

            // Silence for longer than the buffer is a dead stream, not a slow
            // one: the queue gets a chance to re-resolve it from here.
            if !self.loading && self.still.froze(position, Instant::now()) {
                return Tick::Failed(t("The stream stopped").to_owned());
            }
        }

        if engine.take_ended() {
            self.playing = false;
            return Tick::Ended;
        }

        Tick::Idle
    }

    /// Each load gets a single refetch, so a dead URL is retried once and a
    /// queue of broken links cannot spin forever.
    pub fn retry(&mut self) -> bool {
        !std::mem::replace(&mut self.retried, true)
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.loading = false;
        self.loaded = false;
        self.paused = false;
        self.stopped = true;
        if let Some(engine) = &self.engine {
            engine.stop();
        }
    }

    pub fn fail(&mut self, error: String) {
        self.stop();
        self.error = Some(error);
    }
}

#[cfg(test)]
#[path = "player.test.rs"]
mod tests;
