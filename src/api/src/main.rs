//! Bugs API Daemon — 所有前端的统一后端

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bugs_core::hotreload::ConfigWatcher;
use bugs_core::meditate::Meditation;
use bugs_core::memory::MemoryMesh;
use bugs_core::models::{ChatRequest, Message, ProviderRegistry, Role};
use bugs_core::scene::SceneManager;
use bugs_core::scheduler::cron::CronScheduler;
use bugs_core::skill::SkillRegistry;
use bugs_core::trust::TrustEngine;
use bugs_core::types::MemoryConfig;
use bugs_models::*;
use serde::Deserialize;
use std::sync::Arc;

struct AppState {
    registry: Arc<ProviderRegistry>,
    trust: Arc<TrustEngine>,
    meditation: Arc<Meditation>,
    memory: Arc<MemoryMesh>,
    scenes: Arc<SceneManager>,
    skills: parking_lot::RwLock<SkillRegistry>,
    watcher: parking_lot::Mutex<ConfigWatcher>,
}

#[tokio::main]
async fn main() {
    let mut reg = ProviderRegistry::new();
    auto_register(&mut reg);
    let mut skills = SkillRegistry::new();
    let core_skills = std::path::Path::new("core-skills");
    if core_skills.exists() {
        skills.load_from_dir(core_skills);
    }
    let home_skills = std::path::Path::new("/home/nianyv/.bugs/skills");
    if home_skills.exists() {
        skills.load_from_dir(home_skills);
    }

    let state = Arc::new(AppState {
        registry: Arc::new(reg),
        trust: Arc::new(TrustEngine::default()),
        meditation: Arc::new(Meditation::default()),
        memory: Arc::new(MemoryMesh::new(MemoryConfig::default())),
        scenes: Arc::new(SceneManager::new(200, "general".into())),
        skills: parking_lot::RwLock::new(skills),
        watcher: parking_lot::Mutex::new(ConfigWatcher::new(
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".bugs/config.json"))
                .unwrap_or_else(|_| std::path::PathBuf::from("config.json")),
        )),
    });

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/status", get(status))
        .route("/api/trust", get(trust_status))
        .route("/api/browser", get(browser_status))
        .route("/api/tools", get(tools))
        .route("/api/scenes", get(scenes))
        .route("/api/scenes/current", get(current_scene))
        .route("/api/scenes/switch", post(switch_scene))
        .route("/api/memory/search", get(memory_search))
        .route("/api/config", get(config))
        .route("/api/meditate", post(trigger_meditate))
        .route("/api/meditate/status", get(meditate_status))
        .route("/api/meditate/pending", get(meditate_pending))
        .route("/api/meditate/confirm", post(meditate_confirm))
        .route("/api/skills", get(skills_list))
        .with_state(state.clone());

    tokio::spawn(cron_worker(state));

    let port = env_port().unwrap_or(8742);
    println!(
        "🧠 Bugs Daemon {external} (internal {internal}) — http://127.0.0.1:{port}",
        external = VERSION_EXTERNAL,
        internal = VERSION_INTERNAL
    );
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .expect("daemon init");
    axum::serve(listener, app).await.expect("daemon init");
}

async fn cron_worker(state: Arc<AppState>) {
    let mut cron = CronScheduler::new();
    cron.add("记忆整理", 3600, || println!("🧠 Cron: 触发冥想"));
    loop {
        // 配置热重载
        if state.watcher.lock().has_changed() {
            println!("🧠 配置已变更，正在重载...");
        }
        for _ in cron.tick() {}
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

async fn skills_list(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let list = s.skills.read();
    Json(serde_json::json!({
        "skills": list.list().iter().map(|sk| serde_json::json!({
            "name": sk.name, "description": sk.description,
        })).collect::<Vec<_>>(),
        "count": list.list().len(),
    }))
}

const VERSION_EXTERNAL: &str = "2026.05.27-dev-1";
const VERSION_INTERNAL: &str = "1-dev-0-1";

fn auto_register(reg: &mut ProviderRegistry) {
    if let Ok(k) = env("OPENAI_API_KEY") {
        reg.register(Box::new(OpenAiProvider::new(
            "openai",
            b("OPENAI_BASE_URL", "https://api.openai.com/v1"),
            k,
        )));
    }
    if let Ok(k) = env("ANTHROPIC_API_KEY") {
        reg.register(Box::new(AnthropicProvider::new(
            "anthropic",
            b("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
            k,
        )));
    }
    if let Ok(k) = env("DEEPSEEK_API_KEY") {
        reg.register(Box::new(DeepSeekProvider::new_openai(k.clone())));
        reg.register(Box::new(DeepSeekProvider::new_anthropic(k)));
    }
    if let Ok(k) = env("GROQ_API_KEY") {
        reg.register(Box::new(OpenAiProvider::new(
            "groq",
            "https://api.groq.com/openai/v1",
            k,
        )));
    }
    if let Ok(k) = env("MOONSHOT_API_KEY") {
        reg.register(Box::new(MoonshotProvider::new(k)));
    }
    if let Ok(k) = env("ZHIPU_API_KEY") {
        reg.register(Box::new(ZhipuProvider::new(k)));
    }
    let k = env("OLLAMA_API_KEY").unwrap_or_default();
    reg.register(Box::new(OpenAiProvider::new(
        "ollama",
        "http://localhost:11434/v1",
        k,
    )));
}

fn env_port() -> Option<u16> {
    std::env::var("BUGS_PORT").ok().and_then(|p| p.parse().ok())
}

fn env(k: &str) -> Result<String, std::env::VarError> {
    std::env::var(k)
}
fn b(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.into())
}

// ── API 端点 ──

// ... (endpoints unchanged)
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok","version":VERSION_EXTERNAL}))
}

async fn status(State(_s): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok","providers":_s.registry.list(),"uptime":"running"}))
}

async fn trust_status(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({"trust":"active","config":{
        "initial_enhancement":s.trust.config.initial_enhancement,
        "initial_decay":s.trust.config.initial_decay,
        "cross_validate_k":s.trust.config.cross_validate_k,
        "user_confirm_delta":s.trust.config.user_confirm_delta,
        "disprove_penalty":s.trust.config.disprove_penalty,
    }}))
}

async fn browser_status() -> impl IntoResponse {
    Json(serde_json::json!({"browser":"not_connected","search_engines":["bing","google","baidu"]}))
}

async fn tools() -> impl IntoResponse {
    Json(serde_json::json!({"tools":[
        {"name":"read","group":"fs","required":true},
        {"name":"write","group":"fs","required":true},
        {"name":"exec","group":"runtime","required":true},
        {"name":"browser","group":"browser","required":false,"enabled":false},
    ]}))
}

async fn scenes(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({"scenes":s.scenes.list(),"current":s.scenes.current_name()}))
}

async fn current_scene(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({"current":s.scenes.current_name(),"current_id":s.scenes.current()}))
}

#[derive(Deserialize)]
struct SwitchBody {
    name: String,
}

async fn switch_scene(
    State(s): State<Arc<AppState>>,
    Json(body): Json<SwitchBody>,
) -> impl IntoResponse {
    match s.scenes.switch_by_name(&body.name, &s.memory) {
        Some(id) => Json(serde_json::json!({"switched":id,"name":body.name})),
        None => Json(serde_json::json!({"error":format!("场景不存在: {}",body.name)})),
    }
}

async fn memory_search() -> impl IntoResponse {
    Json(serde_json::json!({"memories":[]}))
}

async fn config() -> impl IntoResponse {
    let cfg = bugs_core::types::BugsConfig::default();
    Json(serde_json::json!({
        "scheduler":{"global_max":cfg.scheduler.global_max_parallelism},
        "memory":{"global_max_mb":cfg.memory.global_max_mb,"scene_full_max_mb":cfg.memory.scene_full_max_mb},
        "network":{"local_port":cfg.network.local.port},
        "scenes":{"default":cfg.scenes.default_scene}
    }))
}

async fn trigger_meditate(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let report = s.meditation.meditate(&s.memory, &s.trust);
    Json(serde_json::json!({
        "duration_ms":report.duration_ms,"merged":report.merged,
        "strengthened":report.strengthened,"cleaned":report.cleaned,
        "extracted":report.extracted,"discoveries":report.discoveries,
        "pending":report.pending.len(),"details":report.details,
    }))
}

async fn meditate_status() -> impl IntoResponse {
    Json(serde_json::json!({"available":true,"trigger":"POST /api/meditate"}))
}

async fn meditate_pending(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let report = s.meditation.meditate(&s.memory, &s.trust);
    Json(
        serde_json::json!({"pending":report.pending.iter().map(|m|serde_json::json!({
        "category":format!("{:?}",m.category),"strength":m.strength_cached,"scene_id":m.scene_id,
    })).collect::<Vec<_>>(),"count":report.pending.len()}),
    )
}

#[derive(Deserialize)]
struct ConfirmBody {
    index: usize,
    confirmed: bool,
}

async fn meditate_confirm(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ConfirmBody>,
) -> impl IntoResponse {
    let mut all = s.meditation.collect_all(&s.memory);
    if body.index < all.len() {
        if body.confirmed {
            s.trust.user_confirm(&mut all[body.index], 0);
        } else {
            s.trust.disprove(&mut all[body.index]);
        }
        s.meditation.write_back(&s.memory, all);
    }
    Json(serde_json::json!({"status":"ok"}))
}

#[derive(Deserialize)]
struct ChatBody {
    model: Option<String>,
    messages: Vec<ApiMsg>,
}
#[derive(Deserialize, Clone)]
struct ApiMsg {
    role: String,
    content: String,
}

async fn chat(State(s): State<Arc<AppState>>, Json(body): Json<ChatBody>) -> impl IntoResponse {
    let model = body.model.unwrap_or_else(|| "gpt-4o-mini".into());
    let provider = match s.registry.find(&model) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({"error":format!("找不到模型: {model}")}))
                .into_response()
        }
    };
    let msgs: Vec<Message> = body
        .messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => Role::User,
            },
            content: m.content.clone(),
        })
        .collect();
    match provider
        .chat(ChatRequest {
            model,
            messages: msgs,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            ..Default::default()
        })
        .await
    {
        Ok(resp) => Json(serde_json::json!({"content":resp.content})).into_response(),
        Err(e) => Json(serde_json::json!({"error":e.to_string()})).into_response(),
    }
}
