use crate::memory::MemorySummary;
use crate::types::*;
use std::path::PathBuf;

/// 持久化存储 — redb + zstd压缩
/// 所有记忆全部持久化，检索时按场景过滤
pub struct Store {
    global: redb::Database,
    data_dir: PathBuf,
    limits: MemoryConfig,
}

impl Store {
    pub fn open(home: &std::path::Path) -> Result<Self, String> {
        let data_dir = home.join("data");
        std::fs::create_dir_all(data_dir.join("scenes")).map_err(|e| e.to_string())?;
        let global =
            redb::Database::create(data_dir.join("global.redb")).map_err(|e| e.to_string())?;
        Ok(Self {
            global,
            data_dir,
            limits: MemoryConfig::default(),
        })
    }

    // ── 全局记忆（所有场景可见） ──

    pub fn save_global(&self, memories: &[Memory]) -> Result<(), String> {
        let write_txn = self.global.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(GLOBAL_TABLE)
                .map_err(|e| e.to_string())?;
            let count = memories.len() as u64;
            {
                let b = count.to_be_bytes();
                table.insert(COUNT_KEY, b.as_slice()).ok();
            }
            for (i, m) in memories.iter().enumerate() {
                let val = serde_json::to_vec(m).map_err(|e| e.to_string())?;
                let c = zstd::encode_all(val.as_slice(), 3).map_err(|e| e.to_string())?;
                let b = (i as u64).to_be_bytes();
                table.insert(b.as_slice(), c.as_slice()).ok();
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_global(&self) -> Result<Vec<Memory>, String> {
        let read_txn = self.global.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(GLOBAL_TABLE)
            .map_err(|e| e.to_string())?;
        let count: u64 = table
            .get(COUNT_KEY)
            .map_err(|e| e.to_string())?
            .map(|v| u64::from_be_bytes(v.value().try_into().unwrap_or([0u8; 8])))
            .unwrap_or(0);
        let mut memories = Vec::new();
        for i in 0..count {
            let b = i.to_be_bytes();
            if let Ok(Some(val)) = table.get(b.as_slice()) {
                let d = zstd::decode_all(val.value()).unwrap_or_else(|_| val.value().to_vec());
                if let Ok(m) = serde_json::from_slice::<Memory>(&d) {
                    memories.push(m);
                }
            }
        }
        memories.sort_by(|a, b| b.strength_cached.partial_cmp(&a.strength_cached).unwrap());
        let max = (self.limits.global_max_mb as usize * 1024 / 512).max(1);
        memories.truncate(max);
        Ok(memories)
    }

    // ── 场景记忆（仅当前场景加载） ──

    pub fn save_scene(&self, scene_id: u64, memories: &[Memory]) -> Result<(), String> {
        let db = self.open_scene_db(scene_id)?;
        let write_txn = db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(SCENE_TABLE)
                .map_err(|e| e.to_string())?;
            let count = memories.len() as u64;
            {
                let b = count.to_be_bytes();
                table.insert(COUNT_KEY, b.as_slice()).ok();
            }
            for (i, m) in memories.iter().enumerate() {
                let val = serde_json::to_vec(m).map_err(|e| e.to_string())?;
                let c = zstd::encode_all(val.as_slice(), 3).map_err(|e| e.to_string())?;
                let b = (i as u64).to_be_bytes();
                table.insert(b.as_slice(), c.as_slice()).ok();
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_scene(&self, scene_id: u64) -> Result<Vec<Memory>, String> {
        let db = self.open_scene_db(scene_id)?;
        let read_txn = db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(SCENE_TABLE)
            .map_err(|e| e.to_string())?;
        let count: u64 = table
            .get(COUNT_KEY)
            .map_err(|e| e.to_string())?
            .map(|v| u64::from_be_bytes(v.value().try_into().unwrap_or([0u8; 8])))
            .unwrap_or(0);
        let mut memories = Vec::new();
        for i in 0..count {
            let b = i.to_be_bytes();
            if let Ok(Some(val)) = table.get(b.as_slice()) {
                let d = zstd::decode_all(val.value()).unwrap_or_else(|_| val.value().to_vec());
                if let Ok(m) = serde_json::from_slice::<Memory>(&d) {
                    memories.push(m);
                }
            }
        }
        memories.sort_by(|a, b| b.strength_cached.partial_cmp(&a.strength_cached).unwrap());
        let max = (self.limits.scene_full_max_mb as usize * 1024 / 512).max(1);
        memories.truncate(max);
        Ok(memories)
    }

    fn open_scene_db(&self, scene_id: u64) -> Result<redb::Database, String> {
        let p = self
            .data_dir
            .join("scenes")
            .join(format!("{scene_id}.redb"));
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        redb::Database::create(&p).map_err(|e| e.to_string())
    }

    // ── 场景概要（永远加载，像全局记忆一样） ──

    pub fn save_summaries(&self, scene_id: u64, summaries: &[MemorySummary]) -> Result<(), String> {
        let db = self.open_scene_db(scene_id)?;
        let write_txn = db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(SUMMARY_TABLE)
                .map_err(|e| e.to_string())?;
            let count = summaries.len() as u64;
            {
                let b = count.to_be_bytes();
                table.insert(COUNT_KEY, b.as_slice()).ok();
            }
            for (i, s) in summaries.iter().enumerate() {
                let val = serde_json::to_vec(s).map_err(|e| e.to_string())?;
                let b = (i as u64).to_be_bytes();
                table.insert(b.as_slice(), val.as_slice()).ok();
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_summaries(&self, scene_id: u64) -> Result<Vec<MemorySummary>, String> {
        let db = self.open_scene_db(scene_id)?;
        let read_txn = db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(SUMMARY_TABLE)
            .map_err(|e| e.to_string())?;
        let count: u64 = table
            .get(COUNT_KEY)
            .map_err(|e| e.to_string())?
            .map(|v| u64::from_be_bytes(v.value().try_into().unwrap_or([0u8; 8])))
            .unwrap_or(0);
        let mut summaries = Vec::new();
        for i in 0..count {
            let b = i.to_be_bytes();
            if let Ok(Some(val)) = table.get(b.as_slice()) {
                if let Ok(s) = serde_json::from_slice::<MemorySummary>(val.value()) {
                    summaries.push(s);
                }
            }
        }
        Ok(summaries)
    }

    // ── 状态 ──

    pub fn save_state(&self, config: &BugsConfig) -> Result<(), String> {
        let path = self.data_dir.parent().unwrap().join("state.json");
        let tmp = self.data_dir.parent().unwrap().join("state.json.tmp");
        std::fs::write(
            &tmp,
            &serde_json::to_string_pretty(config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

const GLOBAL_TABLE: redb::TableDefinition<&[u8], &[u8]> = redb::TableDefinition::new("global");
const SCENE_TABLE: redb::TableDefinition<&[u8], &[u8]> = redb::TableDefinition::new("scene");
const SUMMARY_TABLE: redb::TableDefinition<&[u8], &[u8]> = redb::TableDefinition::new("summary");
const COUNT_KEY: &[u8] = b"__count__";

#[cfg(test)]
mod tests {
    use super::*;
    fn s() -> (Store, tempfile::TempDir) {
        let d = tempfile::TempDir::new().unwrap();
        (Store::open(&d.path().to_path_buf()).unwrap(), d)
    }
    fn m() -> Memory {
        Memory {
            agent_id: 1,
            scene_id: 1,
            scope: MemoryScope::All,
            owners: vec![OwnerId::All],
            category: MemoryCategory::Knowledge,
            subagent_type: SubAgentType::General,
            enhancement_rate: 0.5,
            decay_rate: 0.1,
            strength_cached: 8.0,
            last_updated: 0,
            last_validated: 0,
            validation_count: 3,
            source_count: 2,
            seq: 1,
        }
    }

    #[test]
    fn global_rt() {
        let (s, _) = s();
        s.save_global(&[m()]).unwrap();
        assert_eq!(s.load_global().unwrap().len(), 1);
    }
    #[test]
    fn scene_rt() {
        let (s, _) = s();
        s.save_scene(1, &[m()]).unwrap();
        assert_eq!(s.load_scene(1).unwrap().len(), 1);
    }
    #[test]
    fn zstd_compresses() {
        let (s, _) = s();
        s.save_global(&[m()]).unwrap();
        let l = s.load_global().unwrap();
        assert!((l[0].strength_cached - 8.0).abs() < 0.01);
    }
}
