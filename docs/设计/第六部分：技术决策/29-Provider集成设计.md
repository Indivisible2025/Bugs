# LLM Provider 集成设计

## Provider Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, req: ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;
    fn supports_model(&self, model: &ModelId) -> bool;
    fn provider_name(&self) -> &str;
    fn base_url(&self) -> &str;
}

pub struct ChatRequest {
    pub model: ModelId,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
}

pub struct ChatResponse {
    pub content: String,
    pub usage: Usage,
    pub finish_reason: FinishReason,
}
```

## Provider 实现

| Provider | 后端 | 特点 |
|:---------|:----|:----|
| `OpenAiProvider` | /v1/chat/completions | 流式，原生 tools |
| `anthropicProvider` | /v1/messages | 扩展上下文，tool_use |
| `OllamaProvider` | /api/chat | 本地，支持自定义模型 |
| `OpenRouterProvider` | /v1/chat/completions | 多模型聚合访问 |

## 模型路由

```
主Agent 对话 → 走主模型（如 claude-sonnet-4）
子Agent 任务 → 按 subagent_defaults.models 路由：
    "reasoning" → claude-sonnet-4
    "code"      → qwen2.5-coder:7b
    "fast"      → gpt-4o-mini
    "tiny"      → llama3.2:3b
```

## Provider 池

```rust
pub struct ProviderPool {
    providers: HashMap<ProviderId, Arc<dyn LlmProvider>>,
    model_map: HashMap<ModelId, ProviderId>,    // 模型→提供商 路由
    rate_limiters: HashMap<ProviderId, RateLimiter>,
    retry_config: RetryConfig,
}
```

## 并发与限流

| Provider | 默认并发上限 | 说明 |
|:---------|:--------:|:----|
| OpenAI | 500/min | API 限频防御 |
| anthropic 限频防御 |
| Ollama | **无上限** | 本地模型，不限 |
| 自定义 | 可配置 | 用户设 |

## 故障转移

```
Provider A 超时/报错 429
    ↓
自动切换到同模型的备选 Provider
    ↓
备选也失败 → 降级到低一级模型（如 sonnet → haiku）
    ↓
本地 Ollama 始终在最后兜底
```