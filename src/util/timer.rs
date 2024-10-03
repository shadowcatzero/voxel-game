use std::time::{Duration, Instant};

pub struct Timer {
    start: Instant,
    pos: usize,
    times: Vec<Option<Instant>>,
    durs: Vec<Option<Duration>>,
}

impl Timer {
    pub fn new(len: usize) -> Self {
        Self {
            start: Instant::now(),
            pos: 0,
            durs: vec![None; len],
            times: vec![None; len],
        }
    }
    pub fn start(&mut self) {
        self.start = Instant::now();
    }
    pub fn stop(&mut self) {
        let duration = Instant::now() - self.start;
        self.durs[self.pos] = Some(duration);
        self.times[self.pos] = Some(self.start);
        self.pos = (self.pos + 1) % self.times.len();
    }
    pub fn add(&mut self, duration: Duration) {
        self.durs[self.pos] = Some(duration);
        self.times[self.pos] = Some(Instant::now());
        self.pos = (self.pos + 1) % self.times.len();
    }
    pub fn avg(&self) -> Duration {
        let filtered: Vec<_> = self.durs.iter().filter_map(|d| *d).collect();
        let len = filtered.len();
        if len != 0 {
            let total: Duration = filtered.into_iter().sum();
            total / len as u32
        } else {
            Duration::ZERO
        }
    }
    pub fn max(&self) -> Duration {
        self.durs
            .iter()
            .filter_map(|d| *d)
            .max()
            .unwrap_or(Duration::ZERO)
    }
    pub fn per_sec(&self) -> usize {
        let now = Instant::now();
        let mut count = 0;
        let len = self.times.len();
        while count < len {
            let i = (self.pos + len - count - 1) % len;
            let Some(t) = self.times[i] else { break };
            if now - t <= Duration::from_secs(1) {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}
