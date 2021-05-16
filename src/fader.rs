pub struct Fader {
    value: f32,
    step_per_sample: f32,
}

impl Fader {
    pub fn new(value: f32) -> Fader {
        Fader {
            value,
            step_per_sample: 0.0,
        }
    }

    pub fn fade_in(&mut self, samples: usize) {
        self.step_per_sample = 1.0 / (samples as f32);
    }

    pub fn fade_out(&mut self, samples: usize) {
        self.step_per_sample = -1.0 / (samples as f32);
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        self.value = f32::clamp(self.value + self.step_per_sample, 0.0, 1.0);
        sample * self.value
    }
}
