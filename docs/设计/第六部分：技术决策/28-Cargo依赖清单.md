# Cargo 依赖清单

```toml
[workspace]
members = [
    "crates/bugs-core",
    "crates/bugs-api",
    "crates/bugs-tui",
    "crates/bugs-gui",
    "crates/bugs-web",
    "crates/bugs-cli",
]

# ── bugs-core ──
[dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rmp-serde = "1"                     # msgpack
redb = { version = "2", optional = true }     # KV 存储
hnsw_rs = "0.3"                              # 向量检索
zstd = { version = "0.13", features = ["zstdmt"] }
headless_chrome = "1"                # 浏览器控制
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-tungstenite = "0.24"          # WebSocket
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
rand = "0.8"
parking_lot = "0.12"                # 高效锁
json5 = "0.4"                       # 配置 JSON5
sha2 = "0.10"                       # Token 哈希
tracing = "0.1"                     # 日志
tracing-subscriber = "0.3"

# ── bugs-api ──
[dependencies]
axum = { version = "0.8", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
hyper = "1"

# ── bugs-tui ──
[dependencies]
ratatui = "0.29"
crossterm = "0.28"

# ── bugs-gui ──
[dependencies]
egui = "0.29"
eframe = "0.29"

# ── bugs-web ──
[dependencies]
# 复用 bugs-api 的 axum
rust-embed = "8"                     # 嵌入静态资源

# ── bugs-cli ──
[dependencies]
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"                   # 进度条
console = "0.15"
```