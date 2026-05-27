use crate::app::{App, ChatMsg, MsgRole, Panel};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Tabs};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    // ── 标签页 ──
    let tab_names = vec![" Chat ", " Scenes ", " Trust ", " Status "].into_iter().map(ratatui::text::Line::from).collect::<Vec<_>>();
    let selected = match app.panel {
        Panel::Chat => 0, Panel::Scenes => 1, Panel::Trust => 2, Panel::Status => 3,
    };
    let tabs = Tabs::new(tab_names)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    // ── 标题栏 ──
    let title = Paragraph::new(app.i18n.title())
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan)))
        .centered();
    f.render_widget(title, chunks[1]);

    // ── 内容区 ──
    match app.panel {
        Panel::Chat => draw_chat(f, app, chunks[2]),
        Panel::Scenes => draw_scenes(f, app, chunks[2]),
        Panel::Trust => draw_trust(f, app, chunks[2]),
        Panel::Status => draw_status(f, app, chunks[2]),
    }

    // ── 底部帮助 ──
    let help = Paragraph::new(format!("Tab/S-Tab切换面板 | {}", app.i18n.help_text()))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}

fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let visible_msgs: Vec<&ChatMsg> = app.messages.iter().rev().skip(app.scroll).take(chunks[0].height as usize - 2)
        .collect::<Vec<_>>().into_iter().rev().collect();
    let mut lines: Vec<Line> = Vec::new();
    for msg in &visible_msgs {
        let (prefix, color) = match msg.role {
            MsgRole::User => ("❯ ", Color::Green),
            MsgRole::Assistant => ("🧠 ", Color::Yellow),
            MsgRole::System => ("  ", Color::DarkGray),
        };
        for line in msg.content.lines() {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(line, Style::default().fg(Color::White)),
            ]));
        }
    }
    let chat = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(chat, chunks[0]);

    let input_text = format!("{}|", &app.input);
    let input = Paragraph::new(input_text)
        .block(Block::default().borders(Borders::ALL).title(app.i18n.input_placeholder()))
        .style(Style::default().fg(Color::White));
    f.render_widget(input, chunks[1]);
}

fn draw_scenes(f: &mut Frame, _app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(" 场景列表", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" ───────────────", Style::default().fg(Color::DarkGray))),
    ];
    for (name, current) in &_app.scenes {
        let marker = if *current { "→ " } else { "   " };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Green)),
            Span::styled(name, Style::default().fg(if *current { Color::Green } else { Color::White })),
        ]));
    }
    if _app.scenes.is_empty() {
        lines.push(Line::from(Span::raw("  无已注册场景")));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::raw(" /scene <name>  — 切换到指定场景")));
    let p = Paragraph::new(Text::from(lines)).block(Block::default().borders(Borders::ALL).title("📋 场景"));
    f.render_widget(p, area);
}

fn draw_trust(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(" 待确认记忆", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" ───────────────", Style::default().fg(Color::DarkGray))),
    ];
    for m in &app.pending_memories {
        lines.push(Line::from(Span::raw(format!("  ⚠️  {m}"))));
    }
    if app.pending_memories.is_empty() {
        lines.push(Line::from(Span::raw("  无待确认记忆")));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::raw(" 这些记忆信任值偏低，可运行 /meditate 整理")));
    let p = Paragraph::new(Text::from(lines)).block(Block::default().borders(Borders::ALL).title("🔐 信任引擎"));
    f.render_widget(p, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(" 系统状态", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" ───────────────", Style::default().fg(Color::DarkGray))),
    ];
    for line in app.status_info.lines() {
        lines.push(Line::from(Span::raw(format!("  {line}"))));
    }
    let p = Paragraph::new(Text::from(lines)).block(Block::default().borders(Borders::ALL).title("📊 状态"));
    f.render_widget(p, area);
}
