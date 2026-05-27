use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Provider 能力 trait ──
// 核心只定义接口，不认识任何具体 Provider
// Provider 实现位于 modules/models/

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ModelError>;
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, ModelError>;
    fn name(&self) -> &str;
    fn supports(&self, model: &str) -> bool;
    fn api_type(&self) -> ApiType;
}

/// 流式响应——异步 token 迭代器
pub type ChatStream = Box<dyn tokio_stream::Stream<Item = Result<StreamChunk, ModelError>> + Send + Unpin>;

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
}

// ── API 类型 ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiType {
    OpenAi,       // /v1/chat/completions, Bearer
    Anthropic,    // /v1/messages, x-api-key
    OpenAiCompat, // OpenAI 兼容但不标准的 base_url
}

// ── 数据模型 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    /// DeepSeek thinking mode
    pub thinking: Option<ThinkingMode>,
    /// DeepSeek reasoning effort
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingMode {
    #[serde(rename = "type")]
    pub mode: String,  // "enabled" | "disabled"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort { High, Max }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub reasoning_content: Option<String>,  // DeepSeek thinking CoT
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(), messages: vec![],
            temperature: Some(0.7), max_tokens: Some(4096),
            system_prompt: None, thinking: None, reasoning_effort: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    #[error("API 错误: {0}")]
    ApiError(String),
    #[error("超时")]
    Timeout,
    #[error("模型不可用: {0}")]
    ModelUnavailable(String),
}

// ── Provider 注册表 ──
// Provider 模块通过注册表接入，核心不 import 任何具体 Provider

pub struct ProviderRegistry {
    providers: Vec<Box<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: vec![] }
    }

    /// 注册一个 Provider（模块启动时调用）
    pub fn register(&mut self, provider: Box<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// 根据模型名找到 Provider
    pub fn find(&self, model: &str) -> Option<&dyn LlmProvider> {
        self.providers
            .iter()
            .find(|p| p.supports(model))
            .map(|p| p.as_ref())
    }

    /// 列出所有已注册的 Provider 名称
    pub fn list(&self) -> Vec<String> {
        self.providers.iter().map(|p| p.name().to_string()).collect()
    }

    /// 是否有任何 Provider
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider;
    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _: ChatRequest) -> Result<ChatResponse, ModelError> {
            Ok(ChatResponse { content: "ok".into(), reasoning_content: None, finish_reason: None, usage: None })
        }
        async fn chat_stream(&self, _: ChatRequest) -> Result<ChatStream, ModelError> {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let _ = tx.send(Ok(StreamChunk { content: "ok".into(), finish_reason: Some("stop".into()) }));
            Ok(Box::new(tokio_stream::wrappers::UnboundedReceiverStream::new(rx)))
        }
        fn name(&self) -> &str { "mock" }
        fn supports(&self, m: &str) -> bool { m == "mock" }
        fn api_type(&self) -> ApiType { ApiType::OpenAi }
    }

    #[tokio::test]
    async fn registry_finds_provider() {
        let mut reg = ProviderRegistry::new();
        reg.register(Box::new(MockProvider));
        assert!(reg.find("mock").is_some());
        assert!(reg.find("unknown").is_none());
    }
}
