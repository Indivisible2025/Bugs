//! 微信消息解析（个人号 XML 格式）

use serde::{Deserialize, Serialize};

/// 接收消息
#[derive(Debug, Deserialize)]
pub struct WxMessage {
    #[serde(rename = "ToUserName")]
    pub to_user: String,
    #[serde(rename = "FromUserName")]
    pub from_user: String,
    #[serde(rename = "CreateTime")]
    pub create_time: u64,
    #[serde(rename = "MsgType")]
    pub msg_type: String,
    #[serde(rename = "Content")]
    pub content: Option<String>,
    #[serde(rename = "MsgId")]
    pub msg_id: Option<String>,
}

/// 回复消息
#[derive(Debug, Serialize)]
pub struct WxReply {
    #[serde(rename = "ToUserName")]
    pub to_user: String,
    #[serde(rename = "FromUserName")]
    pub from_user: String,
    #[serde(rename = "CreateTime")]
    pub create_time: u64,
    #[serde(rename = "MsgType")]
    pub msg_type: String,
    #[serde(rename = "Content")]
    pub content: String,
}

impl WxReply {
    pub fn text(from: String, to: String, content: String) -> Self {
        Self {
            to_user: to,
            from_user: from,
            create_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            msg_type: "text".into(),
            content,
        }
    }
    pub fn to_xml(&self) -> String {
        format!("<xml><ToUserName><![CDATA[{}]]></ToUserName><FromUserName><![CDATA[{}]]></FromUserName><CreateTime>{}</CreateTime><MsgType><![CDATA[text]]></MsgType><Content><![CDATA[{}]]></Content></xml>",
            self.to_user, self.from_user, self.create_time, self.content)
    }
}
