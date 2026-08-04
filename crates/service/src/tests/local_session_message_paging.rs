use super::*;
use std::io::Write;

fn message_params(
    root: &Path,
    session_id: &str,
    cursor: Option<String>,
    source_revision: Option<String>,
) -> LocalSessionMessagePageParams {
    message_params_for_agent(root, "codex", session_id, 1, cursor, source_revision)
}

fn message_params_for_agent(
    root: &Path,
    agent: &str,
    session_id: &str,
    limit: usize,
    cursor: Option<String>,
    source_revision: Option<String>,
) -> LocalSessionMessagePageParams {
    LocalSessionMessagePageParams {
        authorized_roots: vec![root.to_string_lossy().to_string()],
        auto_discover: Some(false),
        agent: Some(agent.to_string()),
        project_root: None,
        current_cwd: None,
        session_id: session_id.to_string(),
        limit: Some(limit),
        cursor,
        source_revision,
    }
}

#[test]
fn huge_codex_transcript_pages_messages_and_thinking_without_mirrors_or_append_drift() {
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
        json!({"timestamp":"2026-07-17T01:00:01Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"I will inspect the stored events first."}],"phase":"commentary"}}).to_string(),
        json!({"timestamp":"2026-07-17T01:00:01Z","type":"event_msg","payload":{"type":"agent_message","message":"I will inspect the stored events first.","phase":"commentary"}}).to_string(),
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
    assert_eq!(messages.len(), 4);
    assert_eq!(final_total, Some(4));
    assert_eq!(
        messages
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "first request",
            "I will inspect the stored events first.",
            goal,
            "finished response"
        ]
    );
    assert_eq!(
        messages
            .iter()
            .map(|item| item.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["user_message", "thinking", "user_message", "agent_reply"]
    );
    assert!(!messages
        .iter()
        .any(|item| item.text == "appended after snapshot"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn selected_session_pages_every_tool_and_skill_call_past_preview_sample_limit() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-complete-process-page-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let root = app_data_dir.join("sessions");
    fs::create_dir_all(&root).expect("create session root");
    let path = root.join("complete-process.jsonl");
    let mut records = vec![
        json!({"timestamp":"2026-07-17T01:00:00Z","type":"session_meta","payload":{"id":"complete-process"}}).to_string(),
    ];
    for index in 0..260 {
        records.push(
            json!({
                "timestamp": format!("2026-07-17T01:{:02}:{:02}Z", (index / 60) % 60, index % 60),
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type":"input_text","text":format!("run /skill:demo-{index:03}")}]
                }
            })
            .to_string(),
        );
        records.push(
            json!({
                "timestamp": format!("2026-07-17T02:{:02}:{:02}Z", (index / 60) % 60, index % 60),
                "type": "response_item",
                "payload": {"type":"custom_tool_call","name":format!("tool-{index:03}"),"input":"{}"}
            })
            .to_string(),
        );
    }
    records.push(
        json!({
            "timestamp":"2026-07-17T03:00:00Z",
            "type":"response_item",
            "payload":{"type":"custom_tool_call_output","output":"must-not-become-a-call"}
        })
        .to_string(),
    );
    fs::write(&path, format!("{}\n", records.join("\n"))).expect("write transcript");
    let session_id = crate::service_local_sessions::local_session_row_id(
        &path.canonicalize().expect("canonical session path"),
    );
    let host = test_host(app_data_dir.clone());

    let mut cursor = None;
    let mut revision = None;
    let mut items = Vec::new();
    let mut final_total = None;
    for _ in 0..40 {
        let page = host
            .list_local_session_messages(message_params_for_agent(
                &root,
                "codex",
                &session_id,
                100,
                cursor,
                revision,
            ))
            .expect("page complete process events");
        items.extend(page.content_items);
        cursor = page.next_cursor;
        revision = Some(page.source_revision);
        final_total = page.total_count;
        if !page.has_more {
            break;
        }
    }

    assert_eq!(
        items
            .iter()
            .filter(|item| item.kind == "user_message")
            .count(),
        260
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item.kind == "skill_call")
            .count(),
        260
    );
    assert_eq!(
        items.iter().filter(|item| item.kind == "tool_call").count(),
        260
    );
    assert_eq!(items.len(), 780);
    assert_eq!(final_total, Some(780));
    assert!(!items
        .iter()
        .any(|item| item.text.contains("must-not-become-a-call")));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn claude_and_pi_keep_analysis_out_of_final_agent_replies() {
    let fixtures = [
        (
            "claude-code",
            "claude",
            vec![
                json!({"type":"user","message":{"role":"user","content":[{"type":"text","text":"Claude 用户目标"}]}}).to_string(),
                json!({"type":"assistant","message":{"role":"assistant","stop_reason":"tool_use","content":[{"type":"thinking","thinking":"Claude 工具前分析"},{"type":"text","text":"Claude 过程说明"},{"type":"tool_use","name":"Read","input":{}}]}}).to_string(),
                json!({"type":"assistant","message":{"role":"assistant","stop_reason":"end_turn","content":[{"type":"thinking","thinking":"Claude 最终整理"},{"type":"text","text":"Claude 最终回复"}]}}).to_string(),
            ],
            "Claude 最终回复",
            ["Claude 工具前分析", "Claude 过程说明", "Claude 最终整理"],
        ),
        (
            "pi",
            "pi",
            vec![
                json!({"type":"message","message":{"role":"user","content":[{"type":"text","text":"Pi 用户目标"}]}}).to_string(),
                json!({"type":"message","message":{"role":"assistant","stopReason":"toolUse","content":[{"type":"thinking","thinking":"Pi 工具前分析"},{"type":"text","text":"Pi 过程说明"},{"type":"toolCall","name":"read","arguments":{}}]}}).to_string(),
                json!({"type":"message","message":{"role":"assistant","stopReason":"stop","content":[{"type":"thinking","thinking":"Pi 最终整理"},{"type":"text","text":"Pi 最终回复"}]}}).to_string(),
            ],
            "Pi 最终回复",
            ["Pi 工具前分析", "Pi 过程说明", "Pi 最终整理"],
        ),
    ];

    for (case, agent, records, expected_final, expected_thinking) in fixtures {
        let app_data_dir = env::temp_dir().join(format!(
            "skills-copilot-{case}-message-classification-test-{}-{}",
            std::process::id(),
            unique_suffix(),
        ));
        let root = app_data_dir.join("sessions");
        fs::create_dir_all(&root).expect("create session root");
        let path = root.join(format!("{case}-transcript.jsonl"));
        fs::write(&path, format!("{}\n", records.join("\n"))).expect("write transcript");
        let canonical_path = path.canonicalize().expect("canonical session path");
        let session_id = crate::service_local_sessions::local_session_row_id(&canonical_path);
        let host = test_host(app_data_dir.clone());
        let expected_user = if agent == "pi" {
            "Pi 用户目标"
        } else {
            "Claude 用户目标"
        };

        let page = host
            .list_local_session_messages(message_params_for_agent(
                &root,
                agent,
                &session_id,
                20,
                None,
                None,
            ))
            .expect("load paged messages");
        assert_eq!(
            page.content_items
                .iter()
                .map(|item| (item.kind.as_str(), item.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("user_message", expected_user),
                ("thinking", expected_thinking[0]),
                ("thinking", expected_thinking[1]),
                ("tool_call", "{}"),
                ("thinking", expected_thinking[2]),
                ("agent_reply", expected_final),
            ],
            "{case} must page process analysis as thinking and expose only the completed answer as an Agent reply"
        );

        let preview = host
            .preview_local_sessions(LocalSessionPreviewParams {
                authorized_roots: vec![root.to_string_lossy().to_string()],
                auto_discover: Some(false),
                agent: Some(agent.to_string()),
                include_content_items: Some(true),
                limit: Some(10),
                max_excerpt_chars: Some(800),
                ..LocalSessionPreviewParams::default()
            })
            .expect("preview process messages");
        let items = &preview.session_rows[0].content_items;
        for expected in expected_thinking {
            assert!(
                items
                    .iter()
                    .any(|item| item.kind == "thinking" && item.text.contains(expected)),
                "{case} must classify {expected:?} as thinking: {items:?}"
            );
        }
        assert!(
            items
                .iter()
                .any(|item| item.kind == "agent_reply" && item.text == expected_final),
            "{case} must retain the completed answer as the only Agent reply: {items:?}"
        );
        assert_eq!(
            items
                .iter()
                .filter(|item| item.kind == "agent_reply")
                .count(),
            1,
            "{case} must not label process text as another Agent reply: {items:?}"
        );
        assert!(
            items.iter().any(|item| item.kind == "tool_call"),
            "{case} must retain the tool call as its own message type: {items:?}"
        );

        let _ = fs::remove_dir_all(app_data_dir);
    }
}
