use std::time::{Duration, Instant};

pub struct CronJob {
    pub name: &'static str,
    pub interval: Duration,
    pub last_run: parking_lot::Mutex<Instant>,
    pub action: fn(),
}

pub struct CronScheduler {
    pub jobs: Vec<CronJob>,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self { jobs: vec![] }
    }

    pub fn add(&mut self, name: &'static str, interval_secs: u64, action: fn()) {
        self.jobs.push(CronJob {
            name,
            interval: Duration::from_secs(interval_secs),
            last_run: parking_lot::Mutex::new(Instant::now()),
            action,
        });
    }

    /// 检查并触发到期的定时任务
    pub fn tick(&self) -> Vec<&str> {
        let mut triggered = Vec::new();
        for job in &self.jobs {
            if job.last_run.lock().elapsed() >= job.interval {
                (job.action)();
                *job.last_run.lock() = Instant::now();
                triggered.push(job.name);
            }
        }
        triggered
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}
