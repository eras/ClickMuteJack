pub struct CrossFader {
    value: f32,
    step_per_sample: f32,
}

impl CrossFader {
    pub fn new(value: f32) -> CrossFader {
        CrossFader {
            value,
            step_per_sample: 0.0,
        }
    }

    // fades to sample_a
    pub fn fade_in(&mut self, samples: usize) {
        self.step_per_sample = 1.0 / (samples as f32);
    }

    // fades to sample_b
    pub fn fade_out(&mut self, samples: usize) {
        self.step_per_sample = -1.0 / (samples as f32);
    }

    pub fn process(&mut self, sample_a: f32, sample_b: f32) -> f32 {
        self.value = f32::clamp(self.value + self.step_per_sample, 0.0, 1.0);
        sample_a * self.value + sample_b * (1.0 - self.value)
    }
}
