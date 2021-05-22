use crate::sampler::Sampler;
use std::time::Instant;

pub struct ClickInfo {
    pub live_sampler: Sampler,
    pub click_sampler: Sampler,
    pub click_time_delta: f64,
    pub mute_enabled: bool,
    pub num_clicks: usize,
}

impl ClickInfo {
    pub fn new() -> ClickInfo {
        ClickInfo {
            live_sampler: Sampler::new(10240, true),
            click_sampler: {
                let mut sampler = Sampler::new(102400, false);
                sampler.acquire_after(Instant::now());
                sampler
            },
            click_time_delta: 0.0,
            mute_enabled: true,
            num_clicks: 0,
        }
    }
}
