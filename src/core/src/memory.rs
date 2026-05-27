use crate::persistence::Store;
use crate::types::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;

/// 记忆网格 — 所有记忆持久化，检索时按场景过滤
pub struct MemoryMesh {
    pub(crate) global: RwLock<Vec<Memory>>,
    pub(crate) summaries: RwLock<HashMap<u64, Vec<MemorySummary>>>,
    pub(crate) full_scenes: RwLock<HashMap<u64, Vec<Memory>>>,
    config: MemoryConfig,
    store: RwLock<Option<Store>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemorySummary {
    pub agent_id: u64,
    pub category: MemoryCategory,
    pub strength: f64,
    pub content_hash: u64,
    pub last_accessed: u64,
}

pub struct MemoryStats {
    pub global_count: usize,
    pub scene_count: usize,
    pub total_entries: usize,
}

impl MemoryMesh {
    pub fn new(config: MemoryConfig) -> Self {
        Self { global: RwLock::new(Vec::new()), summaries: RwLock::new(HashMap::new()),
            full_scenes: RwLock::new(HashMap::new()), config, store: RwLock::new(None) }
    }

    /// 初始化持久化存储——加载全局记忆 + 所有场景概要
    pub fn init_storage(&self, home: &PathBuf) -> Result<(), String> {
        let store = Store::open(home)?;
        if let Ok(mems) = store.load_global() { *self.global.write() = mems; }
        // 加载所有已有场景的概要
        if let Ok(entries) = std::fs::read_dir(home.join("data/scenes")) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(id_str) = name.strip_suffix(".redb") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            if let Ok(summaries) = store.load_summaries(id) {
                                self.summaries.write().insert(id, summaries);
                            }
                        }
                    }
                }
            }
        }
        *self.store.write() = Some(store);
        Ok(())
    }

    /// 从完整场景记忆生成并持久化概要
    pub fn update_summary(&self, scene_id: u64) -> Result<(), String> {
        let summaries: Vec<MemorySummary> = self.full_scenes.read().get(&scene_id)
            .map(|mems| mems.iter().map(|m| MemorySummary {
                agent_id: m.agent_id, category: m.category,
                strength: m.strength_cached, content_hash: 0,
                last_accessed: m.last_validated,
            }).take(100).collect())
            .unwrap_or_default();
        self.summaries.write().insert(scene_id, summaries.clone());
        if let Some(ref s) = *self.store.read() { s.save_summaries(scene_id, &summaries)?; }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), String> {
        if let Some(ref s) = *self.store.read() { s.save_global(&self.global.read())?; }
        Ok(())
    }

    pub fn flush_scene(&self, scene_id: u64) -> Result<(), String> {
        if let Some(ref s) = *self.store.read() {
            if let Some(m) = self.full_scenes.read().get(&scene_id) { s.save_scene(scene_id, m)?; }
        }
        self.update_summary(scene_id)
    }

    /// 存储记忆（内存 + 延迟持久化）
    pub fn store(&self, m: Memory) {
        match &m.scope {
            MemoryScope::All => { self.global.write().push(m); }
            MemoryScope::Single(id) => { self.full_scenes.write().entry(*id).or_default().push(m); }
            _ => {}
        }
    }

    /// 检索记忆（当前场景可见的记忆组合）
    pub fn retrieve(&self, scene_id: u64, owner: &OwnerId, subagent_type: Option<SubAgentType>, limit: usize) -> Vec<Memory> {
        let mut results: Vec<Memory> = Vec::new();
        results.extend(self.global.read().iter().cloned());
        if let Some(s) = self.full_scenes.read().get(&scene_id) { results.extend(s.iter().cloned()); }
        results.retain(|m| m.owners.iter().any(|o| matches!(o, OwnerId::All) || o == owner));
        if let Some(st) = subagent_type { results.retain(|m| m.subagent_type == st || m.subagent_type == SubAgentType::General); }
        results.sort_by(|a,b| b.strength_cached.partial_cmp(&a.strength_cached).unwrap());
        results.truncate(limit);
        results
    }

    pub fn unload_scene(&self, scene_id: u64) {
        let _ = self.flush_scene(scene_id);
        self.full_scenes.write().remove(&scene_id);
    }

    pub fn load_scene(&self, scene_id: u64) {
        if let Some(ref s) = *self.store.read() {
            if let Ok(m) = s.load_scene(scene_id) {
                self.full_scenes.write().insert(scene_id, m);
                return;
            }
        }
        self.full_scenes.write().entry(scene_id).or_default();
    }

    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            global_count: self.global.read().len(),
            scene_count: self.full_scenes.read().len(),
            total_entries: self.global.read().len() + self.full_scenes.read().values().map(|v|v.len()).sum::<usize>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mem(scope: MemoryScope, cat: MemoryCategory, s: f64) -> Memory {
        Memory { agent_id:1, scene_id:1, scope, owners:vec![OwnerId::All], category:cat, subagent_type:SubAgentType::General, enhancement_rate:0.0, decay_rate:0.1, strength_cached:s, last_updated:0, last_validated:0, validation_count:0, source_count:0, seq:0 }
    }
    #[test] fn global_retrievable() {
        let mesh = MemoryMesh::new(MemoryConfig::default());
        mesh.store(mem(MemoryScope::All, MemoryCategory::Knowledge, 10.0));
        assert_eq!(mesh.retrieve(1, &OwnerId::All, None, 10).len(), 1);
    }
    #[test] fn persistence_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mesh = MemoryMesh::new(MemoryConfig::default());
        mesh.init_storage(&tmp.path().to_path_buf()).unwrap();
        mesh.store(mem(MemoryScope::All, MemoryCategory::Knowledge, 10.0));
        mesh.flush().unwrap();
        drop(mesh);
        let mesh2 = MemoryMesh::new(MemoryConfig::default());
        mesh2.init_storage(&tmp.path().to_path_buf()).unwrap();
        assert!(mesh2.global.read().len() > 0);
    }
}
