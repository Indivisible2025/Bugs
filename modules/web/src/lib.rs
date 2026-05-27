use std::sync::Arc;
use axum::{Router, routing::{get, post}, response::{Html, IntoResponse, Response}, body::Body, extract::Path};

const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");

pub async fn start(port: u16, daemon_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(reqwest::Client::new());
    let api = format!("http://127.0.0.1:{daemon_port}/api");
    let state = (client, api);

    let app = Router::new()
        .route("/", get(root))
        .route("/static/style.css", get(css))
        .route("/static/app.js", get(js))
        .route("/api/{*path}", get(api_get))
        .route("/api/{*path}", post(api_post))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    println!("🌐 WebUI: http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> impl IntoResponse { Html(INDEX_HTML) }
async fn css() -> impl IntoResponse { ([(axum::http::header::CONTENT_TYPE, "text/css;charset=utf-8")], STYLE_CSS) }
async fn js() -> impl IntoResponse { ([(axum::http::header::CONTENT_TYPE, "application/javascript;charset=utf-8")], APP_JS) }

type AppState = (Arc<reqwest::Client>, String);

async fn api_get(Path(path): Path<String>, axum::extract::State(state): axum::extract::State<AppState>) -> Response<Body> {
    let (client, prefix) = &state;
    match client.get(format!("{prefix}/{path}")).send().await {
        Ok(r) => Response::builder().status(r.status()).body(Body::from(r.text().await.unwrap_or_default())).unwrap(),
        Err(_) => Response::builder().status(502).body(Body::from("{\"error\":\"unreachable\"}")).unwrap(),
    }
}

async fn api_post(Path(path): Path<String>, axum::extract::State(state): axum::extract::State<AppState>, body: String) -> Response<Body> {
    let (client, prefix) = &state;
    match client.post(format!("{prefix}/{path}")).header("content-type", "application/json").body(body).send().await {
        Ok(r) => Response::builder().status(r.status()).body(Body::from(r.text().await.unwrap_or_default())).unwrap(),
        Err(_) => Response::builder().status(502).body(Body::from("{\"error\":\"unreachable\"}")).unwrap(),
    }
}
