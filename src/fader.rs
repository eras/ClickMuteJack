pub struct Fader {
    value: f32,
    step_per_sample: f32,
}

pub fn new(value: f32) -> Fader {
    Fader {
        value,
        step_per_sample: 0.0,
    }
}

pub fn fade_in(fader: &mut Fader, samples: usize) {
    fader.step_per_sample = 1.0 / (samples as f32);
}

pub fn fade_out(fader: &mut Fader, samples: usize) {
    fader.step_per_sample = -1.0 / (samples as f32);
}

pub fn process(fader: &mut Fader, sample: f32) -> f32 {
    fader.value = f32::clamp(fader.value + fader.step_per_sample, 0.0, 1.0);
    sample * fader.value
}
