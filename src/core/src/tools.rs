use crate::types::*;
use std::collections::HashMap;

/// 工具注册中心——Agent 可调用的工具库。
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

/// 工具定义
pub struct ToolDefinition {
    pub name: String,
    pub group: String,
    pub description: String,
    pub required: bool,    // 必选工具不可卸载
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };

        // 必选工具
        registry.register(ToolDefinition { name: "read".into(), group: "group:fs".into(), description: "读取文件".into(), required: true });
        registry.register(ToolDefinition { name: "write".into(), group: "group:fs".into(), description: "写入文件".into(), required: true });
        registry.register(ToolDefinition { name: "edit".into(), group: "group:fs".into(), description: "修改文件".into(), required: true });
        registry.register(ToolDefinition { name: "ls".into(), group: "group:fs".into(), description: "列出目录".into(), required: true });
        registry.register(ToolDefinition { name: "exec".into(), group: "group:runtime".into(), description: "执行命令".into(), required: true });
        registry.register(ToolDefinition { name: "memory_search".into(), group: "group:memory".into(), description: "检索记忆".into(), required: true });
        registry.register(ToolDefinition { name: "memory_store".into(), group: "group:memory".into(), description: "写入记忆".into(), required: true });
        registry.register(ToolDefinition { name: "knowledge_query".into(), group: "group:knowledge".into(), description: "知识查询".into(), required: true });

        // 可选工具
        registry.register(ToolDefinition { name: "browser".into(), group: "group:browser".into(), description: "浏览器搜索".into(), required: false });
        registry.register(ToolDefinition { name: "code_exec".into(), group: "group:sandbox".into(), description: "代码沙箱执行".into(), required: false });

        registry
    }

    fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// 列出所有可用工具
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// 检查是否允许该工具
    pub fn is_allowed(&self, name: &str, config: &ToolConfig) -> bool {
        if let Some(tool) = self.tools.get(name) {
            if tool.required {
                return !config.deny.iter().any(|d| d == &tool.group || d == &tool.name);
            }
            return config.allow.iter().any(|a| a == &tool.group || a == &tool.name);
        }
        false
    }

    /// 按分组获取工具
    pub fn by_group(&self, group: &str) -> Vec<&ToolDefinition> {
        self.tools.values().filter(|t| t.group == group).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            allow: vec!["group:fs".into(), "group:runtime".into(), "group:memory".into(), "group:knowledge".into()],
            deny: vec!["group:browser".into(), "group:sandbox".into()],
            protection_paths: vec!["~/.bugs/".into(), "~/.ssh/".into(), "/etc/".into()],
            granted: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_tools_cannot_be_denied() {
        let registry = ToolRegistry::new();
        let config = ToolConfig::default();
        assert!(registry.is_allowed("read", &config));
    }

    #[test]
    fn optional_tools_default_denied() {
        let registry = ToolRegistry::new();
        let config = ToolConfig::default();
        assert!(!registry.is_allowed("browser", &config));
    }
}
