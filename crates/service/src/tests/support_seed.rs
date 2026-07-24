use super::*;

pub(super) fn test_host(app_data_dir: PathBuf) -> ServiceHost {
    ServiceHost {
        app_data_dir,
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    }
}

pub(super) fn test_host_with_project(app_data_dir: PathBuf) -> ServiceHost {
    let project_root = app_data_dir
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .to_path_buf();
    ServiceHost {
        app_data_dir,
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root),
            extra_roots: Vec::new(),
        },
    }
}

pub(super) fn spawn_mock_openai_server() -> (String, std::thread::JoinHandle<String>) {
    spawn_mock_openai_server_with_responder(|request_text| {
        evidence_bound_mock_content(request_text, Some("Draft-only review from mock provider."))
            .unwrap_or_else(|| "Draft-only review from mock provider.".to_string())
    })
}

pub(super) fn spawn_mock_openai_server_with_markdown(
    markdown: impl Into<String>,
) -> (String, std::thread::JoinHandle<String>) {
    let markdown = markdown.into();
    spawn_mock_openai_server_with_responder(move |request_text| {
        evidence_bound_mock_content(request_text, Some(&markdown))
            .expect("mock LLM request must contain an evidence-bound response contract")
    })
}

pub(super) fn spawn_mock_openai_server_with_content(
    content: impl Into<String>,
) -> (String, std::thread::JoinHandle<String>) {
    let content = content.into();
    spawn_mock_openai_server_with_responder(move |_| content.clone())
}

fn spawn_mock_openai_server_with_responder(
    responder: impl Fn(&str) -> String + Send + 'static,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock provider listener");
    let port = listener
        .local_addr()
        .expect("mock provider local addr")
        .port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept mock provider request");
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 1024];
        let mut header_end = None;
        while header_end.is_none() {
            let read =
                std::io::Read::read(&mut stream, &mut buffer).expect("read mock provider headers");
            assert!(read > 0, "mock provider request closed before headers");
            bytes.extend_from_slice(&buffer[..read]);
            header_end = find_header_end(&bytes);
        }
        let header_end = header_end.expect("header end");
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let body_start = header_end + 4;
        while bytes.len().saturating_sub(body_start) < content_length {
            let read =
                std::io::Read::read(&mut stream, &mut buffer).expect("read mock provider body");
            assert!(read > 0, "mock provider request closed before body");
            bytes.extend_from_slice(&buffer[..read]);
        }
        let request_text = String::from_utf8_lossy(&bytes).to_string();
        let content = responder(&request_text);
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": content
                }
            }],
            "usage": {
                "prompt_tokens": 32,
                "completion_tokens": 8,
                "total_tokens": 40
            }
        })
        .to_string();
        let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        std::io::Write::write_all(&mut stream, response.as_bytes())
            .expect("write mock provider response");
        request_text
    });
    (format!("http://127.0.0.1:{port}/v1"), handle)
}

fn evidence_bound_mock_content(request_text: &str, markdown: Option<&str>) -> Option<String> {
    let (_, request_body) = request_text.split_once("\r\n\r\n")?;
    let request: Value = serde_json::from_str(request_body).ok()?;
    let prompt = request
        .get("messages")?
        .as_array()?
        .iter()
        .rev()
        .find_map(|message| message.get("content").and_then(Value::as_str))?;
    let contract_json = prompt
        .split_once("Evidence-bound response contract:\n")?
        .1
        .split_once("\n\nRequired output:")?
        .0;
    let specification: Value = serde_json::from_str(contract_json).ok()?;
    let mut envelope = specification.get("response_envelope")?.clone();
    let evidence_id = specification
        .pointer("/allowed_evidence/0/id")?
        .as_str()?
        .to_string();
    envelope["evidence_refs"] = json!([evidence_id]);
    envelope["action_refs"] = json!([]);
    if let Some(markdown) = markdown {
        if envelope.pointer("/result/markdown").is_some() {
            envelope["result"]["markdown"] = Value::String(markdown.to_string());
        }
    }
    serde_json::to_string(&envelope).ok()
}

pub(super) fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(super) fn seed_catalog_with_llm_skill(host: &ServiceHost, path: &Path) {
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&host.app_data_dir, fs::Permissions::from_mode(0o700))
            .expect("make seeded app data private");
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create skill parent");
    }
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");
    let instance = SkillInstance {
        id: "llm-skill-id".to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: path.to_path_buf(),
        display_path: path.to_path_buf(),
        definition_id: "llm-definition-id".to_string(),
        name: "llm-fixture".to_string(),
        display_name: "llm-fixture".to_string(),
        description: "Fixture skill for local LLM planning.".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: "name: llm-fixture\ndescription: Fixture skill\n".to_string(),
        body: "Analyze local skill posture. OPENAI_API_KEY=<redacted>".to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: "llm-fingerprint".to_string(),
        mtime: 1,
        first_seen: 1,
        last_seen: 1,
    };
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert llm fixture skill");
    catalog
            .refresh_rule_findings(&[RuleFindingDraft {
                id: "llm-finding-id".to_string(),
                instance_id: Some("llm-skill-id".to_string()),
                definition_id: Some("llm-definition-id".to_string()),
                rule_id: "permissions.exec-needs-human".to_string(),
                severity: "error".to_string(),
                message: "Execution-like behavior needs human review; sample-key=fixture-redacted-value must not leak.".to_string(),
                suggestion: Some(
                    "Keep execution disabled and require explicit human confirmation.".to_string(),
                ),
                created_at: 1,
            }])
            .expect("upsert llm fixture finding");
}

pub(super) fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos()
}

#[cfg(unix)]
pub(super) fn assert_private_path_mode(path: &Path, expected: u32) {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(path)
        .expect("private path metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode,
        expected,
        "{} should have private mode {:o}, got {:o}",
        path.display(),
        expected,
        mode
    );
}

#[cfg(not(unix))]
pub(super) fn assert_private_path_mode(_path: &Path, _expected: u32) {}

pub(super) fn write_private_app_data_fixture(path: &Path, content: impl AsRef<[u8]>) {
    let parent = path.parent().expect("private fixture parent");
    fs::create_dir_all(parent).expect("create private fixture parent");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .expect("private fixture parent mode");
    }
    fs::write(path, content).expect("write private app-data fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .expect("private fixture file mode");
    }
}

pub(super) fn provider_test_secret_env_name(profile_id: &str) -> String {
    let account = format!("provider:{profile_id}");
    let suffix = account
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("SKILLS_COPILOT_TEST_SECRET_{suffix}")
}
