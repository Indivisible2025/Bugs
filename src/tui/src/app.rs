#![allow(dead_code)]
use crate::i18n::I18n;
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Chat,
    Scenes,
    Trust,
    Status,
}

pub struct App {
    pub messages: VecDeque<ChatMsg>,
    pub input: String,
    pub cursor: usize,
    pub scroll: usize,
    pub running: bool,
    pub i18n: I18n,
    pub panel: Panel,
    pub status_info: String,
    pub scenes: Vec<(String, bool)>, // (name, is_current)
    pub pending_memories: Vec<String>,
    pub trust_lines: Vec<String>,
}

#[derive(Clone)]
pub struct ChatMsg {
    pub role: MsgRole,
    pub content: String,
}

#[derive(Clone, PartialEq)]
pub enum MsgRole {
    User,
    Assistant,
    System,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            input: String::new(),
            cursor: 0,
            scroll: 0,
            running: true,
            i18n: I18n::new(),
            panel: Panel::Chat,
            status_info: String::new(),
            scenes: Vec::new(),
            pending_memories: Vec::new(),
            trust_lines: Vec::new(),
        }
    }

    pub fn add_msg(&mut self, role: MsgRole, content: String) {
        if self.messages.len() > 200 {
            self.messages.pop_front();
        }
        self.messages.push_back(ChatMsg { role, content });
    }

    pub fn scroll_up(&mut self) {
        self.scroll = (self.scroll + 1).min(self.messages.len().saturating_sub(1));
    }
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll = 0;
    }

    pub fn next_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Chat => Panel::Scenes,
            Panel::Scenes => Panel::Trust,
            Panel::Trust => Panel::Status,
            Panel::Status => Panel::Chat,
        };
    }
    pub fn prev_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Chat => Panel::Status,
            Panel::Scenes => Panel::Chat,
            Panel::Trust => Panel::Scenes,
            Panel::Status => Panel::Trust,
        };
    }

    pub fn fetch_status(&mut self) {
        let client = reqwest::blocking::Client::new();
        self.status_info = "  获取中...".into();
        self.scenes.clear();
        self.pending_memories.clear();

        // Get status
        if let Ok(resp) = client.get("http://127.0.0.1:8742/api/status").send() {
            self.status_info = resp.text().unwrap_or_else(|_| "连接失败".into());
        }
        // Get scenes
        if let Ok(resp) = client.get("http://127.0.0.1:8742/api/scenes").send() {
            if let Ok(text) = resp.text() {
                if let Ok(body) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = body["scenes"].as_array() {
                        for s in arr {
                            let name = s["name"].as_str().unwrap_or("?").to_string();
                            let curr = body["current"].as_str() == Some(&name);
                            self.scenes.push((name, curr));
                        }
                    }
                }
            }
        }
        // Get trust pending
        if let Ok(resp) = client
            .get("http://127.0.0.1:8742/api/meditate/pending")
            .send()
        {
            if let Ok(text) = resp.text() {
                if let Ok(body) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = body["pending"].as_array() {
                        for m in arr {
                            self.pending_memories.push(format!(
                                "[{}] score:{:.1}",
                                m["category"].as_str().unwrap_or("?"),
                                m["strength"].as_f64().unwrap_or(0.0)
                            ));
                        }
                    }
                }
            }
        }
    }
}
