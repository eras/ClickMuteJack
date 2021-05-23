use std::time::Instant;

#[derive(Clone)]
pub enum Mode {
    Capture,                      // captures
    AutoCapture,                  // captures after trigger; and then again after next trigger
    AutoHold,                     // captures after trigger; and then again after next trigger
    CaptureAfterTrigger(Instant), // goes to Capture after instant passes; don't sample in the meanwhile
    Hold,                         // don't sample
}

#[derive(Clone)]
pub struct Sampler {
    data: Vec<f32>,
    max_size: usize,
    write_index: usize,
    read_index: usize,
    mode: Mode,
}

impl Sampler {
    pub fn new(max_size: usize, live: bool) -> Sampler {
        Sampler {
            data: Vec::with_capacity(max_size),
            max_size,
            write_index: 0,
            read_index: 0,
            mode: if live { Mode::Capture } else { Mode::Hold },
        }
    }

    pub fn clear(&mut self) {
        self.data.truncate(0);
        self.write_index = 0;
        self.read_index = 0;
    }

    pub fn acquire_after(&mut self, after: Instant) {
        self.mode = Mode::CaptureAfterTrigger(after);
    }

    pub fn auto(&mut self) {
        self.mode = Mode::AutoCapture;
    }

    pub fn is_empty(&self) -> bool {
        self.read_index == self.write_index
    }

    pub fn is_in_hold(&self) -> bool {
        matches!(self.mode, Mode::Hold)
    }

    pub fn is_in_auto(&self) -> bool {
        matches!(self.mode, Mode::AutoCapture | Mode::AutoHold)
    }

    pub fn is_in_auto_hold(&self) -> bool {
        matches!(self.mode, Mode::AutoHold)
    }

    pub fn hold_or_auto_hold(&mut self) {
        self.mode = match self.mode {
            Mode::AutoCapture | Mode::AutoHold => Mode::AutoHold,
            _ => Mode::Hold,
        };
    }

    pub fn hold(&mut self) {
        self.mode = Mode::Hold;
    }

    pub fn live(&mut self) {
        self.mode = Mode::Capture;
    }

    pub fn is_full(&self) -> bool {
        (self.write_index + 1) % self.max_size == self.read_index
    }

    pub fn trigger(&mut self) {
        self.mode = match &self.mode {
            Mode::Capture => Mode::Capture,
            Mode::CaptureAfterTrigger(after) if Instant::now() >= *after => Mode::Capture,
            Mode::CaptureAfterTrigger(after) => Mode::CaptureAfterTrigger(*after),
            Mode::AutoHold | Mode::AutoCapture => {
                self.clear();
                Mode::AutoCapture
            }
            Mode::Hold => Mode::Hold,
        };
    }

    pub fn is_waiting(&self) -> bool {
        match &self.mode {
            Mode::Capture | Mode::Hold => false,
            Mode::AutoCapture => false,
            Mode::AutoHold => true,
            Mode::CaptureAfterTrigger(_) => true,
        }
    }

    pub fn sample(&mut self, sample: f32) {
        if matches!(self.mode, Mode::Capture | Mode::AutoCapture) {
            if self.data.len() == self.max_size {
                self.data[self.write_index] = sample;
                self.write_index = (self.write_index + 1) % self.max_size;
                if self.write_index == self.read_index {
                    self.read_index = (self.read_index + 1) % self.max_size;
                }
            } else {
                self.data.push(sample);
                self.write_index = (self.write_index + 1) % self.max_size;
            }
        }
    }

    pub fn get(&self) -> Vec<f32> {
        if self.write_index < self.read_index {
            let mut part1 = self.data[self.read_index..self.data.len()].to_vec();
            let part2 = &self.data[0..self.write_index];
            part1.extend_from_slice(&part2);
            part1
        } else {
            self.data[self.read_index..self.write_index].to_vec()
        }
    }

    pub fn rms(&self) -> f32 {
        let samples = self.get();
        let sqr_sum: f32 = samples.iter().map(|x| x.powf(2.0)).sum();
        let mean_sqr_sum = sqr_sum / samples.len() as f32;
        mean_sqr_sum.sqrt()
    }
}
