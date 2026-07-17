use super::{
    is_hidden_local_session_record_type, is_image_placeholder_local_session_title,
    is_json_non_message_type, is_json_thinking_type, is_json_tool_type,
    is_version_like_local_session_title, LocalSessionRecordClassification,
};
use serde_json::Value;
use std::path::Path;

pub(super) fn is_supported_local_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jsonl" | "json" | "txt" | "log"
            )
        })
        .unwrap_or(false)
}

pub(super) fn local_session_metadata_is_internal(agent: Option<&str>, value: &Value) -> bool {
    match agent.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("codex") => codex_session_meta_is_internal(value),
        Some("claude") | Some("claude-code") => claude_session_record_is_sidechain(value),
        Some("opencode") => opencode_session_record_is_child(value),
        Some("hermes") => hermes_session_record_is_internal(value),
        Some("openclaw") => openclaw_session_record_is_internal(value),
        None | Some("") | Some("all") => {
            codex_session_meta_is_internal(value)
                || claude_session_record_is_sidechain(value)
                || opencode_session_record_is_child(value)
                || hermes_session_record_is_internal(value)
                || openclaw_session_record_is_internal(value)
        }
        _ => false,
    }
}

fn codex_session_meta_is_internal(value: &Value) -> bool {
    let Some(record) = value.as_object() else {
        return false;
    };
    if record.get("type").and_then(Value::as_str) != Some("session_meta") {
        return false;
    }
    let metadata = record
        .get("payload")
        .and_then(Value::as_object)
        .unwrap_or(record);
    if let Some(thread_source) = metadata.get("thread_source") {
        match thread_source {
            Value::String(source) if !source.trim().is_empty() => {
                if !source.trim().eq_ignore_ascii_case("user") {
                    return true;
                }
            }
            Value::Object(source) if !source.is_empty() => return true,
            _ => {}
        }
    }
    match metadata.get("source") {
        Some(Value::String(source)) if !source.trim().is_empty() => !matches!(
            source.trim().to_ascii_lowercase().as_str(),
            "cli" | "vscode"
        ),
        Some(Value::Object(source)) => source.contains_key("subagent"),
        _ => false,
    }
}

fn claude_session_record_is_sidechain(value: &Value) -> bool {
    let Some(metadata) = metadata_object(value) else {
        return false;
    };
    metadata
        .get("isSidechain")
        .or_else(|| metadata.get("is_sidechain"))
        .and_then(Value::as_bool)
        == Some(true)
}

fn opencode_session_record_is_child(value: &Value) -> bool {
    let Some(metadata) = metadata_object(value) else {
        return false;
    };
    let has_session_identity = ["id", "sessionID", "sessionId", "session_id"]
        .iter()
        .any(|key| metadata.get(*key).and_then(Value::as_str).is_some());
    has_session_identity
        && ["parentID", "parentId", "parent_id"]
            .iter()
            .any(|key| metadata.get(*key).is_some_and(nonempty_metadata_value))
}

fn hermes_session_record_is_internal(value: &Value) -> bool {
    metadata_object(value)
        .and_then(|metadata| metadata.get("source"))
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .is_some_and(|source| {
            matches!(
                source.as_str(),
                "cron" | "batch" | "subagent" | "memory" | "memory_consolidation"
            )
        })
}

fn openclaw_session_record_is_internal(value: &Value) -> bool {
    let Some(metadata) = metadata_object(value) else {
        return false;
    };
    ["sessionKey", "session_key", "key"]
        .iter()
        .filter_map(|key| metadata.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .any(|key| {
            key.starts_with("cron:")
                || key.starts_with("hook:")
                || [":subagent:", ":cron:", ":hook:", ":heartbeat:", ":acp:"]
                    .iter()
                    .any(|marker| key.contains(marker))
        })
}

fn metadata_object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    let record = value.as_object()?;
    Some(
        record
            .get("payload")
            .and_then(Value::as_object)
            .unwrap_or(record),
    )
}

fn nonempty_metadata_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        _ => true,
    }
}

pub(super) fn is_internal_local_session_title_block(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("# agents.md instructions")
        || lower.starts_with("<permissions instructions>")
        || lower.starts_with("<environment_context>")
        || lower.starts_with("<recommended_plugins>")
        || lower.starts_with("<recommended_skills>")
        || lower.starts_with("<recommended_apps>")
        || lower.starts_with("<recommended_connectors>")
        || lower.starts_with("<apps_instructions>")
        || lower.starts_with("<plugins_instructions>")
        || lower.starts_with("<skills_instructions>")
        || lower.starts_with("<app-context>")
        || lower.starts_with("<collaboration_mode>")
        || lower.starts_with("<multi_agent_mode>")
        || lower.starts_with("<local-command-caveat>")
        || lower.starts_with("<command-")
        || lower.starts_with("<skill name=")
        || lower.starts_with("<turn_")
        || lower.starts_with("you are a delegated subagent")
        || lower.starts_with("you are codex")
        || lower.starts_with("shared instruction entrypoint")
}

pub(super) fn is_unhelpful_local_session_title(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let trimmed = value.trim();
    is_internal_local_session_title_block(value)
        || lower == "normal"
        || lower == "head"
        || lower == "main"
        || lower == "null"
        || lower == "clear"
        || lower == "cls"
        || is_image_placeholder_local_session_title(trimmed)
        || trimmed.starts_with("$HOME")
        || trimmed.starts_with('/')
        || is_version_like_local_session_title(trimmed)
}

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
