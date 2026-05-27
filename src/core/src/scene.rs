use crate::memory::MemoryMesh;
use crate::types::Scene;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 场景管理器——自动检测并切换工作上下文
pub struct SceneManager {
    scenes: RwLock<HashMap<u64, Scene>>,
    current: RwLock<Option<u64>>,
    last_switch: RwLock<std::time::Instant>,
    debounce: std::time::Duration,
    next_id: std::sync::atomic::AtomicU64,
    default_scene: String,
}

impl SceneManager {
    pub fn new(debounce_ms: u64, default_scene: String) -> Self {
        Self {
            scenes: RwLock::new(HashMap::new()),
            current: RwLock::new(None),
            last_switch: RwLock::new(std::time::Instant::now()),
            debounce: std::time::Duration::from_millis(debounce_ms),
            next_id: std::sync::atomic::AtomicU64::new(1),
            default_scene,
        }
    }

    /// 注册场景
    pub fn register(&self, name: String, paths: Vec<PathBuf>) -> u64 {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.scenes.write().insert(id, Scene { id, name, paths, config: Default::default() });
        id
    }

    /// 检测当前目录并自动切换场景
    pub fn detect_and_switch(&self, cwd: &Path, mesh: &MemoryMesh) -> Option<u64> {
        // 防抖
        let mut last = self.last_switch.write();
        if last.elapsed() < self.debounce { return *self.current.read(); }
        *last = std::time::Instant::now();
        drop(last);

        // 检测：当前目录是否有 .bugs/ 或 BUG.md 或 .git/
        let markers = [".bugs", "BUG.md", ".git"];
        let mut matched_id = None;
        let mut matched_name = String::new();

        let scenes = self.scenes.read();
        for (id, scene) in scenes.iter() {
            for path in &scene.paths {
                if cwd.starts_with(path) {
                    matched_id = Some(*id);
                    break;
                }
            }
        }

        // 未匹配现有场景 — 自动注册
        if matched_id.is_none() {
            for marker in &markers {
                if cwd.join(marker).exists() {
                    matched_name = cwd.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    drop(scenes);
                    let id = self.register(matched_name.clone(), vec![cwd.to_path_buf()]);
                    matched_id = Some(id);
                    break;
                }
            }
        }

        if let Some(id) = matched_id {
            let prev = *self.current.read();
            if prev != Some(id) {
                // 卸载旧场景完整记忆
                if let Some(old_id) = prev { mesh.unload_scene(old_id); }
                // 加载新场景完整记忆
                mesh.load_scene(id);
            }
            *self.current.write() = Some(id);
        }

        *self.current.read()
    }

    /// 手动切换场景
    pub fn switch_by_name(&self, name: &str, mesh: &MemoryMesh) -> Option<u64> {
        let scenes = self.scenes.read();
        for (id, scene) in scenes.iter() {
            if scene.name == name {
                let prev = *self.current.read();
                if prev != Some(*id) {
                    if let Some(old_id) = prev { mesh.unload_scene(old_id); }
                    mesh.load_scene(*id);
                }
                *self.current.write() = Some(*id);
                return Some(*id);
            }
        }
        None
    }

    pub fn current(&self) -> Option<u64> { *self.current.read() }
    pub fn current_name(&self) -> Option<String> {
        self.current.read().and_then(|id| self.scenes.read().get(&id).map(|s| s.name.clone()))
    }
    pub fn list(&self) -> Vec<Scene> { self.scenes.read().values().cloned().collect() }
    pub fn create_default(&self) -> u64 {
        self.register(self.default_scene.clone(), vec![PathBuf::from(".")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMesh;
    #[test]
    fn register_and_switch() {
        let mgr = SceneManager::new(0, "general".into());
        let mesh = MemoryMesh::new(crate::types::MemoryConfig::default());
        mgr.register("test".into(), vec!["/tmp".into()]);
        assert!(mgr.switch_by_name("test", &mesh).is_some());
    }
    #[test]
    fn create_default() {
        let mgr = SceneManager::new(0, "general".into());
        mgr.create_default();
        assert_eq!(mgr.list().len(), 1);
    }
    #[test]
    fn debounce_prevents_rapid_switch() {
        let mgr = SceneManager::new(500, "general".into());
        let mesh = MemoryMesh::new(crate::types::MemoryConfig::default());
        let id = mgr.register("test".into(), vec!["/tmp".into()]);
        // rapid switch should be debounced
        mgr.switch_by_name("test", &mesh);
        // store previous
        assert_eq!(mgr.current(), Some(id));
    }
}
