use crate::background_sampler;
use crate::background_sampler::BackgroundSampler;

struct PlayClip {
    sample_a: Vec<f32>,
    sample_b: Vec<f32>,
    playhead: usize,
}

pub struct Looper {
    current_clip: Option<PlayClip>,
}

impl Looper {
    pub fn new() -> Looper {
        Looper { current_clip: None }
    }

    pub fn produce(
        &mut self,
        background_sampler: &mut BackgroundSampler,
    ) -> background_sampler::StereoSample {
        if self.current_clip.is_none() {
            if let Some(clip) = background_sampler.choose_clip(10) {
                self.current_clip = Some(PlayClip {
                    sample_a: clip.sample_a.get(),
                    sample_b: clip.sample_b.get(),
                    playhead: 0,
                })
            }
        }
        if let Some(ref mut clip) = self.current_clip {
            if clip.playhead >= clip.sample_a.len() {
                self.current_clip = None;
                (0.0, 0.0)
            } else {
                let sample = (clip.sample_a[clip.playhead], clip.sample_b[clip.playhead]);
                clip.playhead += 1;
                sample
            }
        } else {
            (0.0, 0.0)
        }
    }
}
