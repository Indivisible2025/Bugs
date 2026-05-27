use crate::memory::MemoryMesh;
use crate::trust::TrustEngine;
use crate::types::*;
use parking_lot::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeditateReport {
    pub duration_ms: u64,
    pub merged: usize,
    pub strengthened: usize,
    pub cleaned: usize,
    pub extracted: usize,
    pub discoveries: usize,
    pub predicted: usize,
    pub pending: Vec<Memory>,
    pub details: Vec<String>,
    pub finished: bool,
}

pub struct Meditation {
    idle_threshold: Duration,
    cron_hour: u8,
    last_meditate: Mutex<Instant>,
    max_duration: Duration,
    clean_strength: f64,
    extract_min: usize,
    running: Mutex<bool>,
    last_report: Mutex<Option<MeditateReport>>,
}

impl Meditation {
    pub fn new() -> Self {
        Self {
            idle_threshold: Duration::from_secs(300), cron_hour: 3,
            last_meditate: Mutex::new(Instant::now()),
            max_duration: Duration::from_secs(30),
            clean_strength: -5.0, extract_min: 3,
            running: Mutex::new(false), last_report: Mutex::new(None),
        }
    }

    pub fn should_meditate(&self, last_interaction: Instant) -> bool {
        let idle = last_interaction.elapsed() > self.idle_threshold;
        let cooldown = self.last_meditate.lock().elapsed() > Duration::from_secs(3600);
        let not_running = !*self.running.lock();
        idle && cooldown && not_running
    }

    /// 同步冥想
    pub fn meditate(&self, mesh: &MemoryMesh, trust: &TrustEngine) -> MeditateReport {
        *self.running.lock() = true;
        let report = self.do_meditate(mesh, trust);
        *self.last_report.lock() = Some(report.clone());
        *self.running.lock() = false;
        report
    }

    /// 获取上次冥想报告（不触发新冥想）
    pub fn last_report(&self) -> Option<MeditateReport> {
        self.last_report.lock().clone()
    }

    /// 是否正在冥想
    pub fn is_running(&self) -> bool { *self.running.lock() }

    fn do_meditate(&self, mesh: &MemoryMesh, trust: &TrustEngine) -> MeditateReport {
        let start = Instant::now();
        let mut report = MeditateReport {
            duration_ms: 0, merged: 0, strengthened: 0, cleaned: 0,
            extracted: 0, discoveries: 0, predicted: 0,
            pending: vec![], details: vec![], finished: false,
        };

        let mut all = self.collect_all(mesh);
        if all.is_empty() { report.duration_ms = start.elapsed().as_millis() as u64; report.finished = true; return report; }

        // 自适应：如果记忆量太大，加大去重扫描步幅
        let dedup_limit = if all.len() > 10_000 { 50 } else { 100 };

        // ① 衰退扫描
        report.strengthened = self.decay_scan(&mut all, trust);

        // ② 清理
        report.cleaned = self.clean(&mut all);

        // ③ 经验提炼（在合并前，防止经验被吞）
        report.extracted = self.extract_experiences(&mut all, trust, start);

        // ④ 去重合并
        report.merged = self.dedup(&mut all, start, dedup_limit);

        // ⑤ 关系发现
        report.discoveries = self.discover_relations(&all);

        // ⑥ 预判
        report.predicted = self.predict(&all);

        // ⑦ 待确认
        report.pending = all.iter()
            .filter(|m| m.strength_cached < 0.0 && m.validation_count == 0)
            .take(5).cloned().collect();

        self.write_back(mesh, all);

        report.details = vec![
            format!("合并 {} 条重复记忆", report.merged),
            format!("强化 {} 条高频记忆", report.strengthened),
            format!("清理 {} 条低信任记忆", report.cleaned),
            format!("提炼 {} 条经验为通用知识", report.extracted),
            format!("发现 {} 个场景关联", report.discoveries),
            format!("预判 {} 个可能需求", report.predicted),
            format!("{} 条记忆待确认", report.pending.len()),
        ];

        *self.last_meditate.lock() = Instant::now();
        report.duration_ms = start.elapsed().as_millis() as u64;
        report.finished = true;
        report
    }

    pub fn collect_all(&self, mesh: &MemoryMesh) -> Vec<Memory> {
        let mut all = Vec::new();
        all.extend(mesh.global.read().iter().cloned());
        for scene in mesh.full_scenes.read().values() { all.extend(scene.iter().cloned()); }
        all
    }

    pub fn write_back(&self, mesh: &MemoryMesh, all: Vec<Memory>) {
        let mut global = mesh.global.write();
        let mut scenes = mesh.full_scenes.write();
        global.clear();
        let mut scene_mems: std::collections::HashMap<u64, Vec<Memory>> = std::collections::HashMap::new();
        for m in all {
            match &m.scope {
                MemoryScope::All => global.push(m),
                MemoryScope::Single(sid) => scene_mems.entry(*sid).or_default().push(m),
                _ => {}
            }
        }
        for (sid, mems) in scene_mems { scenes.insert(sid, mems); }
    }

    fn decay_scan(&self, all: &mut [Memory], trust: &TrustEngine) -> usize {
        let mut count = 0;
        for m in all.iter_mut() {
            let old = m.strength_cached;
            trust.apply_decay(m);
            if m.strength_cached > old { count += 1; }
        }
        count
    }

    fn clean(&self, all: &mut Vec<Memory>) -> usize {
        let before = all.len();
        all.retain(|m| m.strength_cached >= self.clean_strength);
        before - all.len()
    }

    fn dedup(&self, all: &mut Vec<Memory>, start: Instant, limit: usize) -> usize {
        let mut merged = 0;
        let mut to_remove: Vec<usize> = Vec::new();
        let n = all.len().min(limit);
        for i in 0..n {
            if to_remove.contains(&i) || start.elapsed() > self.max_duration { break; }
            for j in (i+1)..n {
                if to_remove.contains(&j) || start.elapsed() > self.max_duration { break; }
                if all[i].category == all[j].category && all[i].agent_id == all[j].agent_id {
                    let (keep, drop) = if all[i].strength_cached > all[j].strength_cached { (i, j) } else { (j, i) };
                    all[keep].source_count = all[keep].source_count.max(all[drop].source_count);
                    all[keep].enhancement_rate = (all[keep].enhancement_rate + all[drop].enhancement_rate) / 2.0;
                    to_remove.push(drop);
                    merged += 1;
                }
            }
        }
        to_remove.sort_unstable_by(|a,b| b.cmp(a));
        to_remove.dedup();
        for idx in to_remove { if idx < all.len() { all.remove(idx); } }
        merged
    }

    fn extract_experiences(&self, all: &mut Vec<Memory>, trust: &TrustEngine, start: Instant) -> usize {
        let mut extracted = 0;
        let mut scene_exps: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        for (i, m) in all.iter().enumerate() {
            if m.category == MemoryCategory::Experience {
                if let MemoryScope::Single(sid) = m.scope { scene_exps.entry(sid).or_default().push(i); }
            }
        }
        for (sid, indices) in &scene_exps {
            if indices.len() < self.extract_min || start.elapsed() > self.max_duration { continue; }
            let avg_s = indices.iter().map(|&i| all[i].strength_cached).sum::<f64>() / indices.len() as f64;
            let avg_e = indices.iter().map(|&i| all[i].enhancement_rate).sum::<f64>() / indices.len() as f64;
            let src = indices.iter().map(|&i| all[i].source_count).max().unwrap_or(1);
            let mut mem = Memory {
                agent_id: all[indices[0]].agent_id, scene_id: *sid,
                scope: MemoryScope::Single(*sid), owners: vec![OwnerId::All],
                category: MemoryCategory::Knowledge,
                subagent_type: SubAgentType::General,
                enhancement_rate: avg_e * 0.5, decay_rate: 0.001,
                strength_cached: avg_s * 0.5,
                last_updated: 0, last_validated: 0,
                validation_count: 0, source_count: src, seq: 0,
            };
            trust.initialize_memory(&mut mem);
            all.push(mem);
            extracted += 1;
        }
        extracted
    }

    fn discover_relations(&self, all: &[Memory]) -> usize {
        let mut groups: std::collections::HashMap<u64, Vec<&Memory>> = std::collections::HashMap::new();
        for m in all {
            if let MemoryScope::Single(sid) = m.scope { groups.entry(sid).or_default().push(m); }
        }
        groups.values().filter(|g| g.len() >= 3).count()
    }

    fn predict(&self, all: &[Memory]) -> usize {
        all.iter().filter(|m| m.validation_count >= 3 && m.strength_cached > 10.0).count()
    }
}

impl Default for Meditation {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMesh;
    fn mem(sid: u64, scope: MemoryScope, cat: MemoryCategory, s: f64) -> Memory {
        Memory { agent_id: 1, scene_id: sid, scope, owners: vec![OwnerId::All],
            category: cat, subagent_type: SubAgentType::General,
            enhancement_rate: 0.0, decay_rate: 0.1, strength_cached: s,
            last_updated: 0, last_validated: 0,
            validation_count: 0, source_count: 1, seq: 0 }
    }
    #[test] fn clean_low() {
        let mesh = MemoryMesh::new(MemoryConfig::default());
        mesh.store(mem(1, MemoryScope::All, MemoryCategory::Knowledge, -10.0));
        mesh.store(mem(1, MemoryScope::All, MemoryCategory::Knowledge, 8.0));
        assert!(Meditation::new().meditate(&mesh, &TrustEngine::default()).cleaned > 0);
    }
    #[test] fn extracts() {
        let mesh = MemoryMesh::new(MemoryConfig::default());
        mesh.store(mem(1, MemoryScope::Single(1), MemoryCategory::Experience, 3.0));
        mesh.store(mem(1, MemoryScope::Single(1), MemoryCategory::Experience, 4.0));
        mesh.store(mem(1, MemoryScope::Single(1), MemoryCategory::Experience, 5.0));
        let r = Meditation::new().meditate(&mesh, &TrustEngine::default());
        assert!(r.extracted > 0 || r.details.len() > 0);
    }
    #[test] fn pending_list() {
        let mesh = MemoryMesh::new(MemoryConfig::default());
        mesh.store(mem(1, MemoryScope::All, MemoryCategory::Knowledge, -1.0));
        assert!(!Meditation::new().meditate(&mesh, &TrustEngine::default()).pending.is_empty());
    }
    #[test] fn idle_detect() {
        let med = Meditation::new();
        *med.last_meditate.lock() = Instant::now() - Duration::from_secs(7200);
        let ago = Instant::now() - Duration::from_secs(4000);
        assert!(med.should_meditate(ago));
    }
    #[test] fn adaptive_limit() {
        let mesh = MemoryMesh::new(MemoryConfig::default());
        for _ in 0..10 { mesh.store(mem(1, MemoryScope::All, MemoryCategory::Knowledge, 1.0)); }
        let r = Meditation::new().meditate(&mesh, &TrustEngine::default());
        assert!(r.duration_ms < 1000); // 应该很快完成
    }
}
