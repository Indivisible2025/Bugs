//! 搜索引擎模块 — 多引擎支持，默认 Bing

use std::collections::HashMap;

/// 搜索引擎配置
#[derive(Debug, Clone)]
pub struct SearchEngine {
    /// 引擎名称
    pub name: String,
    /// 搜索 URL 模板（{query} 会被替换）
    pub url_template: String,
    /// 结果页的内容提取 JS
    pub result_extract_js: &'static str,
}

/// 搜索引擎注册表
pub struct SearchRegistry {
    engines: HashMap<String, SearchEngine>,
    default_engine: String,
}

/// 通用搜索结果提取 JS（适用于大多数搜索引擎）
const GENERIC_EXTRACT_JS: &str = r#"
(function(){
    var results = [];
    document.querySelectorAll('a[href]').forEach(function(a){
        var text = (a.textContent||'').trim();
        if (text.length > 10) results.push({title: text.substring(0,100), url: a.href});
    });
    return JSON.stringify(results.slice(0,10));
})()
"#;

impl SearchRegistry {
    pub fn new() -> Self {
        let mut engines = HashMap::new();

        // Bing（默认——国内外都可用）
        engines.insert(
            "bing".into(),
            SearchEngine {
                name: "Bing".into(),
                url_template: "https://www.bing.com/search?q={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        // Google
        engines.insert(
            "google".into(),
            SearchEngine {
                name: "Google".into(),
                url_template: "https://www.google.com/search?q={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        // 百度
        engines.insert(
            "baidu".into(),
            SearchEngine {
                name: "百度".into(),
                url_template: "https://www.baidu.com/s?wd={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        // DuckDuckGo
        engines.insert(
            "duckduckgo".into(),
            SearchEngine {
                name: "DuckDuckGo".into(),
                url_template: "https://duckduckgo.com/?q={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        // 必应中国
        engines.insert(
            "bing-cn".into(),
            SearchEngine {
                name: "Bing 中国".into(),
                url_template: "https://cn.bing.com/search?q={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        // SearXNG（自部署，隐私优先）
        engines.insert(
            "searxng".into(),
            SearchEngine {
                name: "SearXNG".into(),
                url_template: "https://searx.be/search?q={query}".into(),
                result_extract_js: GENERIC_EXTRACT_JS,
            },
        );

        Self {
            engines,
            default_engine: "bing".into(),
        }
    }

    /// 获取引擎
    pub fn get(&self, name: &str) -> Option<&SearchEngine> {
        self.engines.get(name)
    }

    /// 获取默认引擎
    pub fn default_engine(&self) -> &SearchEngine {
        self.engines.get(&self.default_engine).unwrap()
    }

    /// 设置默认引擎
    pub fn set_default(&mut self, name: &str) -> Result<(), String> {
        if self.engines.contains_key(name) {
            self.default_engine = name.into();
            Ok(())
        } else {
            Err(format!("未知搜索引擎: {name}"))
        }
    }

    /// 列出所有可用引擎
    pub fn list(&self) -> Vec<String> {
        self.engines.keys().cloned().collect()
    }

    /// 注册自定义搜索引擎
    pub fn register(&mut self, id: String, engine: SearchEngine) {
        self.engines.insert(id, engine);
    }

    /// 构造搜索 URL
    pub fn build_url(&self, engine: &str, query: &str) -> Option<String> {
        self.get(engine)
            .map(|e| e.url_template.replace("{query}", &query.replace(' ', "+")))
    }
}

impl Default for SearchRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_bing() {
        let reg = SearchRegistry::new();
        assert_eq!(reg.default_engine().name, "Bing");
    }

    #[test]
    fn switch_to_google() {
        let mut reg = SearchRegistry::new();
        assert!(reg.set_default("google").is_ok());
        assert_eq!(reg.default_engine().name, "Google");
    }

    #[test]
    fn build_bing_url() {
        let reg = SearchRegistry::new();
        let url = reg.build_url("bing", "rust async").unwrap();
        assert!(url.contains("bing.com"));
        assert!(url.contains("rust+async"));
    }

    #[test]
    fn unknown_engine_returns_none() {
        let reg = SearchRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn register_custom_engine() {
        let mut reg = SearchRegistry::new();
        reg.register(
            "custom".into(),
            SearchEngine {
                name: "Custom".into(),
                url_template: "https://example.com?q={query}".into(),
                result_extract_js: "",
            },
        );
        assert!(reg.get("custom").is_some());
    }
}
