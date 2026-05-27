// 阿里通义千问 — 原生 DashScope 兼容 OpenAI 格式
use crate::openai::OpenAiProvider;
use async_trait::async_trait;
use bugs_core::models::*;

pub struct AlibabaProvider(OpenAiProvider);

impl AlibabaProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self(OpenAiProvider::new(
            "alibaba",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            api_key,
        ))
    }
}

#[async_trait]
impl LlmProvider for AlibabaProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> {
        self.0.chat(req).await
    }
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> {
        self.0.chat_stream(req).await
    }
    fn name(&self) -> &str {
        self.0.name()
    }
    fn supports(&self, m: &str) -> bool {
        m.starts_with("qwen-") || m.starts_with("qwq-")
    }
    fn api_type(&self) -> ApiType {
        ApiType::OpenAiCompat
    }
}
