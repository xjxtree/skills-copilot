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
            json!({
                "id":"scan",
                "method":"catalog.scanAll",
                "params":{"explicit_refresh":true}
            }),
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

    fn project_set_preview(&self) -> Value {
        let current = invoke(
            &self.home,
            &self.app_data,
            Some(SECRET_A),
            json!({"id":"project-get","method":"project.getContext","params":{}}),
        );
        assert_success(&current);
        let revision = current
            .pointer("/result/revision")
            .and_then(Value::as_str)
            .expect("project context revision");
        invoke(
            &self.home,
            &self.app_data,
            Some(SECRET_A),
            json!({
                "id":"project-preview",
                "method":"project.previewSetContext",
                "params":{
                    "root_path":self.home,
                    "current_cwd":self.home,
                    "name":"Process Project",
                    "expected_revision":revision
                }
            }),
        )
    }

    fn project_set_request(&self, preview: &Value) -> Value {
        let action = preview
            .pointer("/result/action")
            .expect("project preview action");
        let preview_token = preview
            .pointer("/result/preview_token")
            .and_then(Value::as_str)
            .expect("project preview token");
        let candidate_last_used_at = preview
            .pointer("/result/candidate/active/last_used_at")
            .and_then(Value::as_i64)
            .expect("project candidate timestamp");
        json!({
            "id":"project-apply",
            "method":"project.setContext",
            "params":{
                "root_path":self.home,
                "current_cwd":self.home,
                "name":"Process Project",
                "candidate_last_used_at":candidate_last_used_at,
                "action_confirmation":{
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
fn confirmed_manager_search_initializes_only_fresh_private_app_data() {
    use std::os::unix::fs::PermissionsExt;

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "agent-copilot-fresh-search-process-{}-{suffix}",
        std::process::id()
    ));
    let home = root.join("home");
    let app_data = root.join("app-data");
    let fake_npx = root.join("fake-npx");
    let marker = root.join("manager-search-spawned");
    fs::create_dir_all(&home).expect("create empty fresh home");
    let manager_script = format!(
        "#!/bin/sh\nprintf 'spawned\\n' >> '{}'\nprintf '[{{\"name\":\"fresh-result\",\"source\":\"fixture/fresh\",\"description\":\"Fresh filesystem result\"}}]\\n'\n",
        marker.display()
    );
    fs::write(&fake_npx, &manager_script).expect("write fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");
    let fake_npx_text = fake_npx.to_string_lossy().to_string();
    let manager_env = [("SKILLS_COPILOT_NPX_PATH", fake_npx_text.as_str())];

    assert!(!app_data.exists(), "test must begin with missing app data");
    let preview = invoke_with_extra_env(
        &home,
        &app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"fresh-search-preview",
            "method":"skillManager.search",
            "params":{
                "query":"fresh",
                "owner":"fixture-owner",
                "network_allowed":true
            }
        }),
    );
    assert_success(&preview);
    assert!(
        !app_data.exists(),
        "search preview must not create app data on a fresh filesystem"
    );
    assert!(
        !marker.exists(),
        "search preview must not start the external manager"
    );

    let action = preview
        .pointer("/result/preview/action")
        .expect("fresh search preview action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("fresh search preview token");
    let stale_apply = invoke_with_extra_env(
        &home,
        &app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"fresh-search-stale-apply",
            "method":"skillManager.applySearch",
            "params":{
                "query":"different-query",
                "owner":"fixture-owner",
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
    assert_error_code(&stale_apply, "unknown_action_reference");
    assert!(
        !app_data.exists(),
        "a stale confirmed apply must not initialize app data"
    );
    assert!(
        !marker.exists(),
        "a stale confirmed apply must not start the external manager"
    );

    fs::write(
        &fake_npx,
        "#!/bin/sh\nprintf 'changed executable must not run\\n'\n",
    )
    .expect("drift fake manager after preview");
    let stale_source = invoke_with_extra_env(
        &home,
        &app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"fresh-search-stale-source",
            "method":"skillManager.applySearch",
            "params":{
                "query":"fresh",
                "owner":"fixture-owner",
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
    assert_error_code(&stale_source, "stale_action_reference");
    assert!(
        !app_data.exists(),
        "a stale manager executable revision must not initialize app data"
    );
    assert!(
        !marker.exists(),
        "a stale manager executable revision must not start the external manager"
    );
    fs::write(&fake_npx, &manager_script).expect("restore previewed fake manager");

    let applied = invoke_with_extra_env(
        &home,
        &app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"fresh-search-apply",
            "method":"skillManager.applySearch",
            "params":{
                "query":"fresh",
                "owner":"fixture-owner",
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
    assert_success(&applied);
    assert_eq!(
        fs::read_to_string(&marker).expect("manager process marker"),
        "spawned\n",
        "confirmed apply must reach the previewed manager process exactly once"
    );
    assert_eq!(
        applied
            .pointer("/result/results/0/name")
            .and_then(Value::as_str),
        Some("fresh-result"),
        "apply must return the parsed manager result"
    );
    assert_eq!(
        applied
            .pointer("/result/readback/verified")
            .and_then(Value::as_bool),
        Some(true),
        "fresh apply must return verified manager-inventory read-back"
    );
    assert!(
        app_data
            .join("skill-manager-discovery-state.json")
            .is_file(),
        "confirmed apply must persist its bounded one-time state"
    );
    assert_eq!(
        fs::metadata(&app_data)
            .expect("fresh app-data metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700,
        "confirmed apply must create a private owner directory"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn manager_child_process_never_inherits_the_action_preview_secret() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-child-env");
    let fake_npx = fixture.root.join("fake-npx");
    let marker = fixture.root.join("manager-search-spawned");
    fs::write(
        &fake_npx,
        format!(
            "#!/bin/sh\nif printenv SKILLS_COPILOT_ACTION_PREVIEW_SECRET >/dev/null 2>&1; then\n  echo inherited-secret >&2\n  exit 42\nfi\nprintf 'spawned\\n' >> '{}'\nprintf '[{{\"name\":\"fixture-skill\",\"source\":\"fixture/source\",\"description\":\"Fixture\"}}]\\n'\n",
            marker.display()
        ),
    )
    .expect("write fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");

    let installed = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        json!({
            "id":"manager-installed",
            "method":"skillManager.listInstalled",
            "params":{"scope":"global"}
        }),
    );
    assert_success(&installed);
    assert!(
        !marker.exists(),
        "installed inventory projection must not start the external manager"
    );

    let preview = invoke_with_extra_env(
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
                "owner":"fixture-owner",
                "network_allowed":true
            }
        }),
    );
    assert_success(&preview);
    assert!(
        !marker.exists(),
        "search preview must not start the external manager"
    );
    let action = preview
        .pointer("/result/preview/action")
        .expect("search preview action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("search preview token");
    let apply_request = json!({
        "id":"manager-search-apply",
        "method":"skillManager.applySearch",
        "params":{
            "query":"fixture",
            "owner":"fixture-owner",
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
    });
    let stale_query = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        json!({
            "id":"manager-search-stale-query",
            "method":"skillManager.applySearch",
            "params":{
                "query":"different-query",
                "owner":"fixture-owner",
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
    assert_error_code(&stale_query, "unknown_action_reference");
    assert!(
        !marker.exists(),
        "a stale query must be rejected before the external manager starts"
    );
    let stale_owner = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        json!({
            "id":"manager-search-stale-owner",
            "method":"skillManager.applySearch",
            "params":{
                "query":"fixture",
                "owner":"different-owner",
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
    assert_error_code(&stale_owner, "unknown_action_reference");
    assert!(
        !marker.exists(),
        "a stale owner must be rejected before the external manager starts"
    );
    let applied = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        apply_request.clone(),
    );
    assert_success(&applied);
    assert_eq!(
        fs::read_to_string(&marker).expect("search spawn marker"),
        "spawned\n"
    );
    assert_eq!(
        applied.pointer("/result/readback/verified"),
        Some(&Value::Bool(true))
    );

    let replay = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &[(
            "SKILLS_COPILOT_NPX_PATH",
            fake_npx.to_string_lossy().as_ref(),
        )],
        apply_request,
    );
    assert_error_code(&replay, "stale_action_reference");
    assert_eq!(
        fs::read_to_string(&marker).expect("search spawn marker after replay"),
        "spawned\n",
        "a replay must be rejected before the external manager starts"
    );
}

#[test]
#[cfg(unix)]
fn unsafe_search_inputs_and_failed_external_searches_are_not_retryable() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-search-failure");
    let fake_npx = fixture.root.join("fake-npx-failure");
    let marker = fixture.root.join("manager-search-failed-spawn");
    fs::write(
        &fake_npx,
        format!(
            "#!/bin/sh\nprintf 'spawned\\n' >> '{}'\necho remote-failure >&2\nexit 7\n",
            marker.display()
        ),
    )
    .expect("write failing fake manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make failing fake manager executable");
    let fake_npx_text = fake_npx.to_string_lossy().to_string();
    let manager_env = [("SKILLS_COPILOT_NPX_PATH", fake_npx_text.as_str())];

    for (query, owner) in [
        ("--help", Some("fixture-owner")),
        ("fixture", Some("--json")),
    ] {
        let rejected = invoke_with_extra_env(
            &fixture.home,
            &fixture.app_data,
            Some(SECRET_A),
            &manager_env,
            json!({
                "id":"unsafe-search-preview",
                "method":"skillManager.search",
                "params":{
                    "query":query,
                    "owner":owner,
                    "network_allowed":true
                }
            }),
        );
        assert_error_code(&rejected, "command_error");
    }
    let network_blocked = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"network-blocked-search-preview",
            "method":"skillManager.search",
            "params":{
                "query":"fixture",
                "owner":"fixture-owner",
                "network_allowed":false
            }
        }),
    );
    assert_error_code(&network_blocked, "command_error");
    assert!(
        !marker.exists(),
        "unsafe search inputs must fail before the manager starts"
    );
    assert!(
        !fixture
            .app_data
            .join("skill-manager-discovery-state.json")
            .exists(),
        "rejected previews must not write the replay state"
    );

    let preview = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"failed-search-preview",
            "method":"skillManager.search",
            "params":{
                "query":"fixture",
                "owner":"fixture-owner",
                "network_allowed":true
            }
        }),
    );
    assert_success(&preview);
    let action = preview
        .pointer("/result/preview/action")
        .expect("failed search action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("failed search preview token");
    let apply_request = json!({
        "id":"failed-search-apply",
        "method":"skillManager.applySearch",
        "params":{
            "query":"fixture",
            "owner":"fixture-owner",
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
    });
    let failed = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        apply_request.clone(),
    );
    assert_error_code(&failed, "partial_effect");
    assert_eq!(
        failed
            .pointer("/error/details/state")
            .and_then(Value::as_str),
        Some("outcome_unknown")
    );
    assert_eq!(
        failed
            .pointer("/error/details/retry_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        fs::read_to_string(&marker).expect("failed search spawn marker"),
        "spawned\n"
    );

    let replay = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        apply_request,
    );
    assert_error_code(&replay, "stale_action_reference");
    assert_eq!(
        fs::read_to_string(&marker).expect("failed search marker after replay"),
        "spawned\n",
        "a failed external action is still one-time and must not be retried"
    );
}

#[test]
#[cfg(unix)]
fn exit_zero_unrecognized_search_output_is_non_retryable_partial_effect() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("manager-search-unrecognized-output");
    let fake_npx = fixture.root.join("fake-npx-unrecognized");
    let marker = fixture.root.join("manager-search-unrecognized-spawn");
    fs::write(
        &fake_npx,
        format!(
            "#!/bin/sh\nprintf 'spawned\\n' >> '{}'\nprintf '{{\"unexpected\":[]}}\\n'\n",
            marker.display()
        ),
    )
    .expect("write unrecognized-output manager");
    fs::set_permissions(&fake_npx, fs::Permissions::from_mode(0o700))
        .expect("make unrecognized-output manager executable");
    let fake_npx_text = fake_npx.to_string_lossy().to_string();
    let manager_env = [("SKILLS_COPILOT_NPX_PATH", fake_npx_text.as_str())];
    let preview = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        json!({
            "id":"unrecognized-search-preview",
            "method":"skillManager.search",
            "params":{
                "query":"fixture",
                "owner":"fixture-owner",
                "network_allowed":true
            }
        }),
    );
    assert_success(&preview);
    let action = preview
        .pointer("/result/preview/action")
        .expect("search action");
    let preview_token = preview
        .pointer("/result/preview/preview_token")
        .and_then(Value::as_str)
        .expect("search preview token");
    let request = json!({
        "id":"unrecognized-search-apply",
        "method":"skillManager.applySearch",
        "params":{
            "query":"fixture",
            "owner":"fixture-owner",
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
    });

    let failed = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        request.clone(),
    );
    assert_error_code(&failed, "partial_effect");
    assert_eq!(
        failed
            .pointer("/error/details/state")
            .and_then(Value::as_str),
        Some("outcome_unknown")
    );
    assert_eq!(
        failed
            .pointer("/error/details/retry_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    let replay = invoke_with_extra_env(
        &fixture.home,
        &fixture.app_data,
        Some(SECRET_A),
        &manager_env,
        request,
    );
    assert_error_code(&replay, "stale_action_reference");
    assert_eq!(
        fs::read_to_string(&marker).expect("search spawn marker"),
        "spawned\n",
        "unrecognized output must consume the one-time action without respawn"
    );
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
                "scope":"global",
                "network_allowed":true
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
            json!({
                "name":"not-created",
                "network_allowed":true
            }),
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
    {
        let catalog = rusqlite::Connection::open(fixture.app_data.join("catalog.sqlite"))
            .expect("open catalog before owner contention");
        catalog
            .pragma_update(None, "user_version", 7)
            .expect("mark catalog migration pending");
    }

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
    let catalog = rusqlite::Connection::open(fixture.app_data.join("catalog.sqlite"))
        .expect("inspect blocked catalog");
    let blocked_version: i64 = catalog
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read blocked catalog version");
    assert_eq!(
        blocked_version, 7,
        "catalog migration must wait for the same owner as the confirmed config write"
    );
    drop(catalog);

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
    let catalog = rusqlite::Connection::open(fixture.app_data.join("catalog.sqlite"))
        .expect("inspect migrated catalog");
    let applied_version: i64 = catalog
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read migrated catalog version");
    assert!(
        applied_version > 7,
        "catalog migration should complete only after the owner is released"
    );
}

#[test]
fn project_context_apply_waits_for_the_cross_process_app_mutation_owner_lock() {
    let fixture = Fixture::new("project-owner-lock");
    let preview = fixture.project_set_preview();
    assert_success(&preview);
    let request = fixture.project_set_request(&preview);
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
            .expect("poll blocked project apply")
            .is_none(),
        "project context apply must wait while another process owns the app mutation lock"
    );

    FileExt::unlock(&lock_file).expect("release app mutation owner");
    let output = child.wait_with_output().expect("wait for project apply");
    assert!(output.status.success());
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("decode project apply response");
    assert_success(&response);
    assert_eq!(
        response
            .pointer("/result/state/active/name")
            .and_then(Value::as_str),
        Some("Process Project")
    );
}

#[test]
fn catalog_scan_waits_for_the_cross_process_app_mutation_owner_lock() {
    let fixture = Fixture::new("scan-owner-lock");
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
        json!({
            "id":"scan",
            "method":"catalog.scanAll",
            "params":{"explicit_refresh":true}
        }),
    );
    thread::sleep(Duration::from_millis(150));
    assert!(
        child.try_wait().expect("poll blocked scan").is_none(),
        "catalog scan must wait while another process owns the app mutation lock"
    );

    FileExt::unlock(&lock_file).expect("release app mutation owner");
    let output = child.wait_with_output().expect("wait for scan");
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).expect("decode scan response");
    assert_success(&response);
    assert!(response
        .pointer("/result/readback/verified")
        .and_then(Value::as_bool)
        .unwrap_or(false));
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
