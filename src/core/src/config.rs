use crate::types::BugsConfig;
use std::path::PathBuf;

/// 配置加载器——默认配置即可用，不需要任何文件。
pub struct ConfigLoader {
    /// 配置目录，默认 ~/.bugs/
    pub home: PathBuf,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        let home = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        Self { home }
    }
}

impl ConfigLoader {
    /// 加载配置：先加载默认值，再尝试读取 config.json 覆盖。
    pub fn load(&self) -> BugsConfig {
        let mut config = BugsConfig::default();
        let config_path = self.home.join("config.json");

        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                // 用 JSON5 解析（支持注释和尾逗号）
                if let Ok(user_config) = json5::from_str::<BugsConfig>(&content) {
                    config = user_config;
                }
            }
        }

        config
    }

    /// 确保 ~/.bugs/ 目录存在。
    pub fn ensure_home(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.home)
    }
}

/// 获取默认的 ~/.bugs/ 路径
fn dirs_next() -> Option<PathBuf> {
    std::env::var("BUGS_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".bugs"))
        })
}

/// 最小化的 dirs 实现——不引入 dirs crate。
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| {
                std::env::var_os("USERPROFILE")
            })
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_loaded_without_files() {
        let loader = ConfigLoader::default();
        let config = loader.load();
        assert_eq!(config.scheduler.global_max_parallelism, 100_000);
    }
}
