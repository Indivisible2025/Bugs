use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

// ── Task ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub scene_id: u64,
    pub priority: Priority,
    pub kind: TaskKind,
    pub state: TaskState,
    pub retry_count: u8,
    pub max_retries: u8,
    pub timeout: Duration,
    pub assigned_to: Option<u64>,
    pub created_at: u64,
    pub tags: Vec<String>,
    pub description: String,
}

impl Task {
    pub fn description(&self) -> &str { &self.description }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    Reasoning,
    ToolCall,
    FileOp,
    Browser,
    Knowledge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Emergency = 0,
    High = 1,
    Normal = 4,
    Low = 8,
    Background = 16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
    DeadLetter,
    Canceled,
}

// ── Memory ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub agent_id: u64,
    pub scene_id: u64,
    pub scope: MemoryScope,
    pub owners: Vec<OwnerId>,
    pub category: MemoryCategory,
    pub subagent_type: SubAgentType,
    pub enhancement_rate: f64,
    pub decay_rate: f64,
    pub strength_cached: f64,
    pub last_updated: u64,
    pub last_validated: u64,
    pub validation_count: u32,
    pub source_count: u32,
    pub seq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryScope {
    Single(u64),
    All,
    Partial(Vec<u64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCategory {
    Knowledge,
    Habit,
    Experience,
    Rule,
    Relation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubAgentType {
    Browser,
    FileOp,
    Reasoning,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentState {
    Idle,
    Busy,
    Overloaded,
    Unhealthy,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OwnerId {
    Agent(u64),
    AgentType(SubAgentType),
    All,
}

// ── Trust ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSource {
    pub agent_id: u64,
    pub credibility: f64,
    pub last_validated: u64,
    pub wrong_validations: u32,
    pub correct_validations: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnhancementRate(pub f64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DecayRate(pub f64);

// ── Scene ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: u64,
    pub name: String,
    pub paths: Vec<PathBuf>,
    pub config: SceneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneConfig {
    pub tools: ToolConfig,
    pub trust: TrustConfig,
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub protection_paths: Vec<String>,
    pub granted: std::collections::HashMap<String, GrantLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantLevel {
    AllowPermanent,
    AllowOnce,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustConfig {
    pub initial_enhancement: f64,
    pub initial_decay: f64,
    pub cross_validate_k: f64,
    pub user_confirm_delta: f64,
    pub disprove_penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryConfig {
    pub global_max_mb: u32,
    pub scene_summary_max_kb: u32,
    pub scene_full_max_mb: u32,
}

// ── Browser ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserManager {
    pub headless_pool: ContextPool,
    pub headful_pool: ContextPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPool {
    pub contexts: Vec<BrowserContext>,
    pub max_size: usize,
    pub idle_timeout: Duration,
    pub max_tabs_per_context: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserContext {
    pub id: u64,
    pub tabs: Vec<Tab>,
    pub state: ContextState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: u64,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextState {
    Active,
    Dormant,
    Dead,
}

// ── Config ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugsConfig {
    pub scheduler: SchedulerConfig,
    pub trust: TrustConfig,
    pub tools: ToolConfig,
    pub memory: MemoryConfig,
    pub network: NetworkConfig,
    pub scenes: SceneSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub global_max_parallelism: u32,
    pub per_scene_max_parallelism: u32,
    pub default_timeout_secs: u32,
    pub max_retries: u8,
    pub retry_base_delay_ms: u32,
    pub heartbeat_interval_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub local: LocalNet,
    pub remote: RemoteNet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalNet {
    pub bind: String,
    pub port: u16,
    pub auto_trust: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNet {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
    pub domain: String,
    pub auth_mode: String,
    pub tls_cert: String,
    pub tls_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSettings {
    pub auto_detect: bool,
    pub debounce_ms: u32,
    pub default_scene: String,
}

impl Default for BugsConfig {
    fn default() -> Self {
        Self {
            scheduler: SchedulerConfig {
                global_max_parallelism: 100_000,
                per_scene_max_parallelism: 10_000,
                default_timeout_secs: 30,
                max_retries: 3,
                retry_base_delay_ms: 500,
                heartbeat_interval_secs: 5,
            },
            trust: TrustConfig {
                initial_enhancement: 0.0,
                initial_decay: 0.1,
                cross_validate_k: 0.1,
                user_confirm_delta: 0.5,
                disprove_penalty: 2.0,
            },
            tools: ToolConfig {
                allow: vec![],
                deny: vec![],
                protection_paths: vec![],
                granted: std::collections::HashMap::new(),
            },
            memory: MemoryConfig {
                global_max_mb: 1,
                scene_summary_max_kb: 100,
                scene_full_max_mb: 50,
            },
            network: NetworkConfig {
                local: LocalNet { bind: "127.0.0.1".into(), port: 8742, auto_trust: true },
                remote: RemoteNet {
                    enabled: false, bind: "0.0.0.0".into(), port: 8742,
                    domain: String::new(), auth_mode: "token".into(),
                    tls_cert: String::new(), tls_key: String::new(),
                },
            },
            scenes: SceneSettings {
                auto_detect: true,
                debounce_ms: 200,
                default_scene: "general".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_usable() {
        let cfg = BugsConfig::default();
        assert_eq!(cfg.scheduler.global_max_parallelism, 100_000);
        assert_eq!(cfg.network.local.port, 8742);
        assert_eq!(cfg.scenes.default_scene, "general");
    }
}
