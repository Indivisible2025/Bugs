//! 配置热重载 — 监视文件变更

use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct ConfigWatcher {
    path: PathBuf,
    last_modified: Option<std::time::SystemTime>,
    last_check: Instant,
    interval: Duration,
}

impl ConfigWatcher {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            last_modified: None,
            last_check: Instant::now(),
            interval: Duration::from_secs(2),
        }
    }

    /// 检测配置是否已变更
    pub fn has_changed(&mut self) -> bool {
        if self.last_check.elapsed() < self.interval {
            return false;
        }
        self.last_check = Instant::now();
        let modified = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        if modified != self.last_modified {
            self.last_modified = modified;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::hotreload::*;
    #[test]
    fn watcher_creation() {
        let mut w = ConfigWatcher::new(std::path::PathBuf::from("/tmp/test.json"));
        assert!(!w.has_changed());
    }
}
