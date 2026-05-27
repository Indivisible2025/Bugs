//! LSP 集成 — 语言服务器协议客户端
//! 通过 JSON-RPC 与语言服务器通信，提供代码级上下文

use serde_json::Value;
use std::io::{BufRead, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

/// LSP 客户端，管理一个语言服务器进程
pub struct LspClient {
    process: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    stdout: Mutex<Option<std::io::BufReader<ChildStdout>>>,
    request_id: std::sync::atomic::AtomicU64,
    /// 语言服务器可执行路径
    server_path: String,
}

#[derive(Debug, Clone)]
pub struct LspSymbol {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub file_path: String,
    pub message: String,
    pub severity: String,
    pub line: u32,
}

impl LspClient {
    /// 创建 LSP 客户端
    ///
    /// `server_path`: 语言服务器路径，如 `rust-analyzer`, `typescript-language-server`
    pub fn new(server_path: impl Into<String>) -> Self {
        Self {
            process: Mutex::new(None),
            stdin: Mutex::new(None),
            stdout: Mutex::new(None),
            request_id: std::sync::atomic::AtomicU64::new(1),
            server_path: server_path.into(),
        }
    }

    /// 启动语言服务器进程并初始化
    pub fn start(&self, root_uri: &str) -> Result<(), String> {
        let mut child = Command::new(&self.server_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("无法启动 LSP 服务器 `{}`: {e}", self.server_path))?;

        let stdin = child.stdin.take().ok_or("stdin 不可用")?;
        let stdout = child.stdout.take().ok_or("stdout 不可用")?;
        let reader = std::io::BufReader::new(stdout);

        *self.stdin.lock().unwrap() = Some(stdin);
        *self.stdout.lock().unwrap() = Some(reader);
        *self.process.lock().unwrap() = Some(child);

        // 发送 initialize 请求
        let result = self.send_request(
            "initialize",
            serde_json::json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {}
            }),
        )?;

        // 发送 initialized 通知
        self.send_notification("initialized", serde_json::json!({}));

        Ok(())
    }

    /// 打开文件
    pub fn did_open(&self, path: &str, language: &str, text: &str) -> Result<(), String> {
        self.send_notification(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": format!("file://{path}"),
                    "languageId": language,
                    "version": 1,
                    "text": text,
                }
            }),
        );
        Ok(())
    }

    /// 获取诊断信息（不等待推送，直接请求）
    pub fn diagnostics(&self, path: &str) -> Result<Vec<LspDiagnostic>, String> {
        let result = self.send_request(
            "textDocument/diagnostic",
            serde_json::json!({
                "textDocument": { "uri": format!("file://{path}") }
            }),
        )?;
        let mut diags = Vec::new();
        if let Some(items) = result["diagnostics"].as_array() {
            for d in items {
                diags.push(LspDiagnostic {
                    file_path: path.into(),
                    message: d["message"].as_str().unwrap_or("?").to_string(),
                    severity: d["severity"]
                        .as_i64()
                        .map(|s| match s {
                            1 => "error",
                            2 => "warn",
                            _ => "info",
                        })
                        .unwrap_or("info")
                        .to_string(),
                    line: d["range"]["start"]["line"].as_i64().unwrap_or(0) as u32 + 1,
                });
            }
        }
        Ok(diags)
    }

    /// 跳转到定义
    pub fn go_to_definition(
        &self,
        path: &str,
        line: u32,
        col: u32,
    ) -> Result<Vec<LspSymbol>, String> {
        let result = self.send_request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": format!("file://{path}") },
                "position": { "line": line, "character": col }
            }),
        )?;
        self.parse_locations(result)
    }

    /// 查找引用
    pub fn find_references(
        &self,
        path: &str,
        line: u32,
        col: u32,
    ) -> Result<Vec<LspSymbol>, String> {
        let result = self.send_request(
            "textDocument/references",
            serde_json::json!({
                "textDocument": { "uri": format!("file://{path}") },
                "position": { "line": line, "character": col },
                "context": { "includeDeclaration": true }
            }),
        )?;
        self.parse_locations(result)
    }

    /// 按下标记 — 获取当前光标位置的语义上下文
    pub fn hover(&self, path: &str, line: u32, col: u32) -> Result<String, String> {
        let result = self.send_request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": format!("file://{path}") },
                "position": { "line": line, "character": col }
            }),
        )?;
        Ok(result["contents"]["value"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// 关闭语言服务器
    pub fn shutdown(&self) {
        let _ = self.send_request("shutdown", serde_json::json!(null));
        self.send_notification("exit", serde_json::json!({}));
        if let Ok(mut proc) = self.process.lock() {
            if let Some(ref mut p) = *proc {
                let _ = p.wait();
            }
        }
    }

    // ── JSON-RPC 通信 ──

    fn next_id(&self) -> u64 {
        self.request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id();
        let msg = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        self.write(&msg)?;
        self.read_response(id)
    }

    fn send_notification(&self, method: &str, params: Value) {
        let msg = serde_json::json!({"jsonrpc":"2.0","method":method,"params":params});
        let _ = self.write(&msg);
    }

    fn write(&self, msg: &Value) -> Result<(), String> {
        let content = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        let mut stdin = self.stdin.lock().unwrap();
        if let Some(ref mut s) = *stdin {
            s.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
            s.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
            s.flush().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn read_response(&self, expected_id: u64) -> Result<Value, String> {
        let buf = String::new();
        let mut stdout = self.stdout.lock().unwrap();
        if let Some(ref mut r) = *stdout {
            // 读取 Content-Length header
            let mut header = String::new();
            r.read_line(&mut header).map_err(|e| e.to_string())?;
            let len: usize = header
                .trim_start_matches("Content-Length: ")
                .trim()
                .parse()
                .unwrap_or(0);
            r.read_line(&mut header).map_err(|e| e.to_string())?; // 空行
            let mut content = vec![0u8; len];
            use std::io::Read;
            r.read_exact(&mut content).map_err(|e| e.to_string())?;
            let resp: Value = serde_json::from_slice(&content).map_err(|e| e.to_string())?;
            if let Some(err) = resp.get("error") {
                return Err(err["message"].as_str().unwrap_or("LSP 错误").to_string());
            }
            // 结果可能是单个响应或批量响应
            if let Some(result) = resp.get("result") {
                Ok(result.clone())
            } else {
                Ok(serde_json::json!(null))
            }
        } else {
            Err("stdout 不可用".into())
        }
    }

    fn parse_locations(&self, result: Value) -> Result<Vec<LspSymbol>, String> {
        let mut symbols = Vec::new();
        let locations = match result {
            Value::Array(arr) => arr,
            Value::Object(ref m) if m.contains_key("uri") => vec![result],
            _ => return Ok(symbols),
        };
        for loc in locations {
            symbols.push(LspSymbol {
                name: String::new(),
                kind: "location".into(),
                file_path: loc["uri"].as_str().unwrap_or("").to_string(),
                line: loc["range"]["start"]["line"].as_i64().unwrap_or(0) as u32 + 1,
                column: loc["range"]["start"]["character"].as_i64().unwrap_or(0) as u32,
            });
        }
        Ok(symbols)
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_creation() {
        let c = LspClient::new("rust-analyzer");
        assert!(c.server_path.contains("rust-analyzer"));
    }
    #[test]
    fn fails_on_nonexistent_server() {
        let c = LspClient::new("nonexistent-language-server");
        assert!(c.start("file:///tmp").is_err());
    }
}
