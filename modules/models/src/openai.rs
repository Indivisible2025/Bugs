use async_trait::async_trait;
use bugs_core::models::*;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct OpenAiProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
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

    fn build_body(&self, req: &ChatRequest) -> serde_json::Value {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role { Role::System=>"system", Role::User=>"user", Role::Assistant=>"assistant" },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });
        body
    }

    fn extract_content(data: &serde_json::Value) -> Option<String> {
        data["choices"][0]["delta"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                data["choices"][0]["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
            })
    }

    fn extract_reasoning(data: &serde_json::Value) -> Option<String> {
        data["choices"][0]["delta"]["reasoning_content"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                data["choices"][0]["message"]["reasoning_content"]
                    .as_str()
                    .map(|s| s.to_string())
            })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
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
            content: Self::extract_content(&data).unwrap_or_default(),
            reasoning_content: Self::extract_reasoning(&data),
            finish_reason: data["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
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
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            let line = line.trim();
                            if line.is_empty() || line == "data: [DONE]" {
                                continue;
                            }
                            if let Some(json_str) = line.strip_prefix("data: ") {
                                if let Ok(data) =
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                {
                                    let content = Self::extract_content(&data).unwrap_or_default();
                                    let reasoning =
                                        Self::extract_reasoning(&data).unwrap_or_default();
                                    let finish = data["choices"][0]["finish_reason"]
                                        .as_str()
                                        .map(|s| s.to_string());
                                    let combined = if !reasoning.is_empty() {
                                        format!("[思考]\n{}\n[回答]\n{}", reasoning, content)
                                    } else {
                                        content
                                    };
                                    let _ = tx.send(Ok(StreamChunk {
                                        content: combined,
                                        finish_reason: finish,
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
    fn supports(&self, _model: &str) -> bool {
        true
    }
    fn api_type(&self) -> ApiType {
        ApiType::OpenAiCompat
    }
}
