pub struct Save {
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
}

impl Save {
    pub fn new(num_channels: u16, filename: &str) -> Save {
        let spec = hound::WavSpec {
            channels: num_channels,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let writer = hound::WavWriter::create(filename, spec).unwrap();
        Save { writer }
    }

    pub fn process(&mut self, sample: f32) {
        // TODO: error handling.. but this is for debugging only, for now..
        self.writer.write_sample(sample).unwrap();
    }
}
