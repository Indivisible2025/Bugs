use crate::types::*;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, Semaphore};

/// Bugs 调度引擎——星空虫族的虫后。
/// 负责任务排队、优先级排序、子Agent 分配、故障转移。
pub struct Scheduler {
    /// 五级优先级桶
    buckets: [RwLock<VecDeque<QueuedTask>>; 5],
    /// 子Agent 池
    subagents: RwLock<HashMap<u64, SubAgentInfo>>,
    /// 并发控制
    limits: ConcurrencyLimits,
    /// 事件驱动容醒
    capacity: CapacityGate,
    /// 全局任务 ID 计数器
    next_id: std::sync::atomic::AtomicU64,
    next_agent_id: std::sync::atomic::AtomicU64,
}

/// 排队中的任务
#[derive(Debug, Clone)]
struct QueuedTask {
    task: Task,
    enqueued_at: Instant,
    effective_priority: u8,
}

/// 调度器并发限制
struct ConcurrencyLimits {
    global_max: u32,
    per_scene_max: u32,
}

/// 事件驱动容量门
struct CapacityGate {
    semaphore: Arc<Semaphore>,
    notify: Arc<Notify>,
}

impl CapacityGate {
    fn new(max: u32) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max as usize)),
            notify: Arc::new(Notify::new()),
        }
    }

    async fn wait_for_slot(&self) -> tokio::sync::SemaphorePermit<'_> {
        loop {
            match self.semaphore.try_acquire() {
                Ok(permit) => return permit,
                Err(_) => {
                    self.notify.notified().await;
                }
            }
        }
    }

    fn release(&self) {
        self.semaphore.add_permits(1);
        self.notify.notify_one();
    }
}

impl Scheduler {
    pub fn new(global_max: u32, per_scene_max: u32) -> Self {
        Self {
            buckets: [
                RwLock::new(VecDeque::new()),
                RwLock::new(VecDeque::new()),
                RwLock::new(VecDeque::new()),
                RwLock::new(VecDeque::new()),
                RwLock::new(VecDeque::new()),
            ],
            subagents: RwLock::new(HashMap::new()),
            limits: ConcurrencyLimits {
                global_max,
                per_scene_max,
            },
            capacity: CapacityGate::new(global_max),
            next_id: std::sync::atomic::AtomicU64::new(1),
            next_agent_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 入队一个任务——O(1)，按优先级入桶
    pub fn enqueue(&self, mut task: Task) {
        task.id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let priority = task.priority;
        task.state = TaskState::Queued;

        let bucket_idx = priority_to_bucket(priority);
        let mut bucket = self.buckets[bucket_idx].write();
        bucket.push_back(QueuedTask {
            task,
            enqueued_at: Instant::now(),
            effective_priority: priority as u8,
        });
    }

    /// 出队一个任务——O(k)，k=桶数(5)
    pub fn dequeue(&self) -> Option<Task> {
        for bucket in self.buckets.iter() {
            let mut queue = bucket.write();
            if let Some(qt) = queue.pop_front() {
                return Some(qt.task);
            }
        }
        None
    }

    /// 应用老化——等待超时的任务升桶
    pub fn apply_aging(&self, age_threshold: Duration) {
        for idx in (0..4).rev() {
            let mut bucket = self.buckets[idx].write();
            let mut promoted = Vec::new();
            bucket.retain(|qt| {
                if qt.enqueued_at.elapsed() > age_threshold {
                    promoted.push(qt.clone());
                    false
                } else {
                    true
                }
            });
            drop(bucket);
            for mut qt in promoted {
                qt.effective_priority = qt.effective_priority.saturating_sub(1);
                let target = priority_to_bucket_index(qt.effective_priority).min(4);
                self.buckets[target].write().push_back(qt);
            }
        }
    }

    /// 注册子Agent
    pub fn register_subagent(&self) -> u64 {
        let id = self
            .next_agent_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.subagents.write().insert(
            id,
            SubAgentInfo {
                id,
                state: SubAgentState::Idle,
                current_tasks: 0,
                max_concurrent: 8,
                started_at: Instant::now(),
                last_heartbeat: Instant::now(),
                total_completed: 0,
                total_failed: 0,
                avg_completion_time: Duration::ZERO,
                loaded_resources: vec![],
                recent_task_types: vec![],
            },
        );
        id
    }

    /// 分配任务给最空闲的子Agent
    pub fn assign(&self, task: &mut Task) -> Option<u64> {
        let agents = self.subagents.read();
        let best = agents
            .iter()
            .filter(|(_, a)| a.state == SubAgentState::Idle || a.state == SubAgentState::Busy)
            .min_by_key(|(_, a)| a.current_tasks);
        best.map(|(id, _)| {
            task.assigned_to = Some(*id);
            task.state = TaskState::Running;
            *id
        })
    }

    /// 获取调度器状态
    pub fn status(&self) -> SchedulerStatus {
        let agents = self.subagents.read();
        let total_tasks: usize = self.buckets.iter().map(|b| b.read().len()).sum();
        SchedulerStatus {
            queued_tasks: total_tasks,
            active_subagents: agents.len(),
            idle_subagents: agents
                .values()
                .filter(|a| a.state == SubAgentState::Idle)
                .count(),
        }
    }
}

pub struct SchedulerStatus {
    pub queued_tasks: usize,
    pub active_subagents: usize,
    pub idle_subagents: usize,
}

/// SubAgent 元数据——与 types.rs 中保持一致
#[derive(Debug, Clone)]
pub struct SubAgentInfo {
    pub id: u64,
    pub state: SubAgentState,
    pub current_tasks: u32,
    pub max_concurrent: u32,
    pub started_at: Instant,
    pub last_heartbeat: Instant,
    pub total_completed: u64,
    pub total_failed: u64,
    pub avg_completion_time: Duration,
    pub loaded_resources: Vec<String>,
    pub recent_task_types: Vec<TaskKind>,
}

/// 优先级 → 桶索引
fn priority_to_bucket(p: Priority) -> usize {
    match p {
        Priority::Emergency => 0,
        Priority::High => 1,
        Priority::Normal => 2,
        Priority::Low => 3,
        Priority::Background => 4,
    }
}

fn priority_to_bucket_index(p: u8) -> usize {
    match p {
        0 => 0,
        1..=3 => 1,
        4..=7 => 2,
        8..=15 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(priority: Priority) -> Task {
        Task {
            id: 0,
            scene_id: 1,
            priority,
            kind: TaskKind::Reasoning,
            state: TaskState::Pending,
            retry_count: 0,
            max_retries: 3,
            timeout: Duration::from_secs(30),
            assigned_to: None,
            created_at: 0,
            tags: vec![],
            description: String::new(),
        }
    }

    #[test]
    fn priority_buckets_order_preserved() {
        let s = Scheduler::new(100, 10);
        s.enqueue(make_task(Priority::Background));
        s.enqueue(make_task(Priority::Emergency));
        s.enqueue(make_task(Priority::Normal));

        let first = s.dequeue().unwrap();
        assert_eq!(first.priority, Priority::Emergency);

        let second = s.dequeue().unwrap();
        assert_eq!(second.priority, Priority::Normal);

        let third = s.dequeue().unwrap();
        assert_eq!(third.priority, Priority::Background);
    }

    #[test]
    fn subagent_registration() {
        let s = Scheduler::new(100, 10);
        let id = s.register_subagent();
        assert_eq!(id, 1);
        assert_eq!(s.status().active_subagents, 1);
    }
}
pub mod cron;
