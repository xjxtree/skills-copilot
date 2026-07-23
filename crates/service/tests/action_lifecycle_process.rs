use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use fs4::FileExt;
use serde_json::{json, Value};

const ACTION_SECRET_ENV: &str = "SKILLS_COPILOT_ACTION_PREVIEW_SECRET";
const SECRET_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECRET_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    app_data: PathBuf,
    instance_id: String,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-action-process-{label}-{}-{suffix}",
            std::process::id()
        ));
        let home = root.join("home");
        let app_data = root.join("app-data");
        let skill_dir = home.join(".claude/skills/process-lifecycle");
        fs::create_dir_all(&skill_dir).expect("create fixture skill");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: process-lifecycle\ndescription: Process lifecycle fixture\n---\n# Fixture\n",
        )
        .expect("write fixture skill");
        let scan = invoke(
            &home,
            &app_data,
            Some(SECRET_A),
            json!({"id":"scan","method":"catalog.scanAll","params":{}}),
        );
        assert_success(&scan);
        let instance_id = scan
            .pointer("/result/skills")
            .and_then(Value::as_array)
            .and_then(|skills| {
                skills.iter().find(|skill| {
                    skill.get("agent").and_then(Value::as_str) == Some("claude-code")
                        && skill.get("name").and_then(Value::as_str) == Some("process-lifecycle")
                })
            })
            .and_then(|skill| skill.get("id"))
            .and_then(Value::as_str)
            .expect("scanned fixture instance")
            .to_string();
        Self {
            root,
            home,
            app_data,
            instance_id,
        }
    }

    fn preview(&self, secret: Option<&str>) -> Value {
        invoke(
            &self.home,
            &self.app_data,
            secret,
            json!({
                "id":"preview",
                "method":"batch.previewSkillToggles",
                "params":{
                    "instance_ids":[self.instance_id],
                    "target_enabled":false
                }
            }),
        )
    }

    fn apply(&self, secret: Option<&str>, preview: &Value) -> Value {
        let action = preview
            .pointer("/result/action")
            .expect("preview action")
            .clone();
        let preview_token = preview
            .pointer("/result/preview_token")
            .and_then(Value::as_str)
            .expect("preview token");
        invoke(
            &self.home,
            &self.app_data,
            secret,
            json!({
                "id":"apply",
                "method":"batch.applySkillToggles",
                "params":{
                    "instance_ids":[self.instance_id],
                    "target_enabled":false,
                    "confirmation":{
                        "reference":{
                            "action_id":action["id"],
                            "source_revision":action["source_revision"],
                            "project_id":action.get("project_id").cloned().unwrap_or(Value::Null),
                            "target":action["target"]
                        },
                        "preview_token":preview_token,
                        "confirmed":true
                    }
                }
            }),
        )
    }

    fn settings_path(&self) -> PathBuf {
        self.home.join(".claude/settings.json")
    }

    fn config_save_preview(&self, content: &str) -> Value {
        let current = invoke(
            &self.home,
            &self.app_data,
            Some(SECRET_A),
            json!({"id":"config-read","method":"config.readClaudeSettings","params":{}}),
        );
        assert_success(&current);
        let revision = current
            .pointer("/result/revision")
            .and_then(Value::as_str)
            .expect("current config revision");
        invoke(
            &self.home,
            &self.app_data,
            Some(SECRET_A),
            json!({
                "id":"config-preview",
                "method":"config.previewSaveClaudeSettings",
                "params":{"content":content,"expected_revision":revision}
            }),
        )
    }

    fn config_save_request(&self, content: &str, preview: &Value) -> Value {
        let action = preview
            .pointer("/result/action")
            .expect("config preview action")
            .clone();
        let preview_token = preview
            .pointer("/result/preview_token")
            .and_then(Value::as_str)
            .expect("config preview token");
        json!({
            "id":"config-apply",
            "method":"config.saveClaudeSettings",
            "params":{
                "content":content,
                "confirmation":{
                    "reference":{
                        "action_id":action["id"],
                        "source_revision":action["source_revision"],
                        "project_id":action.get("project_id").cloned().unwrap_or(Value::Null),
                        "target":action["target"]
                    },
                    "preview_token":preview_token,
                    "confirmed":true
                }
            }
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn preview_and_apply_succeed_across_two_sidecar_processes_with_one_app_secret() {
    let fixture = Fixture::new("same-secret");
    let preview = fixture.preview(Some(SECRET_A));
    assert_success(&preview);

    let applied = fixture.apply(Some(SECRET_A), &preview);

    assert_success(&applied);
    let settings = fs::read_to_string(fixture.settings_path()).expect("read applied config");
    assert!(settings.contains("process-lifecycle"));
    assert!(settings.contains("off"));
}

#[test]
fn a_different_sidecar_secret_rejects_apply_without_a_write() {
    let fixture = Fixture::new("different-secret");
    let preview = fixture.preview(Some(SECRET_A));
    assert_success(&preview);

    let applied = fixture.apply(Some(SECRET_B), &preview);

    assert_error_code(&applied, "stale_action_reference");
    assert!(
        !fixture.settings_path().exists(),
        "mismatched secret must fail before config I/O"
    );
}

#[test]
fn missing_or_invalid_sidecar_secret_fails_closed_without_a_write() {
    for (label, secret) in [("missing", None), ("invalid", Some("not-a-secret"))] {
        let fixture = Fixture::new(label);

        let preview = fixture.preview(secret);

        assert_error_code(&preview, "action_token_unavailable");
        assert!(
            !fixture.settings_path().exists(),
            "{label} secret must fail before config I/O"
        );
    }
}

#[test]
#[cfg(unix)]
fn manager_child_process_never_inherits_the_action_preview_secret() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-child-env");
    let fake_npx = fixture.root.join("fake-npx");
    fs::write(
        &fake_npx,
        "#!/bin/sh\nif printenv SKILLS_COPILOT_ACTION_PREVIEW_SECRET >/dev/null 2>&1; then\n  echo inherited-secret >&2\n  exit 42\nfi\nprintf '[]\\n'\n",
    )
    .expect("write fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");

    let response = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        json!({
            "id":"manager-search",
            "method":"skillManager.search",
            "params":{
                "query":"fixture",
                "network_allowed":true
            }
        }),
    );

    assert_success(&response);
}

#[test]
#[cfg(unix)]
fn manager_exit_zero_noop_fails_install_verification_for_a_preexisting_skill() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-noop-verification");
    let existing = fixture.home.join(".agents/skills/already-installed");
    fs::create_dir_all(&existing).expect("create preexisting manager target");
    fs::write(
        existing.join("SKILL.md"),
        "---\nname: already-installed\ndescription: Existing target\n---\n",
    )
    .expect("write preexisting manager target");
    let fake_npx = fixture.root.join("fake-npx-noop");
    fs::write(&fake_npx, "#!/bin/sh\nexit 0\n").expect("write fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");
    let fake_npx_text = fake_npx.to_string_lossy().to_string();
    let manager_env = [("SKILLS_COPILOT_NPX_PATH", fake_npx_text.as_str())];
    let preview = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"manager-preview",
            "method":"skillManager.previewInstall",
            "params":{
                "source":"owner/repository",
                "skills":["already-installed"],
                "agents":["codex"],
                "scope":"global",
                "network_allowed":true
            }
        }),
    );
    assert_success(&preview);
    let action = preview
        .pointer("/result/preview/action")
        .expect("manager action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("manager preview token");
    let applied = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"manager-apply",
            "method":"skillManager.applyInstall",
            "params":{
                "source":"owner/repository",
                "skills":["already-installed"],
                "agents":["codex"],
                "scope":"global",
                "network_allowed":true,
                "confirmed":true,
                "preview_token":preview_token,
                "action_reference":{
                    "action_id":action["id"],
                    "source_revision":action["source_revision"],
                    "project_id":action.get("project_id").cloned().unwrap_or(Value::Null),
                    "target":action["target"]
                }
            }
        }),
    );

    assert_error_code(&applied, "partial_effect");
    assert_eq!(
        applied
            .pointer("/error/details/state")
            .and_then(Value::as_str),
        Some("applied_unverified")
    );
    assert_eq!(
        applied
            .pointer("/error/details/retry_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(applied.pointer("/result/readback").is_none());
}

#[test]
#[cfg(unix)]
fn manager_exit_zero_noop_fails_remove_update_and_local_create_verification() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-noop-other-operations");
    let existing = fixture.home.join(".agents/skills/update-existing");
    fs::create_dir_all(&existing).expect("create update target");
    fs::write(
        existing.join("SKILL.md"),
        "---\nname: update-existing\ndescription: Existing update target\n---\n",
    )
    .expect("write update target");
    let fake_npx = fixture.root.join("fake-npx-noop-other");
    fs::write(&fake_npx, "#!/bin/sh\nexit 0\n").expect("write fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");
    let fake_npx_text = fake_npx.to_string_lossy().to_string();
    let manager_env = [("SKILLS_COPILOT_NPX_PATH", fake_npx_text.as_str())];

    for (label, preview_method, apply_method, params) in [
        (
            "remove",
            "skillManager.previewRemove",
            "skillManager.applyRemove",
            json!({
                "skill":"not-installed",
                "agents":["codex"],
                "scope":"global"
            }),
        ),
        (
            "update",
            "skillManager.previewUpdate",
            "skillManager.applyUpdate",
            json!({
                "skills":["update-existing"],
                "scope":"global",
                "network_allowed":true
            }),
        ),
        (
            "local-create",
            "skillManager.previewLocalCreate",
            "skillManager.applyLocalCreate",
            json!({"name":"not-created"}),
        ),
    ] {
        let applied = invoke_manager_preview_and_apply(
            &fixture,
            &manager_env,
            preview_method,
            apply_method,
            params,
        );
        assert_error_code(&applied, "partial_effect");
        assert_eq!(
            applied
                .pointer("/error/details/state")
                .and_then(Value::as_str),
            Some("applied_unverified"),
            "{label} must expose the post-start outcome state"
        );
        assert!(
            applied.pointer("/result/readback").is_none(),
            "{label} no-op must not return verified read-back"
        );
    }
}

fn invoke_manager_preview_and_apply(
    fixture: &Fixture,
    manager_env: &[(&str, &str)],
    preview_method: &str,
    apply_method: &str,
    params: Value,
) -> Value {
    let preview = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        manager_env,
        json!({"id":"manager-preview","method":preview_method,"params":params}),
    );
    assert_success(&preview);
    let action = preview
        .pointer("/result/preview/action")
        .expect("manager action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("manager preview token");
    let mut apply_params = params;
    let apply_params = apply_params
        .as_object_mut()
        .expect("manager params must be an object");
    apply_params.insert("confirmed".to_string(), Value::Bool(true));
    apply_params.insert(
        "preview_token".to_string(),
        Value::String(preview_token.to_string()),
    );
    apply_params.insert(
        "action_reference".to_string(),
        json!({
            "action_id":action["id"],
            "source_revision":action["source_revision"],
            "project_id":action.get("project_id").cloned().unwrap_or(Value::Null),
            "target":action["target"]
        }),
    );
    invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        manager_env,
        json!({"id":"manager-apply","method":apply_method,"params":apply_params}),
    )
}

#[test]
#[cfg(unix)]
fn config_save_waits_for_the_cross_process_app_mutation_owner_lock() {
    let fixture = Fixture::new("config-owner-lock");
    let content = "{\n  \"ownerLock\": true\n}\n";
    let preview = fixture.config_save_preview(content);
    assert_success(&preview);
    let request = fixture.config_save_request(content, &preview);

    let lock_file = fs::File::open(
        fixture
            .app_data
            .canonicalize()
            .expect("canonical app-data owner"),
    )
    .expect("open app-data owner");
    lock_file.lock_exclusive().expect("hold app mutation owner");
    let mut child = spawn_sidecar(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[],
        request,
    );
    thread::sleep(Duration::from_millis(150));
    assert!(
        child
            .try_wait()
            .expect("poll blocked config save")
            .is_none(),
        "config save must wait while another process owns the shared app mutation lock"
    );

    FileExt::unlock(&lock_file).expect("release app mutation owner");
    let output = child.wait_with_output().expect("wait for config save");
    assert!(
        output.status.success(),
        "sidecar failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("decode config save response");
    assert_success(&response);
    assert_eq!(
        fs::read_to_string(fixture.settings_path()).expect("read saved config"),
        content
    );
}

fn invoke(home: &Path, app_data: &Path, secret: Option<&str>, request: Value) -> Value {
    invoke_with_extra_env(home, app_data, secret, &[], request)
}

fn invoke_with_extra_env(
    home: &Path,
    app_data: &Path,
    secret: Option<&str>,
    extra_env: &[(&str, &str)],
    request: Value,
) -> Value {
    let child = spawn_sidecar(home, app_data, secret, extra_env, request);
    let output = child.wait_with_output().expect("wait for sidecar");
    assert!(
        output.status.success(),
        "sidecar failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("decode sidecar response")
}

fn spawn_sidecar(
    home: &Path,
    app_data: &Path,
    secret: Option<&str>,
    extra_env: &[(&str, &str)],
    request: Value,
) -> std::process::Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_skills-copilot-service"));
    command
        .env("HOME", home)
        .env("SKILLS_COPILOT_HOME", home)
        .env("SKILLS_COPILOT_APP_DATA_DIR", app_data)
        .env_remove(ACTION_SECRET_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(secret) = secret {
        command.env(ACTION_SECRET_ENV, secret);
    }
    command.envs(extra_env.iter().copied());
    let mut child = command.spawn().expect("spawn service sidecar");
    child
        .stdin
        .take()
        .expect("sidecar stdin")
        .write_all(request.to_string().as_bytes())
        .expect("write sidecar request");
    child
}

fn assert_success(response: &Value) {
    assert_eq!(
        response.get("ok").and_then(Value::as_bool),
        Some(true),
        "unexpected response: {response}"
    );
}

fn assert_error_code(response: &Value, expected: &str) {
    assert_eq!(
        response.pointer("/error/code").and_then(Value::as_str),
        Some(expected),
        "unexpected response: {response}"
    );
}
