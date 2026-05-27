#![allow(dead_code)]
//! 国际化——跟随系统语言

#[derive(Debug, Clone, Copy)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn detect() -> Self {
        let lang = std::env::var("LANG").unwrap_or_default();
        if lang.starts_with("zh") {
            Lang::Zh
        } else {
            Lang::En
        }
    }
}

pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    pub fn new() -> Self {
        Self {
            lang: Lang::detect(),
        }
    }

    pub fn title(&self) -> &str {
        match self.lang {
            Lang::Zh => "🧠 Overmind",
            Lang::En => "🧠 Overmind",
        }
    }
    pub fn model_label(&self) -> &str {
        match self.lang {
            Lang::Zh => "模型",
            Lang::En => "Model",
        }
    }
    pub fn input_placeholder(&self) -> &str {
        match self.lang {
            Lang::Zh => "输入消息... (Enter发送, /help帮助)",
            Lang::En => "Type a message... (Enter to send)",
        }
    }
    pub fn help_text(&self) -> &str {
        match self.lang {
            Lang::Zh => "/exit退出 /clear清空 /init初始化 /dispatch任务派遣",
            Lang::En => "/exit /clear /init /dispatch",
        }
    }
    pub fn exit_msg(&self) -> &str {
        match self.lang {
            Lang::Zh => "会话结束",
            Lang::En => "Session ended",
        }
    }
    pub fn loading(&self) -> &str {
        match self.lang {
            Lang::Zh => "思考中...",
            Lang::En => "Thinking...",
        }
    }
    pub fn init_ok(&self, path: &str) -> String {
        match self.lang {
            Lang::Zh => format!("✅ 配置已创建: {path}"),
            Lang::En => format!("✅ Config created: {path}"),
        }
    }
    pub fn dispatch_header(&self, count: usize) -> String {
        match self.lang {
            Lang::Zh => format!("🧠 派遣 {} 个子Agent", count),
            Lang::En => format!("🧠 Dispatching {} sub-agents", count),
        }
    }
    pub fn no_provider(&self) -> &str {
        match self.lang {
            Lang::Zh => "请设置 API Key（如 DEEPSEEK_API_KEY）",
            Lang::En => "Please set an API Key (e.g. DEEPSEEK_API_KEY)",
        }
    }
}
