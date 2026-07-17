use std::{
    env,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use skills_copilot_adapters::codex_home_dir;
use skills_copilot_core::{
    AdapterContext, AgentId, PermissionRequest, Scope, SkillInstance, SkillState,
};
use skills_copilot_scanner::{ScanIssue, ScanIssueKind, ScanReport};

const CODEX_RUNTIME_TIMEOUT: Duration = Duration::from_secs(6);
const CODEX_RUNTIME_MAX_STDOUT_BYTES: usize = 8 * 1024 * 1024;
const CODEX_RUNTIME_ROOT: &str = ".agent-copilot-runtime";

#[derive(Debug, Deserialize)]
struct RpcEnvelope {
    id: Option<u64>,
    result: Option<CodexSkillsResult>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CodexSkillsResult {
    #[serde(default)]
    data: Vec<CodexSkillsForCwd>,
}

#[derive(Debug, Deserialize)]
struct CodexSkillsForCwd {
    #[serde(default)]
    skills: Vec<CodexRuntimeSkill>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct CodexRuntimeSkill {
    name: String,
    #[serde(default)]
    description: String,
    path: PathBuf,
    scope: String,
    enabled: bool,
}

pub(super) fn merge_codex_runtime_inventory(ctx: &AdapterContext, report: &mut ScanReport) {
    if !should_query_runtime(ctx) {
        return;
    }

    let runtime_root = codex_home_dir(ctx).join(CODEX_RUNTIME_ROOT);
    match query_codex_runtime(ctx) {
        Ok(result) => {
            let mut had_errors = false;
            let mut skills = Vec::new();
            for row in result.data {
                had_errors |= !row.errors.is_empty();
                skills.extend(row.skills);
            }
            merge_runtime_skills(ctx, report, skills);
            if had_errors {
                report.partial_roots.push(runtime_root.clone());
                report.issues.push(ScanIssue {
                    path: runtime_root,
                    kind: ScanIssueKind::EntryUnreadable,
                    detail: "Codex reported one or more runtime skill loading errors; current results were retained without an agent-wide missing sweep".to_string(),
                });
            }
        }
        Err(detail) => {
            report.skipped_roots.push(runtime_root.clone());
            report.issues.push(ScanIssue {
                path: runtime_root,
                kind: ScanIssueKind::RootUnavailable,
                detail,
            });
        }
    }
}

fn should_query_runtime(ctx: &AdapterContext) -> bool {
    if env_flag("SKILLS_COPILOT_DISABLE_CODEX_RUNTIME") {
        return false;
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .is_some_and(|home| same_lexical_path(&home, &ctx.user_home))
}

fn same_lexical_path(left: &Path, right: &Path) -> bool {
    left.canonicalize().unwrap_or_else(|_| left.to_path_buf())
        == right.canonicalize().unwrap_or_else(|_| right.to_path_buf())
}

fn env_flag(name: &str) -> bool {
    env::var(name).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn query_codex_runtime(ctx: &AdapterContext) -> Result<CodexSkillsResult, String> {
    let cwd = ctx
        .project_cwd
        .as_deref()
        .or(ctx.project_root.as_deref())
        .unwrap_or(&ctx.user_home);
    let mut child = Command::new("codex")
        .arg("app-server")
        .arg("--stdio")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Codex runtime skill inventory is unavailable: {error}"))?;

    let stdout = child.stdout.take().ok_or_else(|| {
        stop_child(&mut child);
        "Codex runtime skill inventory did not expose stdout".to_string()
    })?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut total = 0usize;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(read) => {
                    total = total.saturating_add(read);
                    if total > CODEX_RUNTIME_MAX_STDOUT_BYTES {
                        let _ = sender.send(Err(
                            "Codex runtime skill inventory exceeded its bounded response size"
                                .to_string(),
                        ));
                        break;
                    }
                    if sender.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(format!(
                        "Codex runtime skill inventory could not be read: {error}"
                    )));
                    break;
                }
            }
        }
    });

    let mut stdin = child.stdin.take().ok_or_else(|| {
        stop_child(&mut child);
        "Codex runtime skill inventory did not expose stdin".to_string()
    })?;
    write_rpc(
        &mut stdin,
        &serde_json::json!({
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "agent-copilot",
                    "title": "Agent Copilot",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )?;

    let deadline = Instant::now() + CODEX_RUNTIME_TIMEOUT;
    wait_for_response(&receiver, 1, deadline)?;
    write_rpc(
        &mut stdin,
        &serde_json::json!({"method": "initialized", "params": {}}),
    )?;
    write_rpc(
        &mut stdin,
        &serde_json::json!({
            "id": 2,
            "method": "skills/list",
            "params": {"cwds": [cwd], "forceReload": true}
        }),
    )?;

    let response = wait_for_response(&receiver, 2, deadline);
    drop(stdin);
    stop_child(&mut child);
    let response = response?;
    if response.error.is_some() {
        return Err("Codex rejected the runtime skills/list request".to_string());
    }
    response
        .result
        .ok_or_else(|| "Codex returned an empty runtime skills/list response".to_string())
}

fn write_rpc(stdin: &mut impl Write, value: &serde_json::Value) -> Result<(), String> {
    serde_json::to_writer(&mut *stdin, value)
        .map_err(|error| format!("Codex runtime request could not be encoded: {error}"))?;
    stdin
        .write_all(b"\n")
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("Codex runtime request could not be sent: {error}"))
}

fn wait_for_response(
    receiver: &mpsc::Receiver<Result<String, String>>,
    expected_id: u64,
    deadline: Instant,
) -> Result<RpcEnvelope, String> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Codex runtime skill inventory timed out".to_string());
        }
        let line = receiver
            .recv_timeout(remaining)
            .map_err(|_| "Codex runtime skill inventory timed out".to_string())??;
        let Ok(envelope) = serde_json::from_str::<RpcEnvelope>(&line) else {
            continue;
        };
        if envelope.id == Some(expected_id) {
            return Ok(envelope);
        }
    }
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn merge_runtime_skills(
    ctx: &AdapterContext,
    report: &mut ScanReport,
    skills: Vec<CodexRuntimeSkill>,
) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX);
    let mut merged = Vec::with_capacity(skills.len());
    for skill in skills {
        let canonical_runtime_path = skill
            .path
            .canonicalize()
            .unwrap_or_else(|_| skill.path.clone());
        if let Some(index) = report
            .instances
            .iter()
            .position(|instance| instance.path == canonical_runtime_path)
        {
            let mut instance = report.instances.swap_remove(index);
            instance.name.clone_from(&skill.name);
            instance.display_name.clone_from(&skill.name);
            instance.description.clone_from(&skill.description);
            instance.definition_id = canonical_definition_id(&skill.name);
            instance.enabled = skill.enabled;
            instance.state = if skill.enabled {
                SkillState::Loaded
            } else {
                SkillState::Disabled
            };
            instance.scope = runtime_scope(&skill.scope);
            if instance.scope == Scope::AgentProject {
                instance.project_root.clone_from(&ctx.project_root);
            }
            merged.push(instance);
        } else {
            merged.push(runtime_only_instance(ctx, skill, now));
        }
    }
    report.instances.extend(merged);
}

fn runtime_only_instance(
    ctx: &AdapterContext,
    skill: CodexRuntimeSkill,
    now: i64,
) -> SkillInstance {
    let identity = format!("{}|{}", skill.scope, skill.name);
    let digest = hex_digest(&identity);
    let path = codex_home_dir(ctx)
        .join(CODEX_RUNTIME_ROOT)
        .join(&digest[..24])
        .join("SKILL.md");
    let scope = runtime_scope(&skill.scope);
    let fingerprint = hex_digest(&format!(
        "{}\0{}\0{}",
        skill.name, skill.description, skill.enabled
    ));
    SkillInstance {
        id: format!("codex-runtime:{digest}"),
        agent: AgentId::Codex,
        scope,
        project_root: (scope == Scope::AgentProject)
            .then(|| ctx.project_root.clone())
            .flatten(),
        path,
        display_path: PathBuf::from("Codex Runtime").join(&skill.name),
        definition_id: canonical_definition_id(&skill.name),
        name: skill.name.clone(),
        display_name: skill.name,
        description: skill.description,
        version: None,
        state: if skill.enabled {
            SkillState::Loaded
        } else {
            SkillState::Disabled
        },
        enabled: skill.enabled,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint,
        mtime: now,
        first_seen: now,
        last_seen: now,
    }
}

fn runtime_scope(scope: &str) -> Scope {
    match scope.trim().to_ascii_lowercase().as_str() {
        "repo" | "project" => Scope::AgentProject,
        _ => Scope::AgentGlobal,
    }
}

fn canonical_definition_id(name: &str) -> String {
    hex_digest(&name.trim().to_ascii_lowercase())
}

fn hex_digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_codex_skills_list_shape() {
        let response: RpcEnvelope = serde_json::from_value(serde_json::json!({
            "id": 2,
            "result": {
                "data": [{
                    "cwd": "/tmp/project",
                    "skills": [{
                        "name": "plugin:review",
                        "description": "Review changes",
                        "path": "/tmp/cache/plugin/SKILL.md",
                        "scope": "user",
                        "enabled": true
                    }],
                    "errors": []
                }]
            }
        }))
        .expect("runtime response parses");
        let result = response.result.expect("result");
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].skills[0].name, "plugin:review");
    }

    #[test]
    fn runtime_only_instance_never_persists_runtime_cache_path() {
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        let instance = runtime_only_instance(
            &ctx,
            CodexRuntimeSkill {
                name: "plugin:review".to_string(),
                description: "Review changes".to_string(),
                path: PathBuf::from(
                    "/tmp/home/.codex/plugins/cache/p/review/1/skills/review/SKILL.md",
                ),
                scope: "user".to_string(),
                enabled: true,
            },
            1,
        );
        assert!(instance
            .path
            .starts_with("/tmp/home/.codex/.agent-copilot-runtime"));
        assert!(!instance.path.to_string_lossy().contains("plugins/cache"));
        assert_eq!(
            instance.display_path,
            PathBuf::from("Codex Runtime/plugin:review")
        );
    }
}
