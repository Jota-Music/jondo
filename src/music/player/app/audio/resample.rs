use std::collections::VecDeque;

pub(super) struct Resampler {
    pub(super) step: f64,
    pub(super) position: f64,
    pub(super) previous: [f32; 2],
    pub(super) primed: bool,
    pub(super) dst_channels: usize,
}

impl Resampler {
    pub(super) fn new(source_rate: u32, destination_rate: u32, dst_channels: usize) -> Self {
        let step = if destination_rate == 0 {
            1.0
        } else {
            source_rate as f64 / destination_rate as f64
        };

        Self {
            step,
            position: 0.0,
            previous: [0.0; 2],
            primed: false,
            dst_channels,
        }
    }

    pub(super) fn reset(&mut self) {
        self.position = 0.0;
        self.previous = [0.0; 2];
        self.primed = false;
    }

    pub(super) fn push(&mut self, samples: &[f32], channels: usize, out: &mut VecDeque<f32>) {
        if channels == 0 {
            return;
        }

        for frame in samples.chunks_exact(channels) {
            let current = if channels == 1 {
                [frame[0], frame[0]]
            } else {
                [frame[0], frame[1]]
            };

            if !self.primed {
                self.previous = current;
                self.primed = true;
                self.position = 0.0;
            }

            while self.position < 1.0 {
                let left =
                    self.previous[0] + (current[0] - self.previous[0]) * self.position as f32;
                let right =
                    self.previous[1] + (current[1] - self.previous[1]) * self.position as f32;
                match self.dst_channels {
                    1 => out.push_back((left + right) * 0.5),
                    _ => {
                        out.push_back(left);
                        out.push_back(right);
                        for _ in 2..self.dst_channels {
                            out.push_back(0.0);
                        }
                    }
                }

                self.position += self.step;
            }

            self.position -= 1.0;
            self.previous = current;
        }
    }
}
