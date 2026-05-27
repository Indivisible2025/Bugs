use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[async_trait]
pub trait BugModule: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    async fn on_install(&self) -> Result<(), ModuleError>;
    async fn on_enable(&self) -> Result<(), ModuleError>;
    async fn on_disable(&self) -> Result<(), ModuleError>;
    async fn on_uninstall(&self) -> Result<(), ModuleError>;
    async fn run(&self) -> Result<ModuleRunHandle, ModuleError>;
    fn capabilities(&self) -> Vec<ModuleCapability>;
    fn permissions(&self) -> Vec<ModulePermission>;
}

pub struct ModuleRunHandle { pub abort: tokio::sync::oneshot::Sender<()> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleCapability {
    TextInference, SpeechSynthesis, SpeechRecognition,
    WebSearch, Channel { name: String }, Tool { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModulePermission { FileSystemRead, FileSystemWrite, NetworkOutbound, ExecuteCommand, AccessMemory }

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("模块未找到")] NotFound,
    #[error("版本不兼容: 需要 Bugs >= {0}")] VersionMismatch(String),
    #[error("安装失败: {0}")] InstallFailed(String),
    #[error("运行时错误: {0}")] Runtime(String),
}

#[derive(Debug, Clone)]
pub struct ModuleManifest {
    pub id: String, pub name: String, pub version: String, pub description: String,
    pub author: String, pub bugs_version: String,
    pub capabilities: Vec<ModuleCapability>, pub permissions: Vec<ModulePermission>,
    pub install_path: PathBuf,
}

pub struct ModuleRegistry {
    modules: RwLock<HashMap<String, Arc<dyn BugModule>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self { Self { modules: RwLock::new(HashMap::new()) } }
    pub fn register(&self, module: Arc<dyn BugModule>) {
        self.modules.write().insert(module.id().to_string(), module);
    }
    pub fn list(&self) -> Vec<String> {
        self.modules.read().keys().cloned().collect()
    }
    pub fn get(&self, id: &str) -> Option<Arc<dyn BugModule>> {
        self.modules.read().get(id).cloned()
    }
}

impl Default for ModuleRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use crate::module::*;
    #[test]
    fn registry_works() {
        let r = ModuleRegistry::new();
        assert!(r.list().is_empty());
    }
}
