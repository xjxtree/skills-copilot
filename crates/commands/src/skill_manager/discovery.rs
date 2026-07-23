use super::*;
use std::io::Write;

const DISCOVERY_STATE_SCHEMA_VERSION: u32 = 1;
const MAX_DISCOVERY_STATE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryActionState {
    schema_version: u32,
    generation: u64,
    token_digest: String,
    action_id: String,
    source_revision: String,
    phase: String,
    state: String,
}

pub fn preview_search_skills_with_manager(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
) -> Result<SkillManagerSearchRecord, CommandError> {
    let preview = build_search_preview(app_data_dir, ctx, params, false)?;
    Ok(skill_manager_search_record(preview, None, Vec::new(), None))
}

pub fn apply_search_skills_with_manager(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerSearchApplyParams,
) -> Result<SkillManagerSearchRecord, CommandError> {
    let inputs = SkillManagerSearchParams {
        query: params.query.clone(),
        owner: params.owner.clone(),
        network_allowed: params.network_allowed,
    };
    let preflight = build_search_preview(app_data_dir, ctx, &inputs, true)?;
    ensure_confirmed(
        &preflight,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;

    with_manager_mutation_lock(app_data_dir, "search", || {
        let preview = build_search_preview(app_data_dir, ctx, &inputs, true)?;
        ensure_confirmed(
            &preview,
            params.confirmed,
            params.preview_token.as_deref(),
            params.action_reference.as_ref(),
        )?;
        validate_manager_preconditions(ctx, &preview)?;

        let token = params.preview_token.as_deref().ok_or_else(|| {
            CommandError::ActionConfirmationRequired(
                "skill manager search requires a fresh preview_token".to_string(),
            )
        })?;
        let action = preview.action.as_ref().ok_or_else(|| {
            CommandError::MismatchedActionReference(
                "skill manager search preview has no typed action".to_string(),
            )
        })?;
        let reserved = reserve_discovery_action(app_data_dir, token, action)?;

        let execution = match run_previewed_command(ctx, &preview) {
            Ok(execution) => execution,
            Err(error) => {
                let state_recorded =
                    finish_discovery_action(app_data_dir, &reserved, "outcome", "partial").is_ok();
                return Err(search_outcome_unknown(ctx, &error, state_recorded));
            }
        };
        let verified = (|| {
            let results = parse_search_results(&execution.machine_stdout);
            let result_json = serde_json::to_string(&results)?;
            let result_revision = action_source_revision(
                "manager.search.readback",
                &[
                    ("action_id", &action.id),
                    ("source_revision", &action.source_revision),
                    ("results", &result_json),
                ],
            )?;
            let readback = ActionReadbackRecord::verified(
                action,
                vec![ActionReadbackObservation {
                    domain: ActionReadbackDomain::ManagerInventory,
                    target_id: action.target.id.clone(),
                    revision: result_revision,
                }],
            )?;
            Ok::<_, CommandError>((results, readback))
        })();
        let (results, readback) = match verified {
            Ok(verified) => verified,
            Err(error) => {
                let state_recorded =
                    finish_discovery_action(app_data_dir, &reserved, "outcome", "partial").is_ok();
                return Err(search_outcome_unknown(ctx, &error, state_recorded));
            }
        };
        finish_discovery_action(app_data_dir, &reserved, "outcome", "verified").map_err(|_| {
            CommandError::PartialEffect {
                operation: "skillManager.applySearch".to_string(),
                state: "applied_unverified",
                cleanup_required: true,
                detail:
                    "search completed but its one-time action outcome could not be durably recorded"
                        .to_string(),
            }
        })?;
        Ok(skill_manager_search_record(
            preview,
            Some(execution.output.without_machine_stdout()),
            results,
            Some(readback),
        ))
    })
}

fn search_outcome_unknown(
    ctx: &AdapterContext,
    error: &CommandError,
    state_recorded: bool,
) -> CommandError {
    let detail = if state_recorded {
        format!("the one-time search action was consumed and cannot be retried safely: {error}")
    } else {
        format!(
            "the one-time search action outcome and replay state are unknown; do not retry: {error}"
        )
    };
    CommandError::PartialEffect {
        operation: "skillManager.applySearch".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail: redact_command_output(ctx, &detail),
    }
}

pub fn list_installed_skills_from_projection(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerListInstalledParams,
) -> Result<SkillManagerInstalledListRecord, CommandError> {
    let normalized_scope =
        normalize_manager_scope(params.scope.as_deref())?.unwrap_or_else(|| "project".to_string());
    let scope = if normalized_scope == "global" {
        Scope::AgentGlobal
    } else {
        Scope::AgentProject
    };
    let records = catalog.list_skill_records_for_project_context(ctx.project_root.as_deref())?;
    let (lock, lock_revision) = read_manager_lock_projection(ctx, Some(&normalized_scope))?;
    let mut installed = Vec::new();
    if let Some(lock) = lock {
        for (name, entry) in lock.skills {
            let mut matching = records
                .iter()
                .filter(|record| {
                    record.state != "missing"
                        && record.scope == scope.as_str()
                        && record.name.eq_ignore_ascii_case(&name)
                        && manager_agent_alias(&record.agent).is_ok()
                })
                .collect::<Vec<_>>();
            matching.sort_by(|left, right| left.id.cmp(&right.id));
            let mut agents = matching
                .iter()
                .filter_map(|record| manager_agent_alias(&record.agent).ok())
                .collect::<Vec<_>>();
            agents.sort();
            agents.dedup();
            let path = matching.first().map(|record| {
                let path = if record
                    .display_path
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
                {
                    record
                        .display_path
                        .parent()
                        .unwrap_or(&record.display_path)
                        .to_path_buf()
                } else {
                    record.display_path.clone()
                };
                redact_command_output(ctx, &path.to_string_lossy())
            });
            let source = entry
                .source
                .as_deref()
                .map(|value| redact_command_output(ctx, value))
                .or_else(|| path.clone());
            let source_kind = if entry
                .source_type
                .as_deref()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("local"))
                || entry.source.as_deref().is_some_and(manager_source_is_local)
            {
                "local"
            } else {
                "manager"
            };
            installed.push(SkillManagerInstalledRecord {
                name,
                source,
                source_kind: source_kind.to_string(),
                agents,
                scope: Some(normalized_scope.clone()),
                path,
                raw: Value::Null,
            });
        }
    }
    installed.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.source.cmp(&right.source))
    });

    let catalog_projection = records
        .iter()
        .filter(|record| record.state != "missing" && record.scope == scope.as_str())
        .map(|record| {
            (
                record.id.as_str(),
                record.agent.as_str(),
                record.name.as_str(),
                record.path.to_string_lossy().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let catalog_json = serde_json::to_string(&catalog_projection)?;
    let installed_json = serde_json::to_string(&installed)?;
    let source_revision = action_source_revision(
        "manager.installed.projection",
        &[
            ("scope", &normalized_scope),
            ("lock_revision", &lock_revision),
            ("catalog", &catalog_json),
            ("installed", &installed_json),
        ],
    )?;
    let cwd = manager_cwd(ctx, Some(&normalized_scope))?;
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: "catalog-projection".to_string(),
        operation: "listInstalled".to_string(),
        command: Vec::new(),
        cwd: redact_command_output(ctx, &cwd.to_string_lossy()),
        env: Vec::new(),
        requires_confirmation: false,
        confirmed: false,
        network_required: false,
        network_allowed: false,
        will_run: false,
        preview_token: source_revision.clone(),
        summary: "List installed skills from the accepted catalog and manager lock projection."
            .to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };
    let output = SkillManagerCommandOutput {
        status: "cached".to_string(),
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
    };
    Ok(skill_manager_installed_record(
        preview,
        output,
        installed,
        source_revision,
    ))
}

fn build_search_preview(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
    confirmed: bool,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let query = params.query.trim();
    if query.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager.search requires a non-empty query".to_string(),
        ));
    }
    validate_discovery_input("query", query, MAX_MANAGER_SEARCH_QUERY_BYTES)?;
    let owner = params
        .owner
        .as_deref()
        .map(str::trim)
        .filter(|owner| !owner.is_empty());
    if let Some(owner) = owner {
        validate_discovery_input("owner", owner, MAX_MANAGER_SEARCH_OWNER_BYTES)?;
    }
    if !params.network_allowed {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager.search preview requires network_allowed=true so the confirmation shows the actual network posture"
                .to_string(),
        ));
    }
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "find".to_string(),
        query.to_string(),
    ];
    if let Some(owner) = owner {
        args.push("--owner".to_string());
        args.push(owner.to_string());
    }
    let accepted_revision = discovery_state_revision(app_data_dir)?;
    let cwd = search_manager_cwd(ctx)?;
    command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "search",
            args,
            cwd,
            network_required: true,
            network_allowed: true,
            confirmed,
            summary: "Search remote skill indexes with npx skills.".to_string(),
            risks: vec![
                "Search contacts the external manager's configured index and may contact npm or git-host metadata."
                    .to_string(),
            ],
            source: None,
            skills: Vec::new(),
            accepted_revision: Some(accepted_revision),
        },
    )
}

fn search_manager_cwd(ctx: &AdapterContext) -> Result<PathBuf, CommandError> {
    let cwd = manager_cwd(ctx, None)?;
    let metadata = fs::symlink_metadata(&cwd).map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "skillManager.search working directory is unavailable".to_string(),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "skillManager.search working directory must be an existing non-symlink directory"
                .to_string(),
        ));
    }
    cwd.canonicalize().map_err(Into::into)
}

fn validate_discovery_input(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), CommandError> {
    if value.len() > max_bytes {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "skillManager.search {field} exceeds the {max_bytes}-byte safety limit"
        )));
    }
    if value.starts_with('-') {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "skillManager.search {field} cannot begin with an option prefix"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "skillManager.search {field} contains an invalid control character"
        )));
    }
    Ok(())
}

fn discovery_state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("skill-manager-discovery-state.json")
}

fn discovery_state_revision(app_data_dir: &Path) -> Result<String, CommandError> {
    let path = discovery_state_path(app_data_dir);
    let bytes = read_discovery_state_bytes(&path)?;
    action_source_revision(
        "manager.discovery.replay-state",
        &[(
            "state",
            &bytes
                .as_deref()
                .map(|value| format!("{:x}", Sha256::digest(value)))
                .unwrap_or_else(|| "missing".to_string()),
        )],
    )
}

fn read_discovery_state_bytes(path: &Path) -> Result<Option<Vec<u8>>, CommandError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_DISCOVERY_STATE_BYTES
    {
        return Err(CommandError::UnsafeConfigPath(
            "skill manager discovery replay state is not a bounded regular file".to_string(),
        ));
    }
    let bytes = fs::read(path)?;
    let state: DiscoveryActionState = serde_json::from_slice(&bytes).map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "skill manager discovery replay state is invalid".to_string(),
        )
    })?;
    if state.schema_version != DISCOVERY_STATE_SCHEMA_VERSION
        || state.token_digest.trim().is_empty()
        || state.action_id.trim().is_empty()
        || state.source_revision.trim().is_empty()
    {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager discovery replay state is invalid".to_string(),
        ));
    }
    Ok(Some(bytes))
}

fn reserve_discovery_action(
    app_data_dir: &Path,
    preview_token: &str,
    action: &ActionDescriptor,
) -> Result<DiscoveryActionState, CommandError> {
    let path = discovery_state_path(app_data_dir);
    let current = read_discovery_state_bytes(&path)?
        .map(|bytes| serde_json::from_slice::<DiscoveryActionState>(&bytes))
        .transpose()
        .map_err(|_| {
            CommandError::InvalidSkillManagerRequest(
                "skill manager discovery replay state is invalid".to_string(),
            )
        })?;
    let token_digest = format!("{:x}", Sha256::digest(preview_token.as_bytes()));
    if current
        .as_ref()
        .is_some_and(|state| state.token_digest == token_digest)
    {
        return Err(CommandError::StaleActionReference);
    }
    let state = DiscoveryActionState {
        schema_version: DISCOVERY_STATE_SCHEMA_VERSION,
        generation: current
            .as_ref()
            .map_or(1, |state| state.generation.saturating_add(1)),
        token_digest,
        action_id: action.id.clone(),
        source_revision: action.source_revision.clone(),
        phase: "reservation".to_string(),
        state: "not_started".to_string(),
    };
    write_discovery_state(&path, &state)?;
    Ok(state)
}

fn finish_discovery_action(
    app_data_dir: &Path,
    reserved: &DiscoveryActionState,
    phase: &str,
    state: &str,
) -> Result<(), CommandError> {
    let path = discovery_state_path(app_data_dir);
    let current = read_discovery_state_bytes(&path)?
        .map(|bytes| serde_json::from_slice::<DiscoveryActionState>(&bytes))
        .transpose()
        .map_err(|_| {
            CommandError::InvalidSkillManagerRequest(
                "skill manager discovery replay state is invalid".to_string(),
            )
        })?
        .ok_or(CommandError::StaleActionReference)?;
    if current.generation != reserved.generation
        || current.token_digest != reserved.token_digest
        || current.action_id != reserved.action_id
    {
        return Err(CommandError::StaleActionReference);
    }
    write_discovery_state(
        &path,
        &DiscoveryActionState {
            phase: phase.to_string(),
            state: state.to_string(),
            ..reserved.clone()
        },
    )
}

fn write_discovery_state(path: &Path, state: &DiscoveryActionState) -> Result<(), CommandError> {
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath(
            "skill manager discovery state has no owner directory".to_string(),
        )
    })?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "skill manager discovery state owner is invalid".to_string(),
        ));
    }
    let bytes = serde_json::to_vec(state)?;
    if bytes.len() as u64 > MAX_DISCOVERY_STATE_BYTES {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager discovery replay state exceeded its safety limit".to_string(),
        ));
    }
    let temp = parent.join(format!(
        ".skill-manager-discovery-state.{}.{}.tmp",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok::<(), CommandError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn read_manager_lock_projection(
    ctx: &AdapterContext,
    scope: Option<&str>,
) -> Result<(Option<ManagerLockFile>, String), CommandError> {
    let normalized_scope = normalize_manager_scope(scope)?;
    let path = if normalized_scope.as_deref() == Some("global") {
        ctx.user_home.join(".agents/.skill-lock.json")
    } else {
        manager_cwd(ctx, scope)?.join("skills-lock.json")
    };
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let revision =
                action_source_revision("manager.lock.projection", &[("state", "missing")])?;
            return Ok((None, revision));
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_MANAGER_LOCK_BYTES
    {
        return Err(CommandError::InvalidSkillManagerRequest(
            "manager lock projection is not a bounded regular file".to_string(),
        ));
    }
    let bytes = fs::read(path)?;
    let lock = serde_json::from_slice::<ManagerLockFile>(&bytes).map_err(|_| {
        CommandError::InvalidSkillManagerRequest("manager lock projection is invalid".to_string())
    })?;
    let revision = action_source_revision(
        "manager.lock.projection",
        &[("bytes", &format!("{:x}", Sha256::digest(&bytes)))],
    )?;
    Ok((Some(lock), revision))
}
