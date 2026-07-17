use super::*;
use std::io::Write;

fn message_params(
    root: &Path,
    session_id: &str,
    cursor: Option<String>,
    source_revision: Option<String>,
) -> LocalSessionMessagePageParams {
    LocalSessionMessagePageParams {
        authorized_roots: vec![root.to_string_lossy().to_string()],
        auto_discover: Some(false),
        agent: Some("codex".to_string()),
        project_root: None,
        current_cwd: None,
        session_id: session_id.to_string(),
        limit: Some(1),
        cursor,
        source_revision,
    }
}

#[test]
fn huge_codex_transcript_pages_all_final_messages_without_mirrors_or_append_drift() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-message-page-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let root = app_data_dir.join("sessions");
    fs::create_dir_all(&root).expect("create session root");
    let path = root.join("rollout.jsonl");
    let oversized_tool_output = "x".repeat(33 * 1024 * 1024);
    let goal = "真实测试 agent pet maker 技能并处理发现的问题";
    let records = [
        json!({"timestamp":"2026-07-17T01:00:00Z","type":"session_meta","payload":{"id":"native-session"}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"first request"}]}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"first request"}}).to_string(),
        format!(r#"{{"timestamp":"2026-07-17T01:00:02Z","type":"response_item","payload":{{"type":"custom_tool_call_output","output":"{oversized_tool_output}"}}}}"#),
        json!({"timestamp":"2026-07-17T01:00:03Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("<codex_internal_context source=\"goal\">\n<objective>\n{goal}\n</objective>\n</codex_internal_context>")}]}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:04Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"finished response"}],"phase":"final_answer"}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:04Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("<codex_internal_context source=\"goal\">\n<objective>\n{goal}\n</objective>\n</codex_internal_context>")}]}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:04Z","type":"event_msg","payload":{"type":"agent_message","message":"finished response","phase":"final_answer"}}).to_string(),
    ];
    fs::write(&path, format!("{}\n", records.join("\n"))).expect("write transcript");
    let canonical_path = path.canonicalize().expect("canonical session path");
    let session_id = crate::service_local_sessions::local_session_row_id(&canonical_path);
    let host = test_host(app_data_dir.clone());

    let first = host
        .list_local_session_messages(message_params(&root, &session_id, None, None))
        .expect("first message page");
    assert_eq!(first.content_items.len(), 1);
    assert_eq!(first.content_items[0].text, "first request");
    assert!(first.has_more);

    let appended = json!({"timestamp":"2026-07-17T01:00:05Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"appended after snapshot"}]}}).to_string();
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open transcript for append");
    writeln!(file, "{appended}").expect("append transcript");

    let mut messages = first.content_items;
    let mut cursor = first.next_cursor;
    let revision = first.source_revision;
    let mut saw_empty_progress_page = false;
    let mut final_total = None;
    let mut page_shapes = Vec::new();
    for _ in 0..8 {
        let page = host
            .list_local_session_messages(message_params(
                &root,
                &session_id,
                cursor.clone(),
                Some(revision.clone()),
            ))
            .expect("continuation message page");
        saw_empty_progress_page |= page.content_items.is_empty() && page.has_more;
        page_shapes.push((
            page.content_items.len(),
            page.scanned_bytes,
            page.scanned_through_bytes,
            page.has_more,
        ));
        messages.extend(page.content_items);
        cursor = page.next_cursor;
        final_total = page.total_count;
        if !page.has_more {
            break;
        }
    }

    assert!(saw_empty_progress_page, "the oversized tool record should advance without becoming a message or blocking a page: {page_shapes:?}");
    assert_eq!(messages.len(), 3);
    assert_eq!(final_total, Some(3));
    assert_eq!(
        messages
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        vec!["first request", goal, "finished response"]
    );
    assert_eq!(
        messages
            .iter()
            .map(|item| item.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["user_message", "user_message", "agent_reply"]
    );
    assert!(!messages
        .iter()
        .any(|item| item.text == "appended after snapshot"));

    let _ = fs::remove_dir_all(app_data_dir);
}
