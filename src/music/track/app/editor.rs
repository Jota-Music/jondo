use std::time::{Duration, Instant};

use crate::music::track::domain::Mode;
use crate::shared::kernel::app::Load;
use crate::shared::music::domain::Song;
use crate::shared::youtube::Video;

pub(crate) const SWITCH: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Slide {
    from: f32,
    to: f32,
    at: Option<Instant>,
}

impl Slide {
    pub(crate) fn value(&self, now: Instant) -> f32 {
        let Some(at) = self.at else {
            return self.to;
        };

        let t = (now.duration_since(at).as_secs_f32() / SWITCH.as_secs_f32()).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t) * (1.0 - t);
        self.from + (self.to - self.from) * eased
    }

    pub(crate) fn start(&mut self, to: f32, now: Instant) {
        self.from = self.value(now);
        self.to = to;
        self.at = Some(now);
    }

    pub(crate) fn running(&self, now: Instant) -> bool {
        self.at.is_some_and(|at| now.duration_since(at) < SWITCH)
    }
}

#[derive(Default)]
pub struct Editor {
    pub open: bool,
    pub song: Option<Song>,
    pub mode: Mode,
    pub link: String,
    pub load: bool,
    pub search: String,
    pub query: String,
    pub results: Load<Vec<Video>>,
    pub resolving: bool,
    pub focus: bool,
    pub(crate) link_at: Option<Instant>,
    pub(crate) search_at: Option<Instant>,
    pub(crate) slide: Slide,
}

impl Editor {
    /// Whether a debounced link or search resolve is still owed a frame.
    pub fn waiting(&self) -> bool {
        self.link_at.is_some() || self.search_at.is_some()
    }

    /// Whether the assign/search transition still needs frames.
    pub fn animating(&self) -> bool {
        self.slide.running(Instant::now())
    }
}
