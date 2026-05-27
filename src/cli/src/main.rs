use clap::{Parser, Subcommand};
use bugs_core::types::BugsConfig;
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "bugs", about = "Bugs — AI Agent 运行时")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动守护进程
    Start,
    /// 停止守护进程
    Stop,
    /// 查看守护进程状态
    Status,
    /// 打开 TUI 终端界面
    Tui,
    /// 启动 WebUI
    Web,
    /// 启动 GUI 桌面
    Gui,
    /// 对话模式（默认）
    Chat,
}

fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Chat) {
        Commands::Start => daemon_cmd("start"),
        Commands::Stop  => daemon_cmd("stop"),
        Commands::Status => daemon_cmd("status"),
        Commands::Tui   => frontend("bugs-tui"),
        Commands::Web   => frontend("bugs-web"),
        Commands::Gui   => frontend("bugs-gui"),
        Commands::Chat  => chat_mode(),
    }
}

fn daemon_cmd(action: &str) {
    match action {
        "start" => {
            if daemon_running() {
                eprintln!("bugs-daemon 已在运行");
                return;
            }
            let child = Command::new("bugs-daemon").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
            match child {
                Ok(_) => println!("✅ bugs-daemon 已启动"),
                Err(e) => eprintln!("❌ 无法启动 bugs-daemon: {e}"),
            }
        }
        "stop" => {
            let output = Command::new("pkill").arg("bugs-daemon").output();
            match output {
                Ok(_) => println!("✅ bugs-daemon 已停止"),
                Err(_) => eprintln!("bugs-daemon 未在运行"),
            }
        }
        "status" => {
            if daemon_running() {
                println!("🟢 bugs-daemon 运行中 (http://127.0.0.1:8742)");
            } else {
                println!("⚫ bugs-daemon 未运行");
            }
        }
        _ => {}
    }
}

fn frontend(bin: &str) {
    if !daemon_running() {
        println!("⚠️  bugs-daemon 未运行，正在启动...");
        daemon_cmd("start");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let status = Command::new(bin).spawn();
    match status {
        Ok(mut child) => { let _ = child.wait(); }
        Err(_) => eprintln!("❌ 无法启动 {bin}，请确认已安装"),
    }
}

fn daemon_running() -> bool {
    Command::new("pgrep").arg("bugs-daemon").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

fn chat_mode() {
    if !daemon_running() {
        println!("⚠️  bugs-daemon 未运行，正在启动...");
        daemon_cmd("start");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    // Simple chat via API
    println!("🧠 Overmind 已就绪 (输入 /exit 退出)");
    let mut input = String::new();
    loop {
        print!("❯ ");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        input.clear();
        if std::io::stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();
        if input == "/exit" { break; }
        if input.is_empty() { continue; }
        let client = reqwest::blocking::Client::new();
        match client.post("http://127.0.0.1:8742/api/chat")
            .json(&serde_json::json!({"messages":[{"role":"user","content":input}]}))
            .send() {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    println!("\n{}", body["content"].as_str().unwrap_or(&body.to_string()));
                    println!();
                }
            }
            Err(e) => eprintln!("❌ {e}"),
        }
    }
}
