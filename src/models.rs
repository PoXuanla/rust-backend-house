use serde::{Serialize,Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    #[serde(rename = "chat")]
    Chat(ChatPayload),
    #[serde(rename = "system")]
    System(String)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatPayload {
    pub username: String,
    pub content: String,
    pub pro_config: Option<ProConfig>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProConfig {
    pub frame_style: String,
    pub badge: String
}