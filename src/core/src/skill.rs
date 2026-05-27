//! SKILL.md 技能系统 — 读取 OpenClaw 兼容的 Skills

use std::collections::HashMap;
use std::path::Path;

/// 技能定义
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// 技能指令（注入到 Agent 上下文的 markdown）
    pub instructions: String,
    /// 需要哪些工具才能运行
    pub requires: Vec<String>,
    /// 适用的操作系统
    pub os: Option<Vec<String>>,
}

impl Skill {
    /// 从 SKILL.md 文件解析
    pub fn from_file(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let name = path.file_stem()?.to_str()?.to_string();

        // 解析 YAML frontmatter (--- 之间的内容)
        let (frontmatter, instructions) = if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let fm = &content[3..3+end];
                let body = &content[3+end+3..];
                (fm.to_string(), body.to_string())
            } else { (String::new(), content) }
        } else { (String::new(), content) };

        let desc = frontmatter.lines()
            .find(|l| l.starts_with("description:"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_default();

        Some(Self {
            name, description: desc, instructions,
            requires: vec![], os: None,
        })
    }
}

/// 技能注册表
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self { Self { skills: HashMap::new() } }

    pub fn load_from_dir(&mut self, dir: &Path) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(skill) = Skill::from_file(&path) {
                        self.skills.insert(skill.name.clone(), skill);
                    }
                }
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&Skill> { self.skills.get(name) }
    pub fn list(&self) -> Vec<&Skill> { self.skills.values().collect() }
}

impl Default for SkillRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use crate::skill::*;
    #[test]
    fn skill_from_file_fails_on_nonexistent() {
        let s = Skill::from_file(&std::path::PathBuf::from("nonexistent.md"));
        assert!(s.is_none());
    }
}
