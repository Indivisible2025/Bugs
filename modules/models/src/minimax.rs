use async_trait::async_trait;
use bugs_core::models::*;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// MiniMax API Provider — 国际版和国内版都支持 Token/Auth 双认证
pub struct MinimaxProvider {
    name: String,
    base_url: String,
    api_key: String,
    auth_mode: MinimaxAuthMode,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimaxAuthMode {
    Token,
    Auth,
}

impl MinimaxProvider {
    /// 通用构造函数
    pub fn new(name: impl Into<String>, base_url: impl Into<String>, api_key: impl Into<String>, auth_mode: MinimaxAuthMode) -> Self {
        Self { name: name.into(), base_url: base_url.into(), api_key: api_key.into(), auth_mode, client: reqwest::Client::new() }
    }

    /// 国际版 Token 认证
    pub fn new_international_token(api_key: impl Into<String>) -> Self {
        Self {
            name: "minimax-intl".into(),
            base_url: "https://api.minimax.io/v1".into(),
            api_key: api_key.into(),
            auth_mode: MinimaxAuthMode::Token,
            client: reqwest::Client::new(),
        }
    }

    /// 国际版 Auth 认证
    pub fn new_international_auth(api_key: impl Into<String>) -> Self {
        Self {
            name: "minimax-intl".into(),
            base_url: "https://api.minimax.io/v1".into(),
            api_key: api_key.into(),
            auth_mode: MinimaxAuthMode::Auth,
            client: reqwest::Client::new(),
        }
    }

    /// 国内版 Token 认证
    pub fn new_cn_token(api_key: impl Into<String>) -> Self {
        Self {
            name: "minimax-cn".into(),
            base_url: "https://api.minimax.chat/v1".into(),
            api_key: api_key.into(),
            auth_mode: MinimaxAuthMode::Token,
            client: reqwest::Client::new(),
        }
    }

    /// 国内版 Auth 认证
    pub fn new_cn_auth(api_key: impl Into<String>) -> Self {
        Self {
            name: "minimax-cn".into(),
            base_url: "https://api.minimax.chat/v1".into(),
            api_key: api_key.into(),
            auth_mode: MinimaxAuthMode::Auth,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for MinimaxProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });

        let auth_value = match self.auth_mode {
            MinimaxAuthMode::Token => format!("Bearer {}", self.api_key),
            MinimaxAuthMode::Auth => self.api_key.clone(),
        };

        let resp = self.client
            .post(&url)
            .header("Authorization", &auth_value)
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ModelError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| ModelError::ApiError(e.to_string()))?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse { reasoning_content: None,
            content,
            finish_reason: data["choices"][0]["finish_reason"].as_str().map(|s| s.to_string()),
            usage: None,
        })
    }

    fn name(&self) -> &str { &self.name }
    fn supports(&self, model: &str) -> bool {
        model.starts_with("abab") || model.starts_with("minimax") || model.starts_with("MiniMax")
    }
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role { Role::System=>"system", Role::User=>"user", Role::Assistant=>"assistant" },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        let auth_value = match self.auth_mode {
            MinimaxAuthMode::Token => format!("Bearer {}", self.api_key),
            MinimaxAuthMode::Auth => self.api_key.clone(),
        };

        let resp = self.client.post(&url).header("Authorization", &auth_value).json(&body)
            .send().await.map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ModelError::ApiError(format!("HTTP {}", resp.status())));
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut stream = resp.bytes_stream();
        tokio::spawn(async move {
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        for line in String::from_utf8_lossy(&bytes).lines() {
                            let line = line.trim();
                            if line.is_empty() || line == "data: [DONE]" { continue; }
                            if let Some(j) = line.strip_prefix("data: ") {
                                if let Ok(d) = serde_json::from_str::<serde_json::Value>(j) {
                                    let c = d["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                                    let f = d["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());
                                    let _ = tx.send(Ok(StreamChunk { content: c, finish_reason: f }));
                                }
                            }
                        }
                    }
                    Err(e) => { let _ = tx.send(Err(ModelError::ConnectionFailed(e.to_string()))); break; }
                }
            }
        });
        Ok(Box::new(UnboundedReceiverStream::new(rx)))
    }

    fn api_type(&self) -> ApiType { ApiType::OpenAiCompat }
}
