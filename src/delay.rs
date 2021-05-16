pub struct Delay {
    buffer: Vec<f32>,
    index: usize,
    filled: bool,
}

impl Delay {
    pub fn new(delay: usize) -> Delay {
        assert!(delay > 0);
        Delay {
            buffer: vec![0.0; delay],
            index: 0,
            filled: false,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        if self.filled {
            let delay_sample = self.buffer[self.index];
            self.buffer[self.index] = sample;
            self.index += 1;
            if self.index == self.buffer.len() {
                self.index = 0;
            }
            delay_sample
        } else {
            self.buffer[self.index] = sample;
            self.index += 1;
            if self.index == self.buffer.len() {
                self.index = 0;
                self.filled = true;
            }
            0.0
        }
    }
}
