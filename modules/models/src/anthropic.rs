use async_trait::async_trait;
use bugs_core::models::*;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct AnthropicProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> {
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));
        let system = req.messages.iter().find(|m| m.role == Role::System);
        let messages: Vec<_> = req
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| serde_json::json!({"role": role_to(m.role), "content": m.content}))
            .collect();

        let mut body = serde_json::json!({"model": req.model, "messages": messages, "max_tokens": req.max_tokens.unwrap_or(4096)});
        if let Some(s) = system {
            body["system"] = serde_json::json!(s.content);
        }
        if let Some(t) = req.temperature {
            body["temperature"] = serde_json::json!(t);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ModelError::ApiError(format!(
                "HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ModelError::ApiError(e.to_string()))?;
        Ok(ChatResponse {
            content: data["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            reasoning_content: None,
            finish_reason: data["stop_reason"].as_str().map(|s| s.to_string()),
            usage: None,
        })
    }

    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> {
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));
        let system = req.messages.iter().find(|m| m.role == Role::System);
        let messages: Vec<_> = req
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| serde_json::json!({"role": role_to(m.role), "content": m.content}))
            .collect();

        let mut body = serde_json::json!({"model": req.model, "messages": messages, "max_tokens": req.max_tokens.unwrap_or(4096), "stream": true});
        if let Some(s) = system {
            body["system"] = serde_json::json!(s.content);
        }
        if let Some(t) = req.temperature {
            body["temperature"] = serde_json::json!(t);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
            let mut buffer = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_str = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();
                            for line in event_str.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data)
                                    {
                                        if val["type"] == "content_block_delta" {
                                            if let Some(text) = val["delta"]["text"].as_str() {
                                                let _ = tx.send(Ok(StreamChunk {
                                                    content: text.to_string(),
                                                    finish_reason: None,
                                                }));
                                            }
                                        }
                                        if val["type"] == "message_stop" {
                                            let _ = tx.send(Ok(StreamChunk {
                                                content: String::new(),
                                                finish_reason: Some("stop".into()),
                                            }));
                                        }
                                    }
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
    fn supports(&self, _model: &str) -> bool {
        true
    }
    fn api_type(&self) -> ApiType {
        ApiType::Anthropic
    }
}

fn role_to(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "user",
    }
}
