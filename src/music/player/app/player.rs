use crate::music::player::app::audio::Engine;
use crate::music::player::domain::queue::{Queue, Repeat};
use crate::shared::music::domain::Song;
use crate::shared::youtube::Stream;

pub struct Player {
    pub queue: Queue,
    pub repeat: Repeat,
    pub progress: f64,
    pub duration: f64,
    pub playing: bool,
    pub loading: bool,
    pub volume: f32,
    pub muted: bool,
    pub error: Option<String>,
    pub seek_preview: Option<f64>,
    pub loaded: bool,
    pub agent: Option<String>,
    engine: Option<Engine>,
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
            volume,
            muted,
            error,
            seek_preview: None,
            loaded: false,
            agent: None,
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
        engine.load(&stream.url, &stream.agent, start);
        self.loaded = true;
        self.playing = true;
        self.loading = true;
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
        engine.load_paused(&stream.url, &stream.agent, start);
        self.loaded = true;
        self.playing = false;
        self.loading = false;
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

        self.playing = true;
        if let Some(engine) = &self.engine {
            engine.play(true);
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
        if let Some(engine) = &self.engine {
            engine.play(false);
        }
    }

    pub fn toggle(&mut self) -> bool {
        if self.queue.current().is_none() || !self.loaded {
            return false;
        }

        if self.playing {
            self.playing = false;
            if let Some(engine) = &self.engine {
                engine.play(false);
            }

            return true;
        }

        if self.duration > 0.0 && self.progress < self.duration - 0.2 {
            self.playing = true;
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

    pub fn tick(&mut self) -> bool {
        if let Some(engine) = &self.engine {
            if self.playing {
                let position = engine.position();
                if position > 0.0 {
                    self.progress = position;
                }
            }

            if engine.take_ended() {
                self.playing = false;
                return true;
            }
        }

        false
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.loading = false;
        self.loaded = false;
        if let Some(engine) = &self.engine {
            engine.stop();
        }
    }

    pub fn fail(&mut self, error: String) {
        self.loading = false;
        self.playing = false;
        self.loaded = false;
        self.error = Some(error);
    }
}

#[cfg(test)]
#[path = "player.test.rs"]
mod tests;
