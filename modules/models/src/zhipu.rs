// 智谱 GLM — 原生 API 格式
use crate::openai::OpenAiProvider;
use async_trait::async_trait;
use bugs_core::models::*;

pub struct ZhipuProvider(OpenAiProvider);

impl ZhipuProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self(OpenAiProvider::new(
            "zhipu",
            "https://open.bigmodel.cn/api/paas/v4",
            api_key,
        ))
    }
}

#[async_trait]
impl LlmProvider for ZhipuProvider {
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
        m.starts_with("glm-") || m.starts_with("chatglm")
    }
    fn api_type(&self) -> ApiType {
        ApiType::OpenAiCompat
    }
}
