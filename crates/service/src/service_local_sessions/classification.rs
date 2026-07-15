use super::{
    is_hidden_local_session_record_type, is_json_non_message_type, is_json_thinking_type,
    is_json_tool_type, LocalSessionRecordClassification,
};

pub(super) fn local_session_type_name_classification(
    record_type: &str,
) -> LocalSessionRecordClassification {
    let normalized = record_type.to_ascii_lowercase().replace(['_', '-'], "");
    if is_hidden_local_session_record_type(record_type) || is_json_non_message_type(&normalized) {
        LocalSessionRecordClassification::Deny
    } else if is_json_tool_type(&normalized) {
        LocalSessionRecordClassification::Tool
    } else if is_json_thinking_type(&normalized) {
        LocalSessionRecordClassification::Thinking
    } else {
        match normalized.as_str() {
            "user" | "human" => LocalSessionRecordClassification::User,
            "assistant" | "agent" | "model" | "agentmessage" => {
                LocalSessionRecordClassification::Assistant
            }
            "agentreasoning" => LocalSessionRecordClassification::Thinking,
            "session" | "sessionmeta" | "responseitem" | "eventmsg" | "message" | "inputtext"
            | "outputtext" | "text" => LocalSessionRecordClassification::KnownStructure,
            _ => LocalSessionRecordClassification::Unproven,
        }
    }
}

pub(super) fn local_session_role_classification(role: &str) -> LocalSessionRecordClassification {
    let normalized = role.to_ascii_lowercase().replace(['_', '-'], "");
    match normalized.as_str() {
        "user" | "human" | "customer" => LocalSessionRecordClassification::User,
        "assistant" | "agent" | "model" => LocalSessionRecordClassification::Assistant,
        "tool" | "function" | "toolresult" => LocalSessionRecordClassification::Tool,
        "system" | "developer" | "summary" => LocalSessionRecordClassification::Deny,
        _ => LocalSessionRecordClassification::Unproven,
    }
}
