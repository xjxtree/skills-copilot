#![cfg_attr(not(unix), allow(dead_code))]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

const ACTION_SECRET_ENV: &str = "SKILLS_COPILOT_ACTION_PREVIEW_SECRET";
const ACTION_SECRET: &str = "1111111111111111111111111111111111111111111111111111111111111111";

#[test]
#[cfg(unix)]
fn durable_app_data_writes_reject_symlink_owners_without_touching_the_victim() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = unique_root("final-owner");
    let home = root.join("home");
    let app_data = root.join("app-data");
    let victim = root.join("victim");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir(&victim).expect("create victim");
    fs::set_permissions(&victim, fs::Permissions::from_mode(0o755))
        .expect("set observable victim mode");
    fs::write(victim.join("sentinel"), "unchanged").expect("seed victim");
    fs::create_dir(victim.join("keep")).expect("seed victim child");

    let project_preview = preview_project_set(&home, &app_data);
    let provider_params = json!({
        "id":"owner-safety-provider",
        "display_name":"Owner Safety Provider",
        "provider_type":"openai-compatible",
        "base_url":"https://example.invalid/v1",
        "model":"fixture-model",
        "enabled":true,
        "single_request_token_limit":4096,
        "monthly_budget_usd":3.5
    });
    let provider_preview = invoke(
        &home,
        &app_data,
        json!({
            "id":"provider-preview",
            "method":"llm.previewSaveProviderProfile",
            "params":provider_params
        }),
    );
    assert_success(&provider_preview);

    symlink(&victim, &app_data).expect("replace app-data owner with symlink");
    let expected_children = directory_names(&victim);
    let expected_mode = fs::metadata(&victim)
        .expect("victim metadata")
        .permissions()
        .mode()
        & 0o777;

    let scan = invoke(
        &home,
        &app_data,
        json!({
            "id":"scan",
            "method":"catalog.scanAll",
            "params":{"explicit_refresh":true}
        }),
    );
    assert_unsafe_owner(&scan);

    let project_apply = invoke(&home, &app_data, project_apply_request(&project_preview));
    assert_unsafe_owner(&project_apply);

    let mut provider_apply_params = provider_params;
    provider_apply_params["action_confirmation"] = action_confirmation(&provider_preview);
    let provider_apply = invoke(
        &home,
        &app_data,
        json!({
            "id":"provider-apply",
            "method":"llm.saveProviderProfile",
            "params":provider_apply_params
        }),
    );
    assert_eq!(
        provider_apply
            .pointer("/error/code")
            .and_then(Value::as_str),
        Some("action_not_started"),
        "{provider_apply}"
    );
    assert_eq!(
        provider_apply
            .pointer("/error/message")
            .and_then(Value::as_str),
        Some(
            "action did not start: provider replay state could not be replaced before the action started"
        ),
        "provider rejection must be stable and path-free: {provider_apply}"
    );

    assert_eq!(
        fs::metadata(&victim)
            .expect("victim metadata after rejected writes")
            .permissions()
            .mode()
            & 0o777,
        expected_mode,
        "rejection must not chmod a symlink target"
    );
    assert_eq!(
        fs::read_to_string(victim.join("sentinel")).expect("victim sentinel"),
        "unchanged"
    );
    assert_eq!(
        directory_names(&victim),
        expected_children,
        "catalog, project, and provider rejection must create no victim children"
    );

    let _ = fs::remove_file(&app_data);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn recursive_owner_creation_rejects_an_intermediate_symlink_without_creating_a_leaf() {
    use std::os::unix::fs::symlink;

    let root = unique_root("intermediate-owner");
    let home = root.join("home");
    let victim = root.join("victim");
    let linked_component = root.join("linked-component");
    let app_data = linked_component.join("nested/app-data");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir(&victim).expect("create victim");
    fs::write(victim.join("sentinel"), "unchanged").expect("seed victim");
    symlink(&victim, &linked_component).expect("create intermediate symlink");

    let scan = invoke(
        &home,
        &app_data,
        json!({
            "id":"scan",
            "method":"catalog.scanAll",
            "params":{"explicit_refresh":true}
        }),
    );

    assert_unsafe_owner(&scan);
    assert_eq!(
        fs::read_to_string(victim.join("sentinel")).expect("victim sentinel"),
        "unchanged"
    );
    assert!(
        !victim.join("nested").exists(),
        "an intermediate symlink must not redirect app-data initialization"
    );
    let _ = fs::remove_file(linked_component);
    let _ = fs::remove_dir_all(root);
}

fn preview_project_set(home: &Path, app_data: &Path) -> Value {
    let current = invoke(
        home,
        app_data,
        json!({"id":"project-get","method":"project.getContext","params":{}}),
    );
    assert_success(&current);
    let revision = current
        .pointer("/result/revision")
        .and_then(Value::as_str)
        .expect("project context revision");
    let preview = invoke(
        home,
        app_data,
        json!({
            "id":"project-preview",
            "method":"project.previewSetContext",
            "params":{
                "root_path":home,
                "current_cwd":home,
                "name":"Owner Safety Project",
                "expected_revision":revision
            }
        }),
    );
    assert_success(&preview);
    preview
}

fn project_apply_request(preview: &Value) -> Value {
    let candidate_last_used_at = preview
        .pointer("/result/candidate/active/last_used_at")
        .and_then(Value::as_i64)
        .expect("candidate timestamp");
    json!({
        "id":"project-apply",
        "method":"project.setContext",
        "params":{
            "root_path":preview.pointer("/result/candidate/active/root_path").expect("root"),
            "current_cwd":preview.pointer("/result/candidate/active/current_cwd").expect("cwd"),
            "name":"Owner Safety Project",
            "candidate_last_used_at":candidate_last_used_at,
            "action_confirmation":action_confirmation(preview)
        }
    })
}

fn action_confirmation(preview: &Value) -> Value {
    let action = preview.pointer("/result/action").expect("preview action");
    json!({
        "reference":{
            "action_id":action["id"],
            "source_revision":action["source_revision"],
            "project_id":action.get("project_id").cloned().unwrap_or(Value::Null),
            "target":action["target"]
        },
        "preview_token":preview.pointer("/result/preview_token").expect("preview token"),
        "confirmed":true
    })
}

fn assert_unsafe_owner(response: &Value) {
    assert_eq!(
        response.pointer("/error/code").and_then(Value::as_str),
        Some("command_error"),
        "{response}"
    );
    assert_eq!(
        response.pointer("/error/message").and_then(Value::as_str),
        Some(
            "command error: unsafe config path: mutation lock owner must be a non-symlink app data directory"
        ),
        "{response}"
    );
}

fn assert_success(response: &Value) {
    assert_eq!(response.get("ok"), Some(&Value::Bool(true)), "{response}");
}

fn invoke(home: &Path, app_data: &Path, request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_skills-copilot-service"))
        .env("SKILLS_COPILOT_HOME", home)
        .env("HOME", home)
        .env("SKILLS_COPILOT_APP_DATA_DIR", app_data)
        .env(ACTION_SECRET_ENV, ACTION_SECRET)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn service sidecar");
    child
        .stdin
        .as_mut()
        .expect("sidecar stdin")
        .write_all(
            serde_json::to_string(&request)
                .expect("serialize request")
                .as_bytes(),
        )
        .expect("write request");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait for sidecar");
    assert!(
        output.status.success(),
        "service failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse service response")
}

fn directory_names(path: &Path) -> Vec<String> {
    let mut names = fs::read_dir(path)
        .expect("read directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn unique_root(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agent-copilot-app-data-owner-{label}-{}-{suffix}",
        std::process::id()
    ))
}
