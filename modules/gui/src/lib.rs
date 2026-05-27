//! GUI 桌面模块 — egui 原生窗口，四面板 | 2026-05-27

use eframe::egui::{self, CentralPanel, Color32, ScrollArea, TextEdit, Key, vec2, Layout, Align, TopBottomPanel, SidePanel, ViewportBuilder};
use eframe::{Frame, NativeOptions};

enum ActivePanel { Chat, Scenes, Trust, Status }
impl PartialEq for ActivePanel {
    fn eq(&self, other: &Self) -> bool { core::mem::discriminant(self) == core::mem::discriminant(other) }
}

struct BugsGui {
    input: String,
    messages: Vec<(String, String)>,
    panel: ActivePanel,
    scenes: Vec<String>,
    current_scene: String,
    trust_pending: Vec<String>,
    status_text: String,
}

impl Default for BugsGui {
    fn default() -> Self {
        let mut g = Self {
            input: String::new(), messages: vec![("system".into(), "🧠 Overmind 已就绪".into())],
            panel: ActivePanel::Chat, scenes: vec!["general".into()], current_scene: "general".into(),
            trust_pending: Vec::new(), status_text: "获取中...".into(),
        };
        g.refresh_data();
        g
    }
}

impl BugsGui {
    fn refresh_data(&mut self) {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2)).build().ok();
        let c = match client { Some(c) => c, None => return };
        // Scenes
        if let Ok(r) = c.get("http://127.0.0.1:8742/api/scenes").send() {
            if let Ok(t) = r.text() {
                if let Ok(j) = serde_json::from_str::<serde_json::Value>(&t) {
                    self.scenes = j["scenes"].as_array().map(|a| a.iter().filter_map(|s| s["name"].as_str().map(String::from)).collect()).unwrap_or_default();
                    self.current_scene = j["current"].as_str().unwrap_or("?").to_string();
                }
            }
        }
        // Trust
        if let Ok(r) = c.get("http://127.0.0.1:8742/api/meditate/pending").send() {
            if let Ok(t) = r.text() {
                if let Ok(j) = serde_json::from_str::<serde_json::Value>(&t) {
                    self.trust_pending = j["pending"].as_array().map(|a| a.iter().map(|m| {
                        format!("⚠️ {} s:{:.1}", m["category"].as_str().unwrap_or("?"), m["strength"].as_f64().unwrap_or(0.0))
                    }).collect()).unwrap_or_default();
                }
            }
        }
        // Status
        if let Ok(r) = c.get("http://127.0.0.1:8742/api/status").send() {
            if let Ok(t) = r.text() { self.status_text = t; }
        }
    }
}

impl eframe::App for BugsGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // ── 顶部标签栏 ──
        TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(matches!(self.panel, ActivePanel::Chat), "💬 对话").clicked() { self.panel = ActivePanel::Chat; self.refresh_data(); }
                if ui.selectable_label(matches!(self.panel, ActivePanel::Scenes), "📋 场景").clicked() { self.panel = ActivePanel::Scenes; self.refresh_data(); }
                if ui.selectable_label(matches!(self.panel, ActivePanel::Trust), "🔐 信任").clicked() { self.panel = ActivePanel::Trust; self.refresh_data(); }
                if ui.selectable_label(matches!(self.panel, ActivePanel::Status), "📊 状态").clicked() { self.panel = ActivePanel::Status; self.refresh_data(); }
            });
        });

        match self.panel {
            ActivePanel::Chat => self.draw_chat(ctx),
            ActivePanel::Scenes => self.draw_scenes(ctx),
            ActivePanel::Trust => self.draw_trust(ctx),
            ActivePanel::Status => self.draw_status(ctx),
        }
    }
}

impl BugsGui {
    fn draw_chat(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("💬 对话");
            ScrollArea::vertical().auto_shrink([false, false]).max_height(ui.available_height() - 60.0).show(ui, |ui| {
                for (role, content) in &self.messages {
                    let color = match role.as_str() { "user" => Color32::LIGHT_GREEN, "assistant" => Color32::LIGHT_YELLOW, _ => Color32::GRAY };
                    ui.colored_label(color, format!("{role}: {content}"));
                }
            });
            ui.horizontal(|ui| {
                let r = ui.add_sized([ui.available_width() - 80.0, 30.0],
                    TextEdit::singleline(&mut self.input).hint_text("输入... (Enter)"));
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    let msg = std::mem::take(&mut self.input);
                    if !msg.is_empty() { self.messages.push(("user".into(), msg)); }
                }
            });
        });
    }

    fn draw_scenes(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("📋 场景管理");
            ui.label(format!("当前: {}", self.current_scene));
            ui.separator();
            for s in &self.scenes {
                let is_cur = s == &self.current_scene;
                ui.colored_label(if is_cur { Color32::GREEN } else { Color32::WHITE }, if is_cur { format!("→ {s}") } else { format!("  {s}") });
            }
            if self.scenes.is_empty() { ui.label("无已注册场景"); }
        });
    }

    fn draw_trust(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("🔐 信任引擎");
            ui.horizontal(|ui| { ui.label("待确认记忆:"); if ui.button("🔄").clicked() { self.refresh_data(); } });
            ui.separator();
            if self.trust_pending.is_empty() { ui.label("✅ 无待确认记忆"); }
            for m in &self.trust_pending { ui.colored_label(Color32::YELLOW, m); }
        });
    }

    fn draw_status(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("📊 系统状态");
            if ui.button("🔄 刷新").clicked() { self.refresh_data(); }
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| {
                ui.label(&self.status_text);
            });
        });
    }
}

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size(vec2(1024.0, 768.0)),
        ..NativeOptions::default()
    };
    eframe::run_native("Bugs Overmind", options, Box::new(|_cc| Ok(Box::new(BugsGui::default()))))?;
    Ok(())
}
