use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use gpui::{RenderImage, Rgba};

/// Pixels each tier is decoded at. A row asks for the small one; the vinyl and
/// the page headers ask for the large one.
use super::*;
use super::{disk::*, spin::*};

pub(super) struct Entry {
    pub(super) image: Arc<RenderImage>,
    pub(super) color: Rgba,
    pub(super) last: u64,
}

type Slots = [Option<Entry>; 2];

/// The vinyl at one angle. GPUI cannot rotate a bitmap on the GPU, so the
/// frame is re-rendered for the angle being displayed.
pub(super) struct Spin {
    pub(super) url: String,
    pub(super) source: image::RgbaImage,
    pub(super) angle: f32,
    pub(super) frame: Arc<RenderImage>,
    pub(super) stale: Vec<Arc<RenderImage>>,
}

pub(super) struct Fetched {
    pub(super) url: String,
    pub(super) tier: usize,
    pub(super) image: Option<image::RgbaImage>,
    pub(super) color: Option<Rgba>,
}

pub struct Covers {
    jobs: Vec<Sender<(String, usize)>>,
    next: usize,
    receiver: Receiver<Fetched>,
    pending: HashMap<String, u8>,
    cache: HashMap<String, Slots>,
    /// The frame the cache is currently painting. An entry keeps the frame it
    /// was last asked for, so eviction can leave the covers on screen alone.
    frame: u64,
    failed: HashSet<String>,
    spin: Option<Spin>,
    dropped: Vec<Arc<RenderImage>>,
}

impl Default for Covers {
    fn default() -> Self {
        Self::new()
    }
}

impl Covers {
    pub fn new() -> Self {
        if let Some(dir) = cache_dir() {
            crate::shared::storage::library::trim(&dir, DISK_LIMIT);
        }

        let (result_sender, receiver) = channel();
        let workers = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(2)
            .clamp(2, 3);
        let mut jobs = Vec::with_capacity(workers);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        for index in 0..workers {
            let (job_sender, job_receiver) = channel::<(String, usize)>();
            jobs.push(job_sender);
            let sender = result_sender.clone();
            let client = client.clone();
            std::thread::Builder::new()
                .name(format!("jondo-cover-{index}"))
                .spawn(move || {
                    while let Ok((url, tier)) = job_receiver.recv() {
                        let _ = sender.send(load(&client, &url, tier));
                    }
                })
                .ok();
        }

        Self {
            jobs,
            next: 0,
            receiver,
            pending: HashMap::new(),
            cache: HashMap::new(),
            frame: 0,
            failed: HashSet::new(),
            spin: None,
            dropped: Vec::new(),
        }
    }

    /// The image to paint in a slot about `needed` pixels wide on a 2x display.
    /// Anything up to the small tier's size shares one decode, so a list of
    /// mosaic cells never holds the full size of every tile.
    pub fn image(&mut self, url: &str, needed: f32) -> Option<Arc<RenderImage>> {
        let tier = tier(needed);
        if !self.ready(url, tier) {
            self.ask(url, tier);
            return None;
        }

        let frame = self.frame;
        let slot = self
            .cache
            .get_mut(url)
            .and_then(|slots| slots[tier].as_mut())?;
        slot.last = frame;
        Some(slot.image.clone())
    }

    /// Asks for the full size, for the callers that only need the colour.
    pub fn request(&mut self, url: &str) {
        self.ask(url, FULL);
    }

    pub(super) fn ready(&self, url: &str, tier: usize) -> bool {
        self.cache
            .get(url)
            .is_some_and(|slots| slots[tier].is_some())
    }

    pub(super) fn ask(&mut self, url: &str, tier: usize) {
        if url.is_empty() || self.ready(url, tier) || self.failed.contains(url) {
            return;
        }

        let mark = 1u8 << tier;
        if self.pending.get(url).is_some_and(|bits| bits & mark != 0) {
            return;
        }

        *self.pending.entry(url.to_owned()).or_default() |= mark;
        let index = self.next % self.jobs.len();
        self.next = self.next.wrapping_add(1);
        if self.jobs[index].send((url.to_owned(), tier)).is_err() {
            self.settle(url, tier);
        }
    }

    pub(super) fn settle(&mut self, url: &str, tier: usize) {
        let mark = 1u8 << tier;
        if let Some(bits) = self.pending.get_mut(url) {
            *bits &= !mark;
            if *bits == 0 {
                self.pending.remove(url);
            }
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut loaded = false;
        while let Ok(fetched) = self.receiver.try_recv() {
            self.settle(&fetched.url, fetched.tier);
            let Some(bytes) = fetched.image else {
                if self.failed.len() >= FAILED_LIMIT {
                    self.failed.clear();
                }

                self.failed.insert(fetched.url);
                continue;
            };

            let color = fetched
                .color
                .unwrap_or(crate::shared::ui::views::design::theme::ACCENT);
            let image = Arc::new(RenderImage::new(vec![image::Frame::new(bytes)]));
            let frame = self.frame;
            let entry = Entry {
                image,
                color,
                last: frame,
            };

            self.cache.entry(fetched.url).or_default()[fetched.tier] = Some(entry);
            loaded = true;
        }

        loaded
    }

    pub fn pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Starts a frame. Images asked for after this point are the ones the frame
    /// paints, and the ones eviction must not touch.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Drops the covers no frame has painted since the previous one. It runs at
    /// the start of a frame, before the tree asks for images: a cover the last
    /// frame painted stays, and one that scrolled away is released. Without the
    /// frame check a burst of loads could evict the images on screen, which
    /// sends the row back to its shimmer and asks for the same cover again.
    pub fn prune(&mut self) {
        for tier in [THUMB, FULL] {
            while self.tier(tier) > LIMITS[tier] {
                let oldest = self
                    .cache
                    .iter()
                    .filter_map(|(url, slots)| {
                        let entry = slots[tier].as_ref()?;
                        evictable(entry.last, self.frame).then(|| (url.clone(), entry.last))
                    })
                    .min_by_key(|(_, last)| *last)
                    .map(|(url, _)| url);
                let Some(url) = oldest else {
                    break;
                };

                let Some(slots) = self.cache.get_mut(&url) else {
                    break;
                };

                if let Some(entry) = slots[tier].take() {
                    self.dropped.push(entry.image);
                }

                if slots.iter().all(Option::is_none) {
                    self.cache.remove(&url);
                }
            }
        }
    }

    pub(super) fn tier(&self, tier: usize) -> usize {
        self.cache
            .values()
            .filter(|slots| slots[tier].is_some())
            .count()
    }

    pub fn color(&self, url: &str) -> Option<Rgba> {
        let slots = self.cache.get(url)?;
        slots[FULL]
            .as_ref()
            .or(slots[THUMB].as_ref())
            .map(|entry| entry.color)
    }

    /// The disc at `angle`, so the vinyl can turn at any speed without
    /// stepping between pre-baked rotations.
    pub fn spin(&mut self, url: &str, angle: f32) -> Option<Arc<RenderImage>> {
        if self.spin.as_ref().is_none_or(|cached| cached.url != url) {
            let image = self.image(url, SPIN_SIZE as f32)?;
            let size = image.size(0);
            let width = size.width.0.max(1) as u32;
            let height = size.height.0.max(1) as u32;
            let bytes = image.as_bytes(0)?;
            let source = image::RgbaImage::from_raw(width, height, bytes.to_vec())?;
            let source = image::imageops::resize(
                &source,
                SPIN_SIZE,
                SPIN_SIZE,
                image::imageops::FilterType::Triangle,
            );
            let frame = Arc::new(spin_frame(&source, angle));
            let mut stale = Vec::new();
            if let Some(old) = self.spin.take() {
                stale.extend(old.stale);
                stale.push(old.frame);
            }

            self.spin = Some(Spin {
                url: url.to_owned(),
                source,
                angle,
                frame,
                stale,
            });
        } else if let Some(spin) = &mut self.spin
            && (spin.angle - angle).abs() >= SPIN_STEP
        {
            let frame = Arc::new(spin_frame(&spin.source, angle));
            let old = std::mem::replace(&mut spin.frame, frame);
            spin.stale.push(old);
            spin.angle = angle;
        }

        self.spin.as_ref().map(|spin| spin.frame.clone())
    }

    /// Frames whose atlas tile can go. The two newest stay so a frame the GPU
    /// may still be drawing never has its tile reused underneath it.
    pub fn stale(&mut self) -> Vec<Arc<RenderImage>> {
        let Some(spin) = &mut self.spin else {
            return Vec::new();
        };

        let keep = spin.stale.len().saturating_sub(2);
        spin.stale.drain(..keep).collect()
    }

    /// Evicted covers, whose atlas tiles the window still holds until it is
    /// told to drop them.
    pub fn dropped(&mut self) -> Vec<Arc<RenderImage>> {
        std::mem::take(&mut self.dropped)
    }

    /// Drops every decoded cover. The next render asks for them again, from the
    /// disk cache, so this only reclaims the decoded pixels held in memory.
    pub fn clear(&mut self) {
        for slots in self.cache.drain().map(|(_, slots)| slots) {
            for entry in slots.into_iter().flatten() {
                self.dropped.push(entry.image);
            }
        }
    }

    pub fn failed(&self, url: &str) -> bool {
        self.failed.contains(url)
    }
}

pub(super) fn tier(needed: f32) -> usize {
    if needed <= SIZES[THUMB] as f32 {
        THUMB
    } else {
        FULL
    }
}
