use std::time::{Duration, Instant};

pub fn measure<F, T>(f: F) -> (Duration, T)
where
    F: FnOnce() -> T,
{
    let t1 = Instant::now();
    let value = f();
    let t2 = Instant::now();
    (t2 - t1, value)
}

pub struct Repeated {
    num_calls: u32,
    total_time: Duration,
    prev_time: Duration,
}

impl Repeated {
    pub fn new() -> Self {
        Repeated {
            num_calls: 0,
            total_time: std::default::Default::default(),
            prev_time: std::default::Default::default(),
        }
    }

    pub fn measure<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let (time, retval) = measure(f);
        self.num_calls += 1;
        self.total_time += time;
        self.prev_time = time;
        retval
    }

    pub fn average(&self) -> Duration {
        self.total_time / self.num_calls
    }

    pub fn prev_time(&self) -> Duration {
        self.prev_time
    }
}
