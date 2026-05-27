use async_trait::async_trait;
use bugs_core::models::*;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// DeepSeek 专属 Provider — 原生支持 thinking mode + reasoning_effort
pub struct DeepSeekProvider {
    name: String,
    base_url: String,
    api_key: String,
    api_format: DeepSeekApiFormat,
    client: reqwest::Client,
}

pub enum DeepSeekApiFormat { OpenAi, Anthropic }

impl DeepSeekProvider {
    pub fn new_openai(api_key: impl Into<String>) -> Self {
        Self { name: "deepseek".into(), base_url: "https://api.deepseek.com".into(), api_key: api_key.into(), api_format: DeepSeekApiFormat::OpenAi, client: reqwest::Client::new() }
    }
    pub fn new_anthropic(api_key: impl Into<String>) -> Self {
        Self { name: "deepseek".into(), base_url: "https://api.deepseek.com/anthropic".into(), api_key: api_key.into(), api_format: DeepSeekApiFormat::Anthropic, client: reqwest::Client::new() }
    }
}

impl DeepSeekProvider {
    fn build_openai_body(&self, req: &ChatRequest) -> serde_json::Value {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role { Role::System=>"system", Role::User=>"user", Role::Assistant=>"assistant" },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "thinking": {"type": "enabled"},
            "reasoning_effort": req.reasoning_effort.unwrap_or(ReasoningEffort::High),
        });
        body
    }

    fn build_anthropic_body(&self, req: &ChatRequest) -> serde_json::Value {
        let system = req.messages.iter().find(|m| m.role == Role::System);
        let messages: Vec<_> = req.messages.iter().filter(|m| m.role != Role::System)
            .map(|m| serde_json::json!({"role": match m.role { Role::User=>"user", Role::Assistant=>"assistant", Role::System=>"user" }, "content": m.content})).collect();
        let mut body = serde_json::json!({"model": req.model, "messages": messages, "max_tokens": req.max_tokens.unwrap_or(4096), "output_config": {"effort": req.reasoning_effort.unwrap_or(ReasoningEffort::High)}});
        if let Some(s) = system { body["system"] = serde_json::json!(s.content); }
        body
    }

    fn extract_content(data: &serde_json::Value) -> Option<String> {
        data["choices"][0]["delta"]["content"].as_str().map(|s| s.to_string())
            .or_else(|| data["choices"][0]["message"]["content"].as_str().map(|s| s.to_string()))
            .or_else(|| data["content"][0]["text"].as_str().map(|s| s.to_string()))
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> {
        let (url, body) = match self.api_format {
            DeepSeekApiFormat::OpenAi => (format!("{}/chat/completions", self.base_url.trim_end_matches('/')), self.build_openai_body(&req)),
            DeepSeekApiFormat::Anthropic => (format!("{}/messages", self.base_url.trim_end_matches('/')), self.build_anthropic_body(&req)),
        };
        let mut req_builder = self.client.post(&url).json(&body);
        req_builder = match self.api_format {
            DeepSeekApiFormat::OpenAi => req_builder.header("Authorization", format!("Bearer {}", self.api_key)),
            DeepSeekApiFormat::Anthropic => req_builder.header("x-api-key", &self.api_key).header("anthropic-version", "2023-06-01"),
        };
        let resp = req_builder.send().await.map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() { return Err(ModelError::ApiError(format!("HTTP {}", resp.status()))); }
        let data: serde_json::Value = resp.json().await.map_err(|e| ModelError::ApiError(e.to_string()))?;
        Ok(ChatResponse { content: Self::extract_content(&data).unwrap_or_default(), reasoning_content: None, finish_reason: None, usage: None })
    }

    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> {
        let (url, body) = match self.api_format {
            DeepSeekApiFormat::OpenAi => {
                let mut b = self.build_openai_body(&req);
                b["stream"] = serde_json::json!(true);
                (format!("{}/chat/completions", self.base_url.trim_end_matches('/')), b)
            }
            DeepSeekApiFormat::Anthropic => {
                let mut b = self.build_anthropic_body(&req);
                b["stream"] = serde_json::json!(true);
                (format!("{}/messages", self.base_url.trim_end_matches('/')), b)
            }
        };
        let req_builder = self.client.post(&url).json(&body);
        let req_builder = match self.api_format {
            DeepSeekApiFormat::OpenAi => req_builder.header("Authorization", format!("Bearer {}", self.api_key)),
            DeepSeekApiFormat::Anthropic => req_builder.header("x-api-key", &self.api_key).header("anthropic-version", "2023-06-01"),
        };
        let resp = req_builder.send().await.map_err(|e| ModelError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() { return Err(ModelError::ApiError(format!("HTTP {}", resp.status()))); }

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
                                    let r = d["choices"][0]["delta"]["reasoning_content"].as_str().unwrap_or("").to_string();
                                    let f = d["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());
                                    let combined = if !r.is_empty() { format!("[思考]\n{}\n[回答]\n{}", r, c) } else { c };
                                    let _ = tx.send(Ok(StreamChunk { content: combined, finish_reason: f }));
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

    fn name(&self) -> &str { &self.name }
    fn supports(&self, m: &str) -> bool { m.starts_with("deepseek-") }
    fn api_type(&self) -> ApiType { match self.api_format { DeepSeekApiFormat::OpenAi => ApiType::OpenAi, DeepSeekApiFormat::Anthropic => ApiType::Anthropic } }
}
