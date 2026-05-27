mod app;
mod i18n;
mod ui;

use app::{App, MsgRole};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use serde::{Deserialize, Serialize};
use std::io;

// ── API 通信（TUI不直接依赖核心，全部通过HTTP调用） ──

const API_BASE: &str = "http://127.0.0.1:8742";

#[derive(Serialize)] struct ApiChatReq { model: String, messages: Vec<ApiMsg> }
#[derive(Serialize, Clone)] struct ApiMsg { role: String, content: String }
#[derive(Deserialize)] struct ApiChatResp { content: String, error: Option<String> }

fn var(k: &str) -> Result<String, std::env::VarError> { std::env::var(k) }

async fn api_chat(model: &str, messages: &[ApiMsg]) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{API_BASE}/api/chat"))
        .json(&ApiChatReq { model: model.into(), messages: messages.to_vec() })
        .send().await.map_err(|e| format!("连接失败: {e}"))?;
    let body: ApiChatResp = resp.json().await.map_err(|e| format!("{e}"))?;
    if let Some(e) = body.error { Err(e) } else { Ok(body.content) }
}

async fn api_health() -> bool {
    reqwest::Client::new().get(format!("{API_BASE}/api/health")).send().await.is_ok()
}

fn auto_register_from_env() -> (String, Vec<String>) {
    let model = var("BUGS_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    let mut providers = Vec::new();
    if var("OPENAI_API_KEY").is_ok() { providers.push("openai".into()); }
    if var("ANTHROPIC_API_KEY").is_ok() { providers.push("anthropic".into()); }
    if var("DEEPSEEK_API_KEY").is_ok() { providers.push("deepseek".into()); }
    if var("OLLAMA_BASE_URL").is_ok() || true { providers.push("ollama".into()); }
    (model, providers)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new();
    let (model, providers) = auto_register_from_env();

    // 检查 daemon 健康状态
    let daemon_alive = api_health().await;
    if daemon_alive {
        app.add_msg(MsgRole::System, format!("✅ {}: {model} | Providers: {}", app.i18n.model_label(), providers.join(",")));
    } else {
        app.add_msg(MsgRole::System, format!("⚠️ Daemon未运行。直接模式。模型: {model}"));
        // fallback: 如果 daemon 不可用，直接使用 bugs-core
    }

    let mut messages: Vec<ApiMsg> = Vec::new();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Ok(ev) = event::read() {
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Enter => {
                            let input = std::mem::take(&mut app.input);
                            app.cursor = 0;
                            if input == "/exit" { app.add_msg(MsgRole::System, app.i18n.exit_msg().into()); break; }
                            if input == "/clear" { app.clear_chat(); continue; }
                            if input.is_empty() { continue; }
                            if input.starts_with("/scene") || input.starts_with("/memory") || input.starts_with("/status") {
                                // 场景/记忆/状态管理命令（调用API）
                                let cmd_result = handle_command(&input).await;
                                app.add_msg(MsgRole::System, cmd_result);
                                continue;
                            }
                            // 正常对话
                            app.add_msg(MsgRole::User, input.clone());
                            messages.push(ApiMsg { role: "user".into(), content: input });
                            match api_chat(&model, &messages).await {
                                Ok(content) => {
                                    app.add_msg(MsgRole::Assistant, content.clone());
                                    messages.push(ApiMsg { role: "assistant".into(), content });
                                }
                                Err(e) => app.add_msg(MsgRole::System, format!("✗ {e}")),
                            }
                        }
                        KeyCode::Char(c) => { app.input.insert(app.cursor, c); app.cursor += 1; }
                        KeyCode::Backspace => { if app.cursor > 0 { app.input.remove(app.cursor - 1); app.cursor -= 1; } }
                        KeyCode::Left => { app.cursor = app.cursor.saturating_sub(1); }
                        KeyCode::Right => { app.cursor = (app.cursor + 1).min(app.input.len()); }
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::PageUp => { for _ in 0..5 { app.scroll_up(); } }
                        KeyCode::PageDown => { for _ in 0..5 { app.scroll_down(); } }
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        tokio::task::yield_now().await;
    }

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

async fn handle_command(input: &str) -> String {
    let client = reqwest::Client::new();
    if input == "/scene" || input == "/scenes" {
        match client.get(format!("{API_BASE}/api/scenes")).send().await {
            Ok(resp) => resp.text().await.unwrap_or_else(|_| "场景列表获取失败".into()),
            Err(_) => "⚠️ API 不可用，请启动 daemon".into(),
        }
    } else if input == "/memory" || input.starts_with("/mem") {
        match client.get(format!("{API_BASE}/api/memory/search?q=*")).send().await {
            Ok(resp) => resp.text().await.unwrap_or_else(|_| "记忆检索失败".into()),
            Err(_) => "⚠️ API 不可用".into(),
        }
    } else if input == "/status" {
        match client.get(format!("{API_BASE}/api/status")).send().await {
            Ok(resp) => resp.text().await.unwrap_or_else(|_| "状态获取失败".into()),
            Err(_) => "⚠️ API 不可用".into(),
        }
    } else if input == "/config" {
        match client.get(format!("{API_BASE}/api/config")).send().await {
            Ok(resp) => resp.text().await.unwrap_or_else(|_| "配置获取失败".into()),
            Err(_) => "⚠️ API 不可用".into(),
        }
    } else {
        format!("未知命令: {input}")
    }
}
pub mod tests;
