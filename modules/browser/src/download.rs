//! Chromium 内置模块
//! 来源：Google Chrome for Testing（官方，稳定版本）
//! 中国镜像：npmmirror.com（自动切换）

use bugs_core::module::ModuleError;
use std::path::PathBuf;

pub struct ChromiumBundle {
    cache_dir: PathBuf,
}

/// Google Chrome for Testing — 完整 Chromium（支持 headless + headful 双池）
const CHROMIUM_VERSION: &str = "130.0.6723.31";
const CDN_GOOGLE: &str = "https://storage.googleapis.com/chrome-for-testing-public";
const CDN_MIRROR: &str = "https://registry.npmmirror.com/-/binary/chrome-for-testing"; // 中国加速

impl ChromiumBundle {
    pub fn new() -> Self {
        let home = std::env::var("HOME").ok().map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self { cache_dir: home.join(".bugs/browser/chromium") }
    }

    pub fn is_ready(&self) -> bool { self.executable_path().exists() }

    pub fn executable_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        { self.cache_dir.join("chrome-linux64/chrome") }
        #[cfg(target_os = "macos")]
        { self.cache_dir.join("chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing") }
        #[cfg(target_os = "windows")]
        { self.cache_dir.join("chrome-win64/chrome.exe") }
    }

    /// 模块安装时调用——从 Google 官方下载完整 Chromium（支持无头+有头双池）
    pub fn bundle(&self) -> Result<PathBuf, ModuleError> {
        if self.is_ready() {
            println!("  ✅ Chromium 已内置（双池可用）: {}", self.executable_path().display());
            return Ok(self.executable_path());
        }

        let (platform, zip_name) = self.platform_info();
        let urls = [
            format!("{CDN_GOOGLE}/{CHROMIUM_VERSION}/{platform}/{zip_name}"),
            format!("{CDN_MIRROR}/{CHROMIUM_VERSION}/{platform}/{zip_name}"),
        ];

        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| ModuleError::InstallFailed(format!("缓存目录: {e}")))?;

        let mut last_err = String::new();
        for (i, url) in urls.iter().enumerate() {
            let source = if i == 0 { "Google" } else { "中国镜像" };
            println!("  📦 正在内置 Chromium (~130MB, Headless+Headful双池) [来源: {source}]...");
            match self.download_and_extract(url, &zip_name) {
                Ok(path) => return Ok(path),
                Err(e) => last_err = format!("{e}"),
            }
        }

        Err(ModuleError::InstallFailed(format!("下载失败。\n最后错误: {last_err}\n手动下载: {CDN_GOOGLE}/{CHROMIUM_VERSION}/{platform}/{zip_name}")))
    }

    fn platform_info(&self) -> (&str, &str) {
        #[cfg(target_os = "linux")]   { ("linux64", "chrome-linux64.zip") }
        #[cfg(target_os = "macos")]   { ("mac-x64", "chrome-mac-x64.zip") }
        #[cfg(target_os = "windows")] { ("win64", "chrome-win64.zip") }
    }

    fn download_and_extract(&self, url: &str, zip_name: &str) -> Result<PathBuf, ModuleError> {
        let zip_path = self.cache_dir.join(zip_name);
        let resp = reqwest::blocking::get(url).map_err(|e| ModuleError::InstallFailed(format!("连接失败: {e}")))?;
        if !resp.status().is_success() { return Err(ModuleError::InstallFailed(format!("HTTP {}", resp.status()))); }
        let bytes = resp.bytes().map_err(|e| ModuleError::InstallFailed(format!("读取: {e}")))?;
        println!("  ✓ 下载完成: {} MB", bytes.len() / 1024 / 1024);
        std::fs::write(&zip_path, &bytes).map_err(|e| ModuleError::InstallFailed(format!("写入: {e}")))?;
        let file = std::fs::File::open(&zip_path).map_err(|e| ModuleError::InstallFailed(format!("打开: {e}")))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| ModuleError::InstallFailed(format!("解压: {e}")))?;
        archive.extract(&self.cache_dir).map_err(|e| ModuleError::InstallFailed(format!("解压: {e}")))?;
        let _ = std::fs::remove_file(&zip_path);
        let path = self.executable_path();
        if path.exists() {
            #[cfg(target_os = "linux")] {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&path) {
                    let mut perm = meta.permissions();
                    perm.set_mode(0o755);
                    let _ = std::fs::set_permissions(&path, perm);
                }
            }
            Ok(path)
        } else {
            Err(ModuleError::InstallFailed("解压后未找到可执行文件".into()))
        }
    }
}

