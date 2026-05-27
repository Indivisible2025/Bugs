use async_trait::async_trait;
use bugs_core::models::*;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Moonshot/Kimi 专属 Provider — 原生支持 thinking mode
pub struct MoonshotProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl MoonshotProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            name: "moonshot".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }

    fn build_body(&self, req: &ChatRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role { Role::System=>"system", Role::User=>"user", Role::Assistant=>"assistant" },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });
        // Kimi k2 series has thinking enabled by default, explicitly enable
        if req.model.starts_with("kimi-k2") {
            if let Some(ref t) = req.thinking {
                body["thinking"] = serde_json::json!({"type": t.mode});
            } else {
                body["thinking"] = serde_json::json!({"type": "enabled"});
            }
        }
        body
    }
}

#[async_trait]
impl LlmProvider for MoonshotProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&self.build_body(&req))
            .send()
            .await
            .map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ModelError::ApiError(format!("HTTP {}", resp.status())));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ModelError::ApiError(e.to_string()))?;
        Ok(ChatResponse {
            content: data["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            reasoning_content: None,
            finish_reason: None,
            usage: None,
        })
    }

    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut body = self.build_body(&req);
        body["stream"] = serde_json::json!(true);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
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
                            if line.is_empty() || line == "data: [DONE]" {
                                continue;
                            }
                            if let Some(j) = line.strip_prefix("data: ") {
                                if let Ok(d) = serde_json::from_str::<serde_json::Value>(j) {
                                    let c = d["choices"][0]["delta"]["content"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    let f = d["choices"][0]["finish_reason"]
                                        .as_str()
                                        .map(|s| s.to_string());
                                    let _ = tx.send(Ok(StreamChunk {
                                        content: c,
                                        finish_reason: f,
                                    }));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(ModelError::ConnectionFailed(e.to_string())));
                        break;
                    }
                }
            }
        });
        Ok(Box::new(UnboundedReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        &self.name
    }
    fn supports(&self, m: &str) -> bool {
        m.starts_with("kimi-") || m.starts_with("moonshot-")
    }
    fn api_type(&self) -> ApiType {
        ApiType::OpenAiCompat
    }
}
