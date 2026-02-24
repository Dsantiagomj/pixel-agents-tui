use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum JsonlRecord {
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "user")]
    User { message: UserMessage },
    #[serde(rename = "system")]
    System {
        subtype: Option<String>,
        #[serde(default)]
        duration_ms: Option<u64>,
    },
    #[serde(rename = "progress")]
    Progress {
        subtype: Option<String>,
        #[serde(flatten)]
        data: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
pub struct UserMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_tool_use_record() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool_1","name":"Read","input":{"file_path":"/tmp/test.rs"}}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::Assistant { message } => {
                assert_eq!(message.content.len(), 1);
                match &message.content[0] {
                    ContentBlock::ToolUse { id, name, .. } => {
                        assert_eq!(name, "Read");
                        assert_eq!(id, "tool_1");
                    }
                    _ => panic!("Expected ToolUse"),
                }
            }
            _ => panic!("Expected Assistant record"),
        }
    }

    #[test]
    fn deserialize_tool_result_record() {
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool_1"}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::User { message } => match &message.content[0] {
                ContentBlock::ToolResult { tool_use_id } => {
                    assert_eq!(tool_use_id, "tool_1");
                }
                _ => panic!("Expected ToolResult"),
            },
            _ => panic!("Expected User record"),
        }
    }

    #[test]
    fn deserialize_turn_duration_record() {
        let json = r#"{"type":"system","subtype":"turn_duration","duration_ms":1500}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::System { subtype, .. } => {
                assert_eq!(subtype.as_deref(), Some("turn_duration"));
            }
            _ => panic!("Expected System record"),
        }
    }

    #[test]
    fn deserialize_text_content() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::Assistant { message } => match &message.content[0] {
                ContentBlock::Text { text } => {
                    assert_eq!(text, "Hello world");
                }
                _ => panic!("Expected Text"),
            },
            _ => panic!("Expected Assistant"),
        }
    }

    #[test]
    fn unknown_record_types_dont_crash() {
        let json = r#"{"type":"unknown_future_type","data":123}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        assert!(matches!(record, JsonlRecord::Unknown));
    }
}
