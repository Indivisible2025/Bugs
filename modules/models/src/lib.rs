#![allow(dead_code)]
#![allow(unused_variables)]
//! 统一 AI 模型提供商模块
//!
//! 每个 Provider 独立 .rs 文件，方便后续优化和新增。

use async_trait::async_trait;
use bugs_core::models::ProviderRegistry;
use bugs_core::module::{
    BugModule, ModuleCapability, ModuleError, ModulePermission, ModuleRunHandle,
};

// ── 子模块：每个 Provider 一个文件 ──

pub mod alibaba;
pub mod anthropic;
pub mod baichuan;
pub mod byteplus;
pub mod cohere;
pub mod deepseek;
pub mod google;
pub mod minimax;
pub mod moonshot;
pub mod openai;
pub mod qianfan;
pub mod stepfun;
pub mod tencent;
pub mod zhipu;

// ── 重新导出 ──

pub use alibaba::AlibabaProvider;
pub use anthropic::AnthropicProvider;
pub use baichuan::BaichuanProvider;
pub use byteplus::ByteplusProvider;
pub use cohere::CohereProvider;
pub use deepseek::DeepSeekProvider;
pub use google::GoogleProvider;
pub use minimax::{MinimaxAuthMode, MinimaxProvider};
pub use moonshot::MoonshotProvider;
pub use openai::OpenAiProvider;
pub use qianfan::QianfanProvider;
pub use stepfun::StepfunProvider;
pub use tencent::TencentProvider;
pub use zhipu::ZhipuProvider;

// ── 模块实现 ──

pub struct ModelsModule {
    pub registry: ProviderRegistry,
}

impl ModelsModule {
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
        }
    }

    /// 从环境变量自动注册所有 Provider
    pub fn auto_register(&mut self) {
        let reg = &mut self.registry;

        // 国际
        if let Ok(k) = var("OPENAI_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "openai",
                base("OPENAI_BASE_URL", "https://api.openai.com/v1"),
                k,
            )));
        }
        if let Ok(k) = var("ANTHROPIC_API_KEY") {
            reg.register(Box::new(anthropic::AnthropicProvider::new(
                "anthropic",
                base("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
                k,
            )));
        }
        if let Ok(k) = var("GOOGLE_API_KEY") {
            reg.register(Box::new(google::GoogleProvider::new(k)));
        }
        if let Ok(k) = var("MISTRAL_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "mistral",
                "https://api.mistral.ai/v1",
                k,
            )));
        }
        if let Ok(k) = var("COHERE_API_KEY") {
            reg.register(Box::new(cohere::CohereProvider::new(k)));
        }
        if let Ok(k) = var("PERPLEXITY_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "perplexity",
                "https://api.perplexity.ai",
                k,
            )));
        }
        if let Ok(k) = var("XAI_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "xai",
                "https://api.x.ai/v1",
                k,
            )));
        }
        if let Ok(k) = var("GROQ_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "groq",
                "https://api.groq.com/openai/v1",
                k,
            )));
        }
        if let Ok(k) = var("TOGETHER_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "together",
                "https://api.together.xyz/v1",
                k,
            )));
        }
        if let Ok(k) = var("FIREWORKS_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "fireworks",
                "https://api.fireworks.ai/inference/v1",
                k,
            )));
        }
        if let Ok(k) = var("CEREBRAS_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "cerebras",
                "https://api.cerebras.ai/v1",
                k,
            )));
        }
        if let Ok(k) = var("DEEPINFRA_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "deepinfra",
                "https://api.deepinfra.com/v1/openai",
                k,
            )));
        }
        if let Ok(k) = var("HUGGINGFACE_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "huggingface",
                "https://api-inference.huggingface.co/v1",
                k,
            )));
        }
        if let Ok(k) = var("NVIDIA_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "nvidia",
                "https://integrate.api.nvidia.com/v1",
                k,
            )));
        }
        if let Ok(k) = var("OPENROUTER_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "openrouter",
                "https://openrouter.ai/api/v1",
                k,
            )));
        }

        // 中国
        if let Ok(k) = var("DEEPSEEK_API_KEY") {
            reg.register(Box::new(deepseek::DeepSeekProvider::new_openai(k.clone())));
            reg.register(Box::new(deepseek::DeepSeekProvider::new_anthropic(k)));
        }
        if let Ok(k) = var("MOONSHOT_API_KEY") {
            reg.register(Box::new(moonshot::MoonshotProvider::new(k)));
        }
        if let Ok(k) = var("ZHIPU_API_KEY") {
            reg.register(Box::new(zhipu::ZhipuProvider::new(k)));
        }
        if let Ok(k) = var("BAICHUAN_API_KEY") {
            reg.register(Box::new(baichuan::BaichuanProvider::new(k)));
        }
        if let Ok(k) = var("STEPFUN_API_KEY") {
            reg.register(Box::new(stepfun::StepfunProvider::new(k)));
        }
        if let Ok(k) = var("QIANFAN_API_KEY") {
            reg.register(Box::new(qianfan::QianfanProvider::new(k)));
        }
        if let Ok(k) = var("ALIBABA_API_KEY") {
            reg.register(Box::new(alibaba::AlibabaProvider::new(k)));
        }
        if let Ok(k) = var("TENCENT_API_KEY") {
            reg.register(Box::new(tencent::TencentProvider::new(k)));
        }
        if let Ok(k) = var("BYTEPLUS_API_KEY") {
            reg.register(Box::new(byteplus::ByteplusProvider::new(k)));
        }
        if let Ok(k) = var("SILICONFLOW_API_KEY") {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "siliconflow",
                "https://api.siliconflow.cn/v1",
                k,
            )));
        }

        // MiniMax
        if let Ok(k) = var("MINIMAX_API_KEY") {
            register_minimax(reg, &k, false);
        }
        if let Ok(k) = var("MINIMAX_CN_API_KEY") {
            register_minimax(reg, &k, true);
        }

        // 本地
        if let Ok(b) = var("OLLAMA_BASE_URL") {
            let k = var("OLLAMA_API_KEY").unwrap_or_default();
            reg.register(Box::new(openai::OpenAiProvider::new("ollama", b, k)));
        } else {
            reg.register(Box::new(openai::OpenAiProvider::new(
                "ollama",
                "http://localhost:11434/v1",
                String::new(),
            )));
        }
        if let Ok(b) = var("LMSTUDIO_BASE_URL") {
            let k = var("LMSTUDIO_API_KEY").unwrap_or_default();
            reg.register(Box::new(openai::OpenAiProvider::new("lmstudio", b, k)));
        }
        if let Ok(b) = var("VLLM_BASE_URL") {
            let k = var("VLLM_API_KEY").unwrap_or_default();
            reg.register(Box::new(openai::OpenAiProvider::new("vllm", b, k)));
        }
    }
}

impl Default for ModelsModule {
    fn default() -> Self {
        Self::new()
    }
}

fn var(key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
}
fn base(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

fn register_minimax(reg: &mut ProviderRegistry, key: &str, is_cn: bool) {
    use minimax::MinimaxAuthMode;
    let mode_key = if is_cn {
        "MINIMAX_CN_AUTH_MODE"
    } else {
        "MINIMAX_AUTH_MODE"
    };
    let default_mode = if is_cn { "auth" } else { "token" };
    let mode = std::env::var(mode_key).unwrap_or_else(|_| default_mode.into());
    let (base, auth_mode) = if mode == "auth" {
        (
            if is_cn {
                "https://api.minimax.chat/v1"
            } else {
                "https://api.minimax.io/v1"
            },
            MinimaxAuthMode::Auth,
        )
    } else {
        (
            if is_cn {
                "https://api.minimax.chat/v1"
            } else {
                "https://api.minimax.io/v1"
            },
            MinimaxAuthMode::Token,
        )
    };
    let name = if is_cn { "minimax-cn" } else { "minimax-intl" };
    reg.register(Box::new(minimax::MinimaxProvider::new(
        name, base, key, auth_mode,
    )));
}

#[async_trait]
impl BugModule for ModelsModule {
    fn id(&self) -> &str {
        "models"
    }
    fn name(&self) -> &str {
        "模型提供商"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "统一 AI 模型提供商 — 30+ Provider"
    }
    async fn on_install(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn on_enable(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn on_disable(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn on_uninstall(&self) -> Result<(), ModuleError> {
        Ok(())
    }
    async fn run(&self) -> Result<ModuleRunHandle, ModuleError> {
        let (tx, _) = tokio::sync::oneshot::channel();
        Ok(ModuleRunHandle { abort: tx })
    }
    fn capabilities(&self) -> Vec<ModuleCapability> {
        vec![ModuleCapability::TextInference]
    }
    fn permissions(&self) -> Vec<ModulePermission> {
        vec![]
    }
}
