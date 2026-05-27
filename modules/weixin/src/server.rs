//! 微信回调 HTTP 服务器

use super::{WeixinModule, crypto};
use axum::{Router, extract::Query, response::IntoResponse, routing::get};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

pub async fn start(module: &WeixinModule) -> Result<(), Box<dyn std::error::Error>> {
    let port = module.port;
    println!("  📡 微信回调服务器启动: port {port}");
    println!("     回调 URL: http://your-domain:{port}/wechat");
    println!("     Token: {}", if module.token.is_empty() { "未设置" } else { "已设置" });

    let app = Router::new()
        .route("/wechat", get(verify_url));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// 等待扫码绑定——启动临时服务器，等待微信服务器回调
pub async fn wait_for_bind(port: u16) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(parking_lot::Mutex::new(Some(tx)));

    let app = Router::new()
        .route("/wechat/bind", get(move |Query(p): Query<HashMap<String, String>>| {
            let tx = tx.clone();
            async move {
                let token = p.get("token").cloned().unwrap_or_default();
                if !token.is_empty() {
                    if let Some(tx) = tx.lock().take() {
                        let _ = tx.send(token);
                    }
                    "✅ 绑定成功！可以关闭此页面。"
                } else {
                    "❌ 缺少 token 参数"
                }
            }
        }));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await
        .map_err(|e| format!("端口绑定失败: {e}"))?;

    println!("  ⏳ 等待扫码...");

    let _server = axum::serve(listener, app);
    tokio::select! {
        result = rx => {
            result.map_err(|_| "绑定取消".to_string())
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
            Err("扫码超时(120s)".to_string())
        }
    }
}

async fn verify_url(Query(p): Query<HashMap<String, String>>) -> impl IntoResponse {
    let token = std::env::var("WEIXIN_TOKEN").unwrap_or_default();
    let sig = p.get("signature").cloned().unwrap_or_default();
    let ts = p.get("timestamp").cloned().unwrap_or_default();
    let nonce = p.get("nonce").cloned().unwrap_or_default();
    let echo = p.get("echostr").cloned().unwrap_or_default();
    if crypto::verify_signature(&token, &ts, &nonce, &sig) { echo } else { "fail".into() }
}
