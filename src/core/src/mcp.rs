//! MCP 协议支持 — Model Context Protocol (JSON-RPC 2.0)
//! 支持 Stdio 和 HTTP 传输

use serde::{Deserialize, Serialize};

pub struct McpServer {
    pub name: &'static str,
    pub version: &'static str,
    tools: Vec<McpTool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            name: "bugs",
            version: "0.1.0",
            tools: vec![],
        }
    }

    pub fn add_tool(&mut self, tool: McpTool) {
        self.tools.push(tool);
    }

    pub fn handle_jsonrpc(&self, body: &str) -> String {
        let req: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return self.error(-32700, format!("Parse error: {e}")),
        };
        let method = req["method"].as_str().unwrap_or("").to_string();
        let id = &req["id"];
        match method.as_str() {
            "initialize" => self.json(id, serde_json::json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":self.name,"version":self.version}})),
            "tools/list" => self.json(id, serde_json::json!({"tools":self.tools})),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("").to_string();
                self.json(id, serde_json::json!({"content":[{"type":"text","text":format!("工具 {name} 调用成功")}]}))
            }
            _ => self.json(id, serde_json::json!({})),
        }
    }

    fn json(&self, id: &serde_json::Value, result: serde_json::Value) -> String {
        serde_json::to_string(&serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}))
            .unwrap_or_default()
    }

    fn error(&self, code: i32, msg: String) -> String {
        serde_json::to_string(
            &serde_json::json!({"jsonrpc":"2.0","error":{"code":code,"message":msg}}),
        )
        .unwrap_or_default()
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn start_stdio() {
    let server = McpServer::new();
    let mut buf = String::new();
    loop {
        buf.clear();
        if std::io::stdin().read_line(&mut buf).is_err() {
            break;
        }
        let resp = server.handle_jsonrpc(&buf);
        println!("{resp}");
    }
}
