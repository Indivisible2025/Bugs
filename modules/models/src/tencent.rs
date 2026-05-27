// 腾讯混元 — 原生 API 格式
use crate::openai::OpenAiProvider; use async_trait::async_trait;
use bugs_core::models::*;
pub struct TencentProvider(OpenAiProvider);
impl TencentProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self(OpenAiProvider::new("tencent", "https://api.hunyuan.cloud.tencent.com/v1", api_key))
    }
}
#[async_trait] impl LlmProvider for TencentProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError> { self.0.chat(req).await }
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError> { self.0.chat_stream(req).await }
    fn name(&self) -> &str { self.0.name() }
    fn supports(&self, m: &str) -> bool { m.starts_with("hunyuan-") }
    fn api_type(&self) -> ApiType { ApiType::OpenAiCompat }
}
