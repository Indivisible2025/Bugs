#![allow(dead_code)]
#![allow(unused_variables)]
//! 浏览器模块 — 基于 Chromium DevTools Protocol 双池架构
//! 参考 Chromium 源码: headless_mode, BrowserContext, --dump-dom

use async_trait::async_trait;
use bugs_core::module::{
    BugModule, ModuleCapability, ModuleError, ModulePermission, ModuleRunHandle,
};
use headless_chrome::{Browser, LaunchOptions};
use parking_lot::Mutex;
use rand::Rng;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod behavior;
mod captcha;
mod download;
mod search;
mod stealth;

use download::ChromiumBundle;
use search::SearchRegistry;

// ── 核心组件 ──

pub struct BrowserModule {
    manager: Arc<BrowserManager>,
    chromium_path: parking_lot::Mutex<Option<PathBuf>>,
    search_registry: parking_lot::RwLock<SearchRegistry>,
}

use std::path::PathBuf;

struct BrowserManager {
    headless_pool: Mutex<ContextPool>,
    headful_pool: Mutex<ContextPool>,
    chrome_path: Option<String>,
}

struct ContextEntry {
    browser: Arc<Browser>,
    tabs_used: usize,
    fingerprint: BrowserFingerprint,
    created_at: Instant,
    last_used: Instant,
    search_count: u32,
}

/// 上下文池 — 借鉴 Chromium BrowserContext 隔离
struct ContextPool {
    contexts: Vec<ContextEntry>,
    max_size: usize,
    max_tabs_per_context: usize,
    idle_dormant: Duration, // 30s 休眠
    idle_destroy: Duration, // 300s 销毁
    recycle_interval: u32,  // 50 次回收
}

/// 反检测指纹 — 借鉴 Chromium components/embedder_support/user_agent_utils
#[derive(Debug, Clone)]
struct BrowserFingerprint {
    user_agent: String,
    viewport_width: u32,
    viewport_height: u32,
    locale: String,
    timezone: String,
    platform: String,
}

// ── 反检测参数库 ──

const UAS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Edg/124.0.0.0 Safari/537.36",
];
const RESOLUTIONS: &[(u32, u32)] = &[(1920, 1080), (2560, 1440), (1366, 768), (1440, 900)];
const LOCALES: &[&str] = &["zh-CN", "en-US", "ja-JP", "de-DE"];
const TIMEZONES: &[&str] = &[
    "Asia/Shanghai",
    "America/New_York",
    "Europe/London",
    "Asia/Tokyo",
];
const PLATFORMS: &[&str] = &["Win32", "MacIntel", "Linux x86_64"];

fn random_fingerprint() -> BrowserFingerprint {
    let mut rng = rand::thread_rng();
    BrowserFingerprint {
        user_agent: UAS[rng.gen_range(0..UAS.len())].into(),
        viewport_width: RESOLUTIONS[rng.gen_range(0..RESOLUTIONS.len())].0,
        viewport_height: RESOLUTIONS[rng.gen_range(0..RESOLUTIONS.len())].1,
        locale: LOCALES[rng.gen_range(0..LOCALES.len())].into(),
        timezone: TIMEZONES[rng.gen_range(0..TIMEZONES.len())].into(),
        platform: PLATFORMS[rng.gen_range(0..PLATFORMS.len())].into(),
    }
}

// ── 内容提取（借鉴 Chromium --dump-dom） ──

/// JS 注入脚本 — 类似 Chromium headless 的 --dump-dom 但更智能
const EXTRACT_JS: &str = r#"
(function(){
    // 移除干扰元素
    var bad = document.querySelectorAll('script,style,nav,footer,iframe,aside,svg,.ad,.ads,.sidebar,.banner,.popup,[role="banner"],[role="navigation"]');
    bad.forEach(function(e){e.remove();});
    // 提取主要内容区域
    var main = document.querySelector('main,article,[role="main"],.content,.post,.article,#content');
    if (!main) main = document.body;
    // 提取结构化内容
    var text = main.innerText || main.textContent || '';
    // 保留链接
    var links = [];
    main.querySelectorAll('a[href]').forEach(function(a){
        var href = a.getAttribute('href');
        if (href && !href.startsWith('#') && !href.startsWith('javascript:')) {
            links.push(href + ' | ' + (a.textContent||'').trim().substring(0,60));
        }
    });
    var linkText = links.length > 0 ? '\n\n链接:\n' + links.slice(0,10).join('\n') : '';
    return text.replace(/\s+/g,' ').trim().substring(0,8000) + linkText;
})()
"#;

// ── BrowserModule ──

impl BrowserModule {
    pub fn new() -> Self {
        let cpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let pool_size = (cpu * 2).min(16);
        Self {
            manager: Arc::new(BrowserManager {
                headless_pool: Mutex::new(ContextPool::new(pool_size, 8)),
                headful_pool: Mutex::new(ContextPool::new(2, 2)),
                chrome_path: None,
            }),
            chromium_path: parking_lot::Mutex::new(None),
            search_registry: parking_lot::RwLock::new(SearchRegistry::new()),
        }
    }

    /// 获取 Chromium 路径（自动下载）
    fn get_chrome_path(&self) -> Result<Option<String>, ModuleError> {
        if let Ok(p) = std::env::var("CHROME_PATH") {
            return Ok(Some(p));
        }
        if let Some(ref p) = *self.chromium_path.lock() {
            return Ok(Some(p.display().to_string()));
        }
        let bundle = ChromiumBundle::new();
        let path = bundle.bundle()?;
        let path_str = path.display().to_string();
        *self.chromium_path.lock() = Some(path);
        Ok(Some(path_str))
    }

    /// 池状态
    pub fn status(&self) -> String {
        let h = &self.manager.headless_pool.lock();
        let f = &self.manager.headful_pool.lock();
        format!(
            "Headless: {}/{} | Headful: {}/{}",
            h.contexts.len(),
            h.max_size,
            f.contexts.len(),
            f.max_size
        )
    }

    /// Headless 抓取
    pub fn fetch(&self, url: &str, timeout_secs: u64) -> Result<String, ModuleError> {
        let chrome_path = self.get_chrome_path()?;
        self.manager
            .headless_pool
            .lock()
            .execute(url, timeout_secs, true, &chrome_path)
    }

    /// Headful 模式抓取
    pub fn fetch_headful(&self, url: &str) -> Result<String, ModuleError> {
        let chrome_path = self.get_chrome_path()?;
        self.manager
            .headful_pool
            .lock()
            .execute(url, 60, false, &chrome_path)
    }

    /// 设置代理
    pub fn set_proxy(&self, proxy: &str) {
        std::env::set_var("HTTPS_PROXY", proxy);
        std::env::set_var("HTTP_PROXY", proxy);
    }

    /// 搜索 — 使用默认引擎（Bing）
    pub fn search(&self, query: &str) -> Result<String, ModuleError> {
        let engine = self.search_registry.read().default_engine().name.clone();
        self.search_with(&engine, query)
    }

    /// 指定搜索引擎搜索
    pub fn search_with(&self, engine: &str, query: &str) -> Result<String, ModuleError> {
        let url = self
            .search_registry
            .read()
            .build_url(engine, query)
            .ok_or_else(|| ModuleError::Runtime(format!("未知搜索引擎: {engine}")))?;
        self.fetch(&url, 15)
    }

    /// 设置默认搜索引擎
    pub fn set_default_engine(&self, name: &str) -> Result<(), String> {
        self.search_registry.write().set_default(name)
    }

    /// 列出所有引擎
    pub fn list_engines(&self) -> Vec<String> {
        self.search_registry.read().list()
    }

    /// 添加自定义搜索引擎
    pub fn add_engine(&self, id: String, name: String, url_template: String) {
        self.search_registry.write().register(
            id,
            search::SearchEngine {
                name,
                url_template,
                result_extract_js: "",
            },
        );
    }
}

impl Default for BrowserModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrowserModule {
    fn drop(&mut self) {
        // 借鉴 Chromium — 先回收再清理临时 user-data-dir
        self.manager.headless_pool.lock().contexts.clear();
        self.manager.headful_pool.lock().contexts.clear();
    }
}

// ── ContextPool — 借鉴 Chromium BrowserContext 模型 ──

impl ContextPool {
    fn new(max_size: usize, max_tabs: usize) -> Self {
        Self {
            contexts: vec![],
            max_size,
            max_tabs_per_context: max_tabs,
            idle_dormant: Duration::from_secs(30),
            idle_destroy: Duration::from_secs(300),
            recycle_interval: 50,
        }
    }

    fn get_or_create(
        &mut self,
        chrome_path: &Option<String>,
        headless: bool,
    ) -> Result<(usize, Arc<Browser>), ModuleError> {
        // 找空闲 Context
        for (i, entry) in self.contexts.iter().enumerate() {
            if entry.tabs_used < self.max_tabs_per_context {
                return Ok((i, entry.browser.clone()));
            }
        }
        // 创建新 Context（独立 user-data-dir = 独立 Cookie/指纹）
        if self.contexts.len() < self.max_size {
            let fp = random_fingerprint();
            let browser = launch_browser(chrome_path, headless, &fp)?;
            let entry = ContextEntry {
                browser: browser.clone(),
                tabs_used: 0,
                fingerprint: fp,
                created_at: Instant::now(),
                last_used: Instant::now(),
                search_count: 0,
            };
            let idx = self.contexts.len();
            self.contexts.push(entry);
            return Ok((idx, browser));
        }
        Err(ModuleError::Runtime("Context 池已满".into()))
    }

    fn execute(
        &mut self,
        url: &str,
        _timeout_secs: u64,
        headless: bool,
        chrome_path: &Option<String>,
    ) -> Result<String, ModuleError> {
        let (idx, browser) = self.get_or_create(chrome_path, headless)?;
        let tab = browser
            .new_tab()
            .map_err(|e| ModuleError::Runtime(format!("tab: {e}")))?;

        let entry = &mut self.contexts[idx];
        entry.tabs_used += 1;
        entry.last_used = Instant::now();
        entry.search_count += 1;

        // ⚡ 页面加载前注入全套拟人化脚本
        let _ = tab.evaluate(stealth::STEALTH_JS, false);
        let _ = tab.evaluate(behavior::BEHAVIOR_JS, false);
        let _ = tab.evaluate(captcha::CAPTCHA_DETECT_JS, false);

        let result = (|| -> Result<String, ModuleError> {
            // 📍 导航前：模拟"手动输入URL"前的鼠标移动（不是瞬间到达页面）
            tab.navigate_to("about:blank").ok();
            let _ = tab.evaluate(behavior::BEHAVIOR_JS, false);

            // 实际导航
            tab.navigate_to(url)
                .map_err(|e| ModuleError::Runtime(format!("nav: {e}")))?;
            tab.wait_until_navigated()
                .map_err(|e| ModuleError::Runtime(format!("load: {e}")))?;

            // 👁️ 模拟阅读：等待 + 自然滚动（behavior.js 自动处理）
            std::thread::sleep(Duration::from_millis(800 + rand::random::<u64>() % 1200)); // 0.8-2s

            // 🔍 自动检测+点击验证码（拟人化）
            let _ = tab.evaluate(captcha::CAPTCHA_HUMAN_CLICK_JS, false);

            // 📄 内容提取
            let remote = tab.evaluate(EXTRACT_JS, false);
            match remote {
                Ok(result) => {
                    if let Some(ref val) = result.value {
                        if let Some(s) = val.as_str() {
                            return Ok(s.to_string());
                        }
                    }
                    Ok(String::new())
                }
                Err(_) => {
                    // 降级：直接读 HTML 去标签
                    tab.get_content()
                        .map(|h| strip_html(&h).chars().take(8000).collect())
                        .map_err(|e| ModuleError::Runtime(format!("read: {e}")))
                }
            }
        })();

        // 借鉴 Chromium — Context 级别内存管理
        if entry.search_count >= self.recycle_interval {
            self.contexts.remove(idx);
        }
        self.reap_idle();
        result
    }

    fn reap_idle(&mut self) {
        let now = Instant::now();
        self.contexts
            .retain(|e| now.duration_since(e.last_used) <= self.idle_destroy);
    }
}

// ── 借鉴 Chromium LaunchOptions ──

fn launch_browser(
    chrome_path: &Option<String>,
    headless: bool,
    fp: &BrowserFingerprint,
) -> Result<Arc<Browser>, ModuleError> {
    let ua = OsString::from(format!("--user-agent={}", fp.user_agent));
    let size = OsString::from(format!(
        "--window-size={},{}",
        fp.viewport_width, fp.viewport_height
    ));
    let lang = OsString::from(format!("--lang={}", fp.locale));
    let platform = OsString::from(format!("--platform={}", fp.platform));
    let no_automation = OsString::from("--disable-blink-features=AutomationControlled");
    let no_iso = OsString::from("--disable-features=IsolateOrigins,site-per-process");
    let no_gpu_sandbox = OsString::from("--disable-gpu-sandbox");
    let no_first_run = OsString::from("--no-first-run");
    let no_default_check = OsString::from("--no-default-browser-check");
    let has_gpu = std::env::var("BROWSER_ENABLE_GPU").is_ok();
    let enable_gpu = OsString::from("--enable-gpu");

    let mut args: Vec<&OsStr> = vec![
        &ua,
        &size,
        &lang,
        &platform,
        &no_automation,
        &no_iso,
        &no_gpu_sandbox,
        &no_first_run,
        &no_default_check,
    ];
    if has_gpu {
        args.push(&enable_gpu);
    }

    let chrome_path_arg = chrome_path.as_ref().map(|p| p.clone().into());
    let opts = LaunchOptions {
        headless,
        sandbox: false,
        args,
        path: chrome_path_arg,
        ..Default::default()
    };

    Browser::new(opts)
        .map(Arc::new)
        .map_err(|e| ModuleError::Runtime(format!("browser: {e}")))
}

fn strip_html(html: &str) -> String {
    let mut r = String::new();
    let mut t = false;
    for c in html.chars() {
        if c == '<' {
            t = true;
            continue;
        }
        if c == '>' {
            t = false;
            continue;
        }
        if !t {
            r.push(c);
        }
    }
    r.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── BugModule ──

#[async_trait]
impl BugModule for BrowserModule {
    fn id(&self) -> &str {
        "browser"
    }
    fn name(&self) -> &str {
        "浏览器模块"
    }
    fn version(&self) -> &str {
        "0.3.0"
    }
    fn description(&self) -> &str {
        "Chromium 双池 — 借鉴 BrowserContext 隔离 + --dump-dom JS注入 + 反检测"
    }
    async fn on_install(&self) -> Result<(), ModuleError> {
        // 自动下载 Chromium（首次使用）
        match self.get_chrome_path() {
            Ok(Some(p)) => {
                println!("  ✅ Chromium 就绪: {p}");
                Ok(())
            }
            Ok(None) => Err(ModuleError::InstallFailed("Chromium 路径为空".into())),
            Err(e) => Err(e),
        }
    }
    async fn on_enable(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn on_disable(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn on_uninstall(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn run(&self) -> Result<ModuleRunHandle, ModuleError> {
        let (tx, _) = tokio::sync::oneshot::channel();
        Ok(ModuleRunHandle { abort: tx })
    }
    fn capabilities(&self) -> Vec<ModuleCapability> {
        vec![
            ModuleCapability::WebSearch,
            ModuleCapability::Tool {
                name: "browser".into(),
            },
        ]
    }
    fn permissions(&self) -> Vec<ModulePermission> {
        vec![ModulePermission::NetworkOutbound]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strip_html_works() {
        assert!(strip_html("<p>Hello</p>").contains("Hello"));
    }
    #[test]
    fn fingerprint_varies() {
        let a = random_fingerprint();
        // 随机性测试：生成多个确保覆盖
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            seen.insert(random_fingerprint().user_agent);
        }
        assert!(seen.len() > 1, "UA未随机化");
    }
    #[test]
    fn pool_max_size() {
        assert_eq!(ContextPool::new(4, 8).max_size, 4);
    }
}
