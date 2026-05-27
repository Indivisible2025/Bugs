use crate::types::*;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Bugs 信任引擎——速率驱动的记忆可信度评分系统。
pub struct TrustEngine {
    pub config: TrustConfig,
    /// 来源可信度追踪
    sources: RwLock<HashMap<u64, TrustSource>>,
}

impl TrustEngine {
    pub fn new(config: TrustConfig) -> Self {
        Self {
            config,
            sources: RwLock::new(HashMap::new()),
        }
    }

    /// 初始化一条新记忆的速率
    pub fn initialize_memory(&self, memory: &mut Memory) {
        memory.enhancement_rate = self.config.initial_enhancement;
        memory.decay_rate = self.config.initial_decay;
        memory.strength_cached = 0.0;
    }

    /// 交叉验证——被多个独立来源验证时，增强速率增加
    pub fn cross_validate(&self, memory: &mut Memory, source_id: u64) {
        let credibility = self.get_credibility(source_id);
        memory.enhancement_rate += self.config.cross_validate_k * credibility;
        memory.source_count += 1;
    }

    /// 用户确认——最强验证来源
    pub fn user_confirm(&self, memory: &mut Memory, source_id: u64) {
        memory.enhancement_rate += self.config.user_confirm_delta;
        memory.decay_rate = (memory.decay_rate - 0.1).max(0.01);
        memory.last_validated = now_timestamp();
        memory.validation_count += 1;

        // 提升来源可信度
        let mut sources = self.sources.write();
        if let Some(s) = sources.get_mut(&source_id) {
            s.credibility = (s.credibility * 1.05).min(1.0);
            s.correct_validations += 1;
        }
    }

    /// 证伪——加速衰减
    pub fn disprove(&self, memory: &mut Memory) {
        memory.decay_rate += self.config.disprove_penalty;
        memory.enhancement_rate = 0.0;
    }

    /// 计算当前强度
    pub fn calculate_strength(&self, memory: &mut Memory) -> f64 {
        let days_since_update = 0.0; // 简化为即时计算
        memory.strength_cached += (memory.enhancement_rate - memory.decay_rate) * days_since_update;
        memory.strength_cached
    }

    /// 时间衰减
    pub fn apply_decay(&self, memory: &mut Memory) {
        memory.strength_cached *= (1.0 - memory.decay_rate).max(0.0);
    }

    /// 获取来源可信度
    fn get_credibility(&self, source_id: u64) -> f64 {
        self.sources
            .read()
            .get(&source_id)
            .map(|s| s.credibility)
            .unwrap_or(1.0)
    }

    /// 注册新来源
    pub fn register_source(&self, source_id: u64) {
        self.sources.write().entry(source_id).or_insert(TrustSource {
            agent_id: source_id,
            credibility: 1.0,
            last_validated: now_timestamp(),
            wrong_validations: 0,
            correct_validations: 0,
        });
    }
}

impl Default for TrustEngine {
    fn default() -> Self {
        Self::new(TrustConfig::default())
    }
}

fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            initial_enhancement: 0.0,
            initial_decay: 0.1,
            cross_validate_k: 0.1,
            user_confirm_delta: 0.5,
            disprove_penalty: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_memory() -> Memory {
        Memory {
            agent_id: 1, scene_id: 1, scope: MemoryScope::All,
            owners: vec![OwnerId::All],
            category: MemoryCategory::Knowledge,
            subagent_type: SubAgentType::General,
            enhancement_rate: 0.0, decay_rate: 0.1,
            strength_cached: 0.0,
            last_updated: 0, last_validated: 0,
            validation_count: 0, source_count: 0, seq: 0,
        }
    }

    #[test]
    fn new_memory_starts_at_zero() {
        let engine = TrustEngine::default();
        let mut mem = make_test_memory();
        engine.initialize_memory(&mut mem);
        assert_eq!(mem.enhancement_rate, 0.0);
        assert_eq!(mem.decay_rate, 0.1);
    }

    #[test]
    fn cross_validation_increases_rate() {
        let engine = TrustEngine::default();
        engine.register_source(1);
        let mut mem = make_test_memory();
        engine.cross_validate(&mut mem, 1);
        assert!(mem.enhancement_rate > 0.0);
        assert_eq!(mem.source_count, 1);
    }

    #[test]
    fn user_confirm_strongest() {
        let engine = TrustEngine::default();
        let mut mem = make_test_memory();
        engine.user_confirm(&mut mem, 1);
        assert_eq!(mem.enhancement_rate, 0.5);
        assert!(mem.decay_rate < 0.1);
    }
}
