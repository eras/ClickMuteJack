use crate::sampler::Sampler;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Clip {
    pub sample_a: Sampler,
    pub sample_b: Sampler,
    pub rms: f32,
}

type ClipId = usize;

pub struct BackgroundSampler {
    current_clip: Option<Clip>,
    clip_length: usize,
    num_clips: usize,
    clip_id_gen: ClipId,
    clips: BTreeMap<ClipId, Clip>,
    rng: StdRng,
}

pub type StereoSample = (f32, f32);

// BackgroundSampler periodically (or maybe randomly) samples short samples, keeping at most num_clips latest
// ones. Then it orders the samples by their volume and the user may pick n quietest samples from them to use as
// background noise.  If sampling a sample is interrupted by a pause, then that sample is discarded.
impl BackgroundSampler {
    pub fn new(num_clips: usize, clip_length: usize) -> BackgroundSampler {
        let mut bg_sampler = BackgroundSampler {
            current_clip: None,
            clip_length,
            num_clips,
            clip_id_gen: 0,
            clips: BTreeMap::new(),
            rng: StdRng::from_entropy(),
        };
        bg_sampler.resume();
        bg_sampler
    }

    fn new_clip_id(&mut self) -> usize {
        let id = self.clip_id_gen;
        self.clip_id_gen += 1;
        id
    }

    pub fn sample(&mut self, sample: StereoSample) {
        let full = match self.current_clip {
            None => false,
            Some(ref mut clip) => {
                clip.sample_a.sample(sample.0);
                clip.sample_b.sample(sample.1);
                clip.sample_a.is_full()
            }
        };
        if full {
            let id = self.new_clip_id();
            let mut clip = self.current_clip.take().unwrap();
            clip.rms = clip.sample_a.rms().max(clip.sample_b.rms());
            self.clips.insert(id, clip);
            if self.clips.len() > self.num_clips {
                // we could do: self.clips.pop_first();
                // but let's avoid unstable features for now
                if let Some((&key, _)) = self.clips.iter().next() {
                    self.clips.remove(&key);
                }
            }
            self.resume();
        }
    }

    // pick a random clip from the n least-rms clips
    pub fn choose_clip(&mut self, limit: usize) -> Option<Clip> {
        if self.clips.is_empty() {
            None
        } else {
            let mut clips: Vec<&Clip> = self.clips.iter().map(|(_, v)| v).collect();
            clips.sort_unstable_by(|a, b| {
                if a.rms < b.rms {
                    std::cmp::Ordering::Less
                } else if a.rms > b.rms {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            Some(clips[self.rng.gen_range(0..limit.min(clips.len()))].clone())
        }
    }

    pub fn pause(&mut self) {
        self.current_clip = None;
    }

    pub fn resume(&mut self) {
        if self.current_clip.is_none() {
            self.current_clip = Some(Clip {
                sample_a: Sampler::new(self.clip_length, true),
                sample_b: Sampler::new(self.clip_length, true),
                rms: 0.0,
            });
        }
    }
}
