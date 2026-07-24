use super::*;

#[test]
fn local_session_preview_excludes_agent_specific_internal_sessions() {
    let unique = unique_suffix();
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-agent-internal-session-test-{}-{unique}",
        std::process::id()
    ));
    let user_home = fixture.join("home");

    for (agent, visible_title) in [
        ("claude-code", "Visible Claude task"),
        ("codex", "Visible Codex task"),
        ("opencode", "Visible OpenCode task"),
        ("pi", "Visible Pi task"),
        ("hermes", "Visible Hermes task"),
        ("openclaw", "Visible OpenClaw task"),
    ] {
        let root = fixture.join(format!("{agent}-sessions"));
        fs::create_dir_all(&root).expect("create agent session root");
        match agent {
            "claude-code" => {
                fs::write(
                    root.join("main.jsonl"),
                    json!({"type":"user","isSidechain":false,"sessionId":"claude-main","message":{"role":"user","content":visible_title}}).to_string(),
                )
                .expect("write main Claude session");
                fs::write(
                    root.join("sidechain.jsonl"),
                    json!({"type":"user","isSidechain":true,"sessionId":"claude-child","message":{"role":"user","content":"Hidden Claude sidechain"}}).to_string(),
                )
                .expect("write Claude sidechain");
            }
            "codex" => {
                fs::write(
                    root.join("rollout-main.jsonl"),
                    [
                        json!({"type":"session_meta","payload":{"id":"codex-main","source":"vscode","thread_source":"user"}}),
                        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":visible_title}]}}),
                    ]
                    .into_iter()
                    .map(|row| row.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                )
                .expect("write main Codex session");
                fs::write(
                    root.join("rollout-exec.jsonl"),
                    [
                        json!({"type":"session_meta","payload":{"id":"codex-exec","source":"exec","thread_source":"user"}}),
                        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Hidden Codex exec carrier"}]}}),
                    ]
                    .into_iter()
                    .map(|row| row.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                )
                .expect("write Codex exec carrier");
            }
            "opencode" => {
                fs::write(
                    root.join("root.json"),
                    json!({"type":"session","id":"ses_root","parentID":null,"title":visible_title})
                        .to_string(),
                )
                .expect("write root OpenCode session");
                fs::write(
                    root.join("child.json"),
                    json!({"type":"session","id":"ses_child","parentID":"ses_root","title":"Hidden OpenCode child"}).to_string(),
                )
                .expect("write child OpenCode session");
            }
            "pi" => {
                fs::write(
                    root.join("2026-07-17T10-00-00Z_root.jsonl"),
                    [
                        json!({"type":"session","id":"pi-root"}),
                        json!({"type":"message","message":{"role":"user","content":[{"type":"text","text":visible_title}]}}),
                    ]
                    .into_iter()
                    .map(|row| row.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                )
                .expect("write root Pi session");
                let child = root.join("subagent/run-id/run-0");
                fs::create_dir_all(&child).expect("create Pi subagent session directory");
                fs::write(
                    child.join("session.jsonl"),
                    json!({"type":"message","message":{"role":"user","content":[{"type":"text","text":"Hidden Pi subagent"}]}}).to_string(),
                )
                .expect("write Pi subagent session");
            }
            "hermes" => {
                fs::write(
                    root.join("cli.json"),
                    json!({"type":"session","id":"hermes-cli","source":"cli","title":visible_title}).to_string(),
                )
                .expect("write interactive Hermes session");
                for source in ["cron", "batch", "subagent"] {
                    fs::write(
                        root.join(format!("{source}.json")),
                        json!({"type":"session","id":format!("hermes-{source}"),"source":source,"title":format!("Hidden Hermes {source}")}).to_string(),
                    )
                    .expect("write internal Hermes session");
                }
            }
            "openclaw" => {
                fs::write(
                    root.join("main.json"),
                    json!({"type":"session","sessionId":"openclaw-main","sessionKey":"agent:main:main","title":visible_title}).to_string(),
                )
                .expect("write main OpenClaw session");
                for (name, key) in [
                    ("subagent", "agent:main:subagent:child"),
                    ("cron", "cron:job-id"),
                    ("hook", "hook:event-id"),
                    ("heartbeat", "agent:main:heartbeat:worker"),
                    ("acp", "agent:main:acp:editor-worker"),
                ] {
                    fs::write(
                        root.join(format!("{name}.json")),
                        json!({"type":"session","sessionId":format!("openclaw-{name}"),"sessionKey":key,"title":format!("Hidden OpenClaw {name}")}).to_string(),
                    )
                    .expect("write synthetic OpenClaw session");
                }
            }
            _ => unreachable!(),
        }

        let host = ServiceHost {
            app_data_dir: fixture.join(format!("{agent}-app-data")),
            adapter_ctx: AdapterContext {
                user_home: user_home.clone(),
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
        };
        let response = host.handle(ServiceRequest {
            id: Some(format!("session-preview-internal-{agent}")),
            method: "session.previewLocalSessions".to_string(),
            params: json!({
                "agent": agent,
                "authorized_roots": [root.to_string_lossy()],
                "auto_discover": false,
                "limit": 20,
                "max_excerpt_chars": 800
            }),
        });
        assert!(response.ok, "{agent}: {:?}", response.error);
        let result = response.result.expect("agent internal-session preview");
        assert_eq!(result["count"], json!(1), "{agent}: {result}");
        assert_eq!(result["session_rows"][0]["title"], visible_title, "{agent}");
        assert!(
            !serde_json::to_string(&result)
                .expect("serialize agent internal-session preview")
                .contains("Hidden"),
            "{agent}: {result}"
        );
    }

    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn auto_discovery_excludes_runtime_state_and_extension_session_roots() {
    let unique = unique_suffix();
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-runtime-session-root-test-{}-{unique}",
        std::process::id()
    ));
    let user_home = fixture.join("home");
    let claude_project_root = user_home.join(".claude/projects/project");
    let claude_runtime_root = user_home.join(".claude/sessions");
    let pi_root = user_home.join(".pi/agent/sessions/project");
    let pi_extension_root = user_home.join(".pi/context-mode/sessions");
    for root in [
        &claude_project_root,
        &claude_runtime_root,
        &pi_root,
        &pi_extension_root,
    ] {
        fs::create_dir_all(root).expect("create session-root fixture");
    }
    fs::write(
        claude_project_root.join("main.jsonl"),
        json!({"type":"user","sessionId":"claude-main","message":{"role":"user","content":"Visible Claude task"}}).to_string(),
    )
    .expect("write Claude conversation");
    fs::write(
        claude_runtime_root.join("runtime.json"),
        json!({"type":"session","title":"Hidden Claude runtime lock"}).to_string(),
    )
    .expect("write Claude runtime state");
    fs::write(
        pi_root.join("2026-07-17T10-00-00Z_root.jsonl"),
        json!({"type":"message","message":{"role":"user","content":[{"type":"text","text":"Visible Pi task"}]}}).to_string(),
    )
    .expect("write Pi conversation");
    fs::write(
        pi_extension_root.join("extension.json"),
        json!({"type":"session","title":"Hidden Pi extension state"}).to_string(),
    )
    .expect("write Pi extension state");

    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    for (agent, visible_title) in [
        ("claude-code", "Visible Claude task"),
        ("pi", "Visible Pi task"),
    ] {
        let response = host.handle(ServiceRequest {
            id: Some(format!("session-preview-runtime-root-{agent}")),
            method: "session.previewLocalSessions".to_string(),
            params: json!({"agent":agent,"limit":10,"max_excerpt_chars":800}),
        });
        assert!(response.ok, "{agent}: {:?}", response.error);
        let result = response.result.expect("runtime-root preview");
        assert_eq!(result["count"], json!(1), "{agent}: {result}");
        assert_eq!(result["session_rows"][0]["title"], visible_title, "{agent}");
    }

    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn openclaw_auto_discovery_ignores_legacy_jsonl_without_current_agent_database() {
    let unique = unique_suffix();
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-openclaw-auto-session-test-{}-{unique}",
        std::process::id()
    ));
    let user_home = fixture.join("home");
    let sessions = user_home.join(".openclaw/agents/main/sessions");
    fs::create_dir_all(&sessions).expect("create OpenClaw sessions directory");
    fs::write(
        sessions.join("sessions.json"),
        json!({"agent:main:main":{"sessionId":"openclaw-main","updatedAt":2000}}).to_string(),
    )
    .expect("write OpenClaw session index");
    fs::write(
        sessions.join("openclaw-main.jsonl"),
        [
            json!({"type":"session","sessionId":"openclaw-main","sessionKey":"agent:main:main"}),
            json!({"type":"message","message":{"role":"user","content":"Visible OpenClaw task"}}),
        ]
        .into_iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n"),
    )
    .expect("write OpenClaw main transcript");
    fs::write(
        sessions.join("openclaw-child.jsonl"),
        [
            json!({"type":"session","sessionId":"openclaw-child","sessionKey":"agent:main:subagent:child"}),
            json!({"type":"message","message":{"role":"user","content":"Hidden OpenClaw child"}}),
        ]
        .into_iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n"),
    )
    .expect("write OpenClaw child transcript");

    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let preview = host
        .preview_local_sessions(LocalSessionPreviewParams {
            auto_discover: Some(true),
            agent: Some("openclaw".to_string()),
            limit: Some(20),
            ..LocalSessionPreviewParams::default()
        })
        .expect("preview OpenClaw sessions");

    assert!(preview.session_rows.is_empty());
    assert!(preview.gap_notes.iter().any(|note| {
        note.contains("Legacy OpenClaw JSON/JSONL") && note.contains("not active session storage")
    }));
    let _ = fs::remove_dir_all(fixture);
}
