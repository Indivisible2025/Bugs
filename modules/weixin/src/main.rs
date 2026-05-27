//! 微信模块入口

use bugs_core::module::BugModule;
use bugs_weixin::WeixinModule;

#[tokio::main]
async fn main() {
    let mut module = WeixinModule::new();

    println!("🧠 Bugs 微信渠道模块 v{}", module.version());

    // 检查是否已绑定
    if !module.bound {
        if let Err(e) = module.bind().await {
            eprintln!("❌ {e}");
            return;
        }
    }

    // 启动服务
    println!("  🟢 开始接收消息...");
    if let Err(e) = module.serve().await {
        eprintln!("❌ 服务器错误: {e}");
    }
}
