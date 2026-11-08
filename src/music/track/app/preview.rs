use std::sync::Arc;

use crate::music::player::app::Player;
use crate::music::player::app::audio::Engine;
use crate::shared::youtube::{Stream, Youtube};
use crate::shell::app::{Event, Jobs};

#[derive(Default)]
pub struct Preview {
    pub video: Option<String>,
    pub pending: bool,
    pub playing: bool,
    pub progress: f64,
    pub duration: f64,
    seeking: Option<f64>,
    token: u64,
    resume: bool,
    engine: Option<Engine>,
}

impl Preview {
    pub fn start(&mut self, player: &mut Player, youtube: &Arc<Youtube>, jobs: &Jobs, video: &str) {
        if self.video.as_deref() == Some(video) {
            if !self.pending && !self.playing {
                if let Some(engine) = &self.engine {
                    engine.play(true);
                }

                self.playing = true;
            }

            return;
        }

        self.kill();
        if !self.resume && player.playing {
            player.pause();
            self.resume = true;
        }

        self.token += 1;
        let token = self.token;
        self.video = Some(video.to_owned());
        self.pending = true;
        let youtube = youtube.clone();
        let video = video.to_owned();
        jobs.spawn(async move {
            let result = youtube.stream(&video).await;
            Event::YouTubePreview { token, result }
        });
    }

    pub fn load(
        &mut self,
        player: &mut Player,
        token: u64,
        result: Result<Stream, String>,
    ) -> Result<(), String> {
        if token != self.token {
            return Ok(());
        }

        let stream = result?;
        if self.engine.is_none() {
            self.engine = Some(Engine::new()?);
        }

        let engine = self.engine.as_ref().expect("preview engine");
        engine.set_volume(player.volume, player.muted);
        engine.load(&stream.url, &stream.agent, 0.0);
        self.duration = stream.duration;
        self.progress = 0.0;
        self.seeking = None;
        self.pending = false;
        self.playing = true;
        Ok(())
    }

    pub fn tick(&mut self, player: &mut Player) {
        let Some(engine) = &self.engine else {
            return;
        };

        engine.set_volume(player.volume, player.muted);
        if self.playing {
            let position = engine.position();
            if position > 0.0 {
                self.progress = position;
            }
        }

        if engine.take_ended() {
            self.stop(player);
        }
    }

    pub fn seek(&mut self, seconds: f64) {
        let Some(engine) = &self.engine else {
            return;
        };

        let seconds = seconds.max(0.0);
        engine.seek(seconds);
        self.progress = seconds;
        self.seeking = None;
    }

    pub fn toggle(&mut self) {
        let Some(engine) = &self.engine else {
            return;
        };

        self.playing = !self.playing;
        engine.play(self.playing);
    }

    pub fn stop(&mut self, player: &mut Player) {
        self.kill();
        if self.resume {
            self.resume = false;
            player.resume();
        }
    }

    /// Whether a preview paused the player and will resume it on stop.
    pub fn holding(&self) -> bool {
        self.resume
    }

    pub fn interrupt(&mut self) {
        self.kill();
        self.resume = false;
    }

    fn kill(&mut self) {
        self.token += 1;
        if let Some(engine) = &self.engine {
            engine.stop();
        }

        self.video = None;
        self.pending = false;
        self.playing = false;
        self.progress = 0.0;
        self.duration = 0.0;
        self.seeking = None;
    }
}
