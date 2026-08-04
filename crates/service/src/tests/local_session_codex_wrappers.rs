use super::local_session_preview::preview_codex_session_fixture;
use super::*;

#[test]
fn local_session_preview_normalizes_codex_desktop_wrappers() {
    let user = json!({
        "timestamp": "2026-07-14T06:32:26.787Z",
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "inspect the active session"}]
        }
    });
    let reasoning = json!({
        "timestamp": "2026-07-14T06:32:27.000Z",
        "type": "event_msg",
        "payload": {"type": "agent_reasoning", "text": "checking the bounded log"}
    });
    let tool = json!({
        "timestamp": "2026-07-14T06:32:28.000Z",
        "type": "response_item",
        "payload": {
            "type": "custom_tool_call",
            "name": "exec_command",
            "input": {"cmd": "cargo test"}
        }
    });
    let assistant = json!({
        "timestamp": "2026-07-14T06:32:29.000Z",
        "type": "event_msg",
        "payload": {"type": "agent_message", "message": "the session is readable"}
    });
    let result = preview_codex_session_fixture(
        "codex-desktop-wrappers",
        &format!("{user}\n{reasoning}\n{tool}\n{assistant}\n"),
    );

    assert_eq!(
        result.get("user_message_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result.get("tool_call_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result.get("total_message_count").and_then(Value::as_u64),
        Some(3)
    );
    let kinds = result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .expect("content items")
        .iter()
        .filter_map(|item| item.get("kind").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"user_message"));
    assert!(kinds.contains(&"thinking"));
    assert!(kinds.contains(&"tool_call"));
    assert!(kinds.contains(&"agent_reply"));
}

#[test]
fn codex_commentary_is_thinking_and_only_final_answer_is_agent_reply() {
    let user = json!({
        "timestamp": "2026-07-14T06:32:26.787Z",
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "audit the adapters"}]
        }
    });
    let commentary_before_tool = json!({
        "timestamp": "2026-07-14T06:32:27.000Z",
        "type": "event_msg",
        "payload": {
            "type": "agent_message",
            "phase": "commentary",
            "message": "I found the parser entry point and will inspect each adapter."
        }
    });
    let tool = json!({
        "timestamp": "2026-07-14T06:32:28.000Z",
        "type": "response_item",
        "payload": {
            "type": "custom_tool_call",
            "name": "exec_command",
            "input": {"cmd": "cargo test"}
        }
    });
    let commentary_after_tool = json!({
        "timestamp": "2026-07-14T06:32:29.000Z",
        "type": "event_msg",
        "payload": {
            "type": "agent_message",
            "phase": "commentary",
            "message": "The focused test reproduces the classification bug."
        }
    });
    let final_answer = json!({
        "timestamp": "2026-07-14T06:32:30.000Z",
        "type": "event_msg",
        "payload": {
            "type": "agent_message",
            "phase": "final_answer",
            "message": "The adapter classification audit is complete."
        }
    });
    let result = preview_codex_session_fixture(
        "codex-commentary-and-final-answer",
        &format!(
            "{user}\n{commentary_before_tool}\n{tool}\n{commentary_after_tool}\n{final_answer}\n"
        ),
    );

    let kinds = result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .expect("content items")
        .iter()
        .filter_map(|item| item.get("kind").and_then(Value::as_str))
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            "user_message",
            "thinking",
            "tool_call",
            "thinking",
            "agent_reply"
        ]
    );
}
