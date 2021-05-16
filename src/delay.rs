pub struct Delay {
    buffer: Vec<f32>,
    index: usize,
    filled: bool,
}

pub fn new(delay: usize) -> Delay {
    assert!(delay > 0);
    Delay {
        buffer: vec![0.0; delay],
        index: 0,
        filled: false,
    }
}

pub fn process(delay: &mut Delay, sample: f32) -> f32 {
    if delay.filled {
        let delay_sample = delay.buffer[delay.index];
        delay.buffer[delay.index] = sample;
        delay.index += 1;
        if delay.index == delay.buffer.len() {
            delay.index = 0;
        }
        delay_sample
    } else {
        delay.buffer[delay.index] = sample;
        delay.index += 1;
        if delay.index == delay.buffer.len() {
            delay.index = 0;
            delay.filled = true;
        }
        0.0
    }
}
