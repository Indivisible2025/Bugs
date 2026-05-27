//! 微信个人号渠道模块 — 标准模块格式
//!
//! 支持二维码扫码绑定、消息接收与回复。

use async_trait::async_trait;
use bugs_core::module::{
    BugModule, ModuleCapability, ModuleError, ModulePermission, ModuleRunHandle,
};

pub mod crypto;
pub mod message;
pub mod server;
pub mod qr;

/// 微信模块配置
#[derive(Debug, Clone)]
pub struct WeixinModule {
    /// 微信 Bot Token（扫码后获得）
    pub token: String,
    /// 回调端口
    pub port: u16,
    /// 绑定状态
    pub bound: bool,
}

impl WeixinModule {
    pub fn new() -> Self {
        Self {
            token: std::env::var("WEIXIN_TOKEN").unwrap_or_default(),
            port: std::env::var("WEIXIN_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787),
            bound: !std::env::var("WEIXIN_TOKEN").unwrap_or_default().is_empty(),
        }
    }

    /// 启动绑定流程——显示二维码
    pub async fn bind(&mut self) -> Result<(), ModuleError> {
        if self.bound {
            println!("  ✅ 已绑定");
            return Ok(());
        }

        println!("  📱 正在生成绑定二维码...");
        println!();
        qr::print_bind_qrcode(self.port);
        println!();
        println!("  请使用微信扫描上方二维码完成绑定");
        println!("  等待扫码中...");

        // 启动回调服务器等待扫码结果
        let token = server::wait_for_bind(self.port).await
            .map_err(|e| ModuleError::Runtime(format!("绑定失败: {e}")))?;

        self.token = token;
        self.bound = true;
        println!("  ✅ 微信绑定成功！");
        Ok(())
    }

    /// 启动微信回调服务器
    pub async fn serve(&self) -> Result<(), Box<dyn std::error::Error>> {
        server::start(self).await
    }
}

#[async_trait]
impl BugModule for WeixinModule {
    fn id(&self) -> &str { "weixin" }
    fn name(&self) -> &str { "微信渠道" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "通过微信官方 API，扫码绑定个人微信，接收消息并交由 Overmind 回复" }

    async fn on_install(&self) -> Result<(), ModuleError> {
        println!("  🌐 微信模块 v{}", self.version());
        println!("     → 运行 bugs-weixin 启动扫码绑定");
        Ok(())
    }
    async fn on_enable(&self) -> Result<(), ModuleError> { Ok(()) }
    async fn on_disable(&self) -> Result<(), ModuleError> { Ok(()) }
    async fn on_uninstall(&self) -> Result<(), ModuleError> { Ok(()) }

    async fn run(&self) -> Result<ModuleRunHandle, ModuleError> {
        let (tx, _) = tokio::sync::oneshot::channel();
        println!("  📡 微信模块已启动: port {}", self.port);
        Ok(ModuleRunHandle { abort: tx })
    }

    fn capabilities(&self) -> Vec<ModuleCapability> {
        vec![ModuleCapability::Channel { name: "wechat-personal".into() }]
    }
    fn permissions(&self) -> Vec<ModulePermission> {
        vec![ModulePermission::NetworkOutbound, ModulePermission::AccessMemory]
    }
}
