use super::*;
use std::io::Read;

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
    validate_manager_preconditions(ctx, &preflight)?;

    with_search_mutation_lock(app_data_dir, |owner| {
        let preview = build_search_preview_for_owner(owner, ctx, &inputs, true)?;
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
        let reserved = reserve_discovery_action(owner, token, action)?;

        let execution = match run_previewed_command(ctx, &preview) {
            Ok(execution) => execution,
            Err(error) => {
                let state_recorded =
                    finish_discovery_action(owner, &reserved, "outcome", "partial").is_ok();
                return Err(search_outcome_unknown(ctx, &error, state_recorded));
            }
        };
        let verified = (|| {
            let results = parse_search_results(&execution.machine_stdout)?;
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
                    finish_discovery_action(owner, &reserved, "outcome", "partial").is_ok();
                return Err(search_outcome_unknown(ctx, &error, state_recorded));
            }
        };
        finish_discovery_action(owner, &reserved, "outcome", "verified").map_err(|_| {
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
    let installed =
        installed_projection_rows(ctx, &normalized_scope, scope, &records, lock.as_ref())?;
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

fn installed_projection_rows(
    ctx: &AdapterContext,
    normalized_scope: &str,
    scope: Scope,
    records: &[SkillRecord],
    lock: Option<&ManagerLockFile>,
) -> Result<Vec<SkillManagerInstalledRecord>, CommandError> {
    let guarded_root = if normalized_scope == "global" {
        ctx.user_home.join(".agents/skills")
    } else {
        manager_cwd(ctx, Some(normalized_scope))?.join(".agents/skills")
    };
    let manager_cwd = manager_cwd(ctx, Some(normalized_scope))?;
    let eligible = records
        .iter()
        .filter(|record| {
            record.state != "missing"
                && record.scope == scope.as_str()
                && manager_agent_alias(&record.agent).is_ok()
                && record.read_only_reason.is_none()
                && !record.source_kind.as_deref().is_some_and(|kind| {
                    let kind = kind.to_ascii_lowercase();
                    kind.contains("plugin") || kind.contains("cache") || kind == "codex-runtime"
                })
        })
        .collect::<Vec<_>>();
    let mut installed = Vec::new();
    let mut locked_names = BTreeSet::new();

    if let Some(lock) = lock {
        for (name, entry) in &lock.skills {
            validate_manager_lock_entry(name, entry, &manager_cwd)?;
            locked_names.insert(name.to_ascii_lowercase());

            let anchors = eligible
                .iter()
                .copied()
                .filter(|record| {
                    record.name.eq_ignore_ascii_case(name)
                        && skill_directory(&record.display_path)
                            .is_some_and(|path| path.starts_with(&guarded_root))
                })
                .collect::<Vec<_>>();
            let source_roots = anchors
                .iter()
                .filter_map(|record| skill_directory(&record.path))
                .collect::<BTreeSet<_>>();
            let source_root = if source_roots.len() == 1 {
                source_roots.iter().next()
            } else {
                None
            };
            let mut matching = source_root
                .map(|source_root| {
                    eligible
                        .iter()
                        .copied()
                        .filter(|record| {
                            record.name.eq_ignore_ascii_case(name)
                                && skill_directory(&record.path).as_ref() == Some(source_root)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            matching.sort_by(|left, right| left.id.cmp(&right.id));
            let mut agents = matching
                .iter()
                .filter_map(|record| manager_agent_alias(&record.agent).ok())
                .collect::<Vec<_>>();
            agents.sort();
            agents.dedup();
            let path = anchors
                .iter()
                .filter_map(|record| skill_directory(&record.display_path))
                .min()
                .map(|path| redact_command_output(ctx, &path.to_string_lossy()));
            installed.push(SkillManagerInstalledRecord {
                name: name.clone(),
                source: entry
                    .source
                    .as_deref()
                    .map(|value| redact_command_output(ctx, value)),
                source_kind: "manager".to_string(),
                agents,
                scope: Some(normalized_scope.to_string()),
                path,
                raw: Value::Null,
            });
        }
    }

    let mut local_sources = BTreeMap::<PathBuf, Vec<&SkillRecord>>::new();
    for record in eligible {
        if locked_names.contains(&record.name.to_ascii_lowercase()) {
            continue;
        }
        let Some(source_dir) = skill_directory(&record.path) else {
            continue;
        };
        if !source_dir.starts_with(&guarded_root) {
            continue;
        }
        local_sources.entry(source_dir).or_default().push(record);
    }
    for (source_dir, mut matching) in local_sources {
        matching.sort_by(|left, right| left.id.cmp(&right.id));
        let Some(representative) = matching.first() else {
            continue;
        };
        let mut agents = matching
            .iter()
            .filter_map(|record| manager_agent_alias(&record.agent).ok())
            .collect::<Vec<_>>();
        agents.sort();
        agents.dedup();
        let path = redact_command_output(ctx, &source_dir.to_string_lossy());
        installed.push(SkillManagerInstalledRecord {
            name: representative.name.clone(),
            source: Some(path.clone()),
            source_kind: "local".to_string(),
            agents,
            scope: Some(normalized_scope.to_string()),
            path: Some(path),
            raw: Value::Null,
        });
    }

    installed.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.source_kind.cmp(&right.source_kind))
            .then_with(|| left.source.cmp(&right.source))
    });
    Ok(installed)
}

fn skill_directory(path: &Path) -> Option<PathBuf> {
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
    {
        path.parent().map(Path::to_path_buf)
    } else {
        Some(path.to_path_buf())
    }
}

fn validate_manager_lock_entry(
    name: &str,
    entry: &ManagerLockEntry,
    manager_cwd: &Path,
) -> Result<(), CommandError> {
    validate_discovery_input(
        "manager lock skill name",
        name,
        MAX_MANAGER_SEARCH_QUERY_BYTES,
    )?;
    let source = entry.source.as_deref().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "manager lock entry is missing its source identity".to_string(),
        )
    })?;
    validate_discovery_input(
        "manager lock source",
        source,
        MAX_MANAGER_SEARCH_QUERY_BYTES,
    )?;
    let source_resolution = resolve_manager_source(source, manager_cwd)?;
    let source_type = entry.source_type.as_deref().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "manager lock entry is missing its source type".to_string(),
        )
    })?;
    validate_discovery_input(
        "manager lock source type",
        source_type,
        MAX_MANAGER_SEARCH_OWNER_BYTES,
    )?;
    let declares_local = source_type.eq_ignore_ascii_case("local");
    let explicit_local_source = source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('~')
        || source.starts_with("file://");
    let mismatched_source_type = if declares_local {
        !matches!(source_resolution, ManagerSourceResolution::Local(_))
    } else {
        explicit_local_source
    };
    if mismatched_source_type {
        return Err(CommandError::InvalidSkillManagerRequest(
            "manager lock source type does not match its source identity".to_string(),
        ));
    }
    if let Some(skill_path) = entry.skill_path.as_deref() {
        validate_manager_lock_skill_path(skill_path)?;
    }
    Ok(())
}

fn validate_manager_lock_skill_path(skill_path: &str) -> Result<(), CommandError> {
    validate_discovery_input(
        "manager lock skill path",
        skill_path,
        MAX_MANAGER_SEARCH_QUERY_BYTES,
    )?;
    let path = Path::new(skill_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(CommandError::InvalidSkillManagerRequest(
            "manager lock skill path must be a relative normalized package path".to_string(),
        ));
    }
    Ok(())
}

fn build_search_preview(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
    confirmed: bool,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let accepted_revision = discovery_state_revision(app_data_dir)?;
    build_search_preview_with_revision(ctx, params, confirmed, accepted_revision)
}

fn build_search_preview_for_owner(
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
    confirmed: bool,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let accepted_revision = discovery_state_revision_for_owner(owner)?;
    build_search_preview_with_revision(ctx, params, confirmed, accepted_revision)
}

fn build_search_preview_with_revision(
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
    confirmed: bool,
    accepted_revision: String,
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

fn discovery_state_relative_path() -> &'static Path {
    Path::new("skill-manager-discovery-state.json")
}

fn discovery_state_revision(app_data_dir: &Path) -> Result<String, CommandError> {
    let bytes = if crate::mutation_lock::app_mutation_owner_is_missing(app_data_dir)? {
        None
    } else {
        let lock = crate::mutation_lock::lock_app_mutations(app_data_dir)?;
        lock.validate_owner_path_binding()?;
        let owner = lock.owner_fs();
        read_discovery_state_bytes(&owner)?
    };
    discovery_state_revision_from_bytes(bytes.as_deref())
}

fn discovery_state_revision_for_owner(
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
) -> Result<String, CommandError> {
    let bytes = read_discovery_state_bytes(owner)?;
    discovery_state_revision_from_bytes(bytes.as_deref())
}

fn discovery_state_revision_from_bytes(bytes: Option<&[u8]>) -> Result<String, CommandError> {
    action_source_revision(
        "manager.discovery.replay-state",
        &[(
            "state",
            &bytes
                .map(|value| format!("{:x}", Sha256::digest(value)))
                .unwrap_or_else(|| "missing".to_string()),
        )],
    )
}

fn read_discovery_state_bytes(
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
) -> Result<Option<Vec<u8>>, CommandError> {
    let Some(bytes) = owner.read_bounded_regular_file(
        discovery_state_relative_path(),
        MAX_DISCOVERY_STATE_BYTES,
        "skill manager discovery replay state",
    )?
    else {
        return Ok(None);
    };
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
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
    preview_token: &str,
    action: &ActionDescriptor,
) -> Result<DiscoveryActionState, CommandError> {
    let current = read_discovery_state_bytes(owner)?
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
    write_discovery_state(owner, &state)?;
    Ok(state)
}

fn finish_discovery_action(
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
    reserved: &DiscoveryActionState,
    phase: &str,
    state: &str,
) -> Result<(), CommandError> {
    let current = read_discovery_state_bytes(owner)?
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
        owner,
        &DiscoveryActionState {
            phase: phase.to_string(),
            state: state.to_string(),
            ..reserved.clone()
        },
    )
}

fn write_discovery_state(
    owner: &crate::app_data_owner_fs::AppDataOwnerFs<'_>,
    state: &DiscoveryActionState,
) -> Result<(), CommandError> {
    let bytes = serde_json::to_vec(state)?;
    if bytes.len() as u64 > MAX_DISCOVERY_STATE_BYTES {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager discovery replay state exceeded its safety limit".to_string(),
        ));
    }
    owner.remove_root_regular_files_matching(".skill-manager-discovery-state.", ".tmp", 64)?;
    owner.atomic_replace_private_file(
        discovery_state_relative_path(),
        &bytes,
        "skill-manager-discovery-state",
    )
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
    let bytes = match read_bounded_regular_file_no_follow(
        &path,
        MAX_MANAGER_LOCK_BYTES,
        "manager lock projection",
    )? {
        Some(bytes) => bytes,
        None => {
            let revision =
                action_source_revision("manager.lock.projection", &[("state", "missing")])?;
            return Ok((None, revision));
        }
    };
    let lock = serde_json::from_slice::<ManagerLockFile>(&bytes).map_err(|_| {
        CommandError::InvalidSkillManagerRequest("manager lock projection is invalid".to_string())
    })?;
    let revision = action_source_revision(
        "manager.lock.projection",
        &[("bytes", &format!("{:x}", Sha256::digest(&bytes)))],
    )?;
    Ok((Some(lock), revision))
}

fn read_bounded_regular_file_no_follow(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Option<Vec<u8>>, CommandError> {
    let mut file = match open_projection_file_no_follow(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{label} is not a bounded regular file"
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{label} is not a bounded regular file"
        )));
    }
    Ok(Some(bytes))
}

#[cfg(unix)]
fn open_projection_file_no_follow(path: &Path) -> std::io::Result<File> {
    use rustix::fs::{open, Mode, OFlags};

    let flags =
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
    open(path, flags, Mode::empty())
        .map(File::from)
        .map_err(std::io::Error::from)
}

#[cfg(not(unix))]
fn open_projection_file_no_follow(path: &Path) -> std::io::Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path is a symlink",
        ));
    }
    File::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        id: &str,
        agent: &str,
        name: &str,
        path: &str,
        display_path: &str,
        source_kind: Option<&str>,
        read_only_reason: Option<&str>,
    ) -> SkillRecord {
        SkillRecord {
            id: id.to_string(),
            agent: agent.to_string(),
            scope: Scope::AgentProject.as_str().to_string(),
            path: PathBuf::from(path),
            display_path: PathBuf::from(display_path),
            definition_id: format!("definition:{id}"),
            name: name.to_string(),
            state: "loaded".to_string(),
            enabled: true,
            publisher: None,
            package_name: None,
            package_version: None,
            source_kind: source_kind.map(str::to_string),
            read_only_reason: read_only_reason.map(str::to_string),
        }
    }

    fn context() -> AdapterContext {
        AdapterContext {
            user_home: PathBuf::from("/tmp/manager-projection-home"),
            project_root: Some(PathBuf::from("/tmp/manager-projection-project")),
            project_cwd: Some(PathBuf::from("/tmp/manager-projection-project")),
            extra_roots: Vec::new(),
        }
    }

    #[test]
    fn lock_projection_matches_one_guarded_source_and_excludes_same_name_plugin_rows() {
        let ctx = context();
        let records = vec![
            record(
                "codex-managed",
                AgentId::Codex.as_str(),
                "shared",
                "/tmp/manager-source/shared/SKILL.md",
                "/tmp/manager-projection-project/.agents/skills/shared/SKILL.md",
                None,
                None,
            ),
            record(
                "claude-managed",
                AgentId::ClaudeCode.as_str(),
                "shared",
                "/tmp/manager-source/shared/SKILL.md",
                "/tmp/manager-projection-project/.claude/skills/shared/SKILL.md",
                None,
                None,
            ),
            record(
                "plugin-collision",
                AgentId::Codex.as_str(),
                "shared",
                "/tmp/manager-projection-home/.codex/plugins/cache/vendor/package/1.0.0/skills/shared/SKILL.md",
                "$CODEX_HOME/plugins/package@vendor/skills/shared/SKILL.md",
                Some("chatgpt-plugin-cache"),
                Some("Installed Codex plugin files are read-only"),
            ),
            record(
                "configured-collision",
                AgentId::Pi.as_str(),
                "shared",
                "/tmp/configured-read-only/shared/SKILL.md",
                "/tmp/configured-read-only/shared/SKILL.md",
                None,
                Some("Configured roots are read-only manager inventory"),
            ),
            record(
                "local-fallback",
                AgentId::Codex.as_str(),
                "local-only",
                "/tmp/manager-projection-project/.agents/skills/local-only/SKILL.md",
                "/tmp/manager-projection-project/.agents/skills/local-only/SKILL.md",
                None,
                None,
            ),
        ];
        let lock = ManagerLockFile {
            skills: BTreeMap::from([(
                "shared".to_string(),
                ManagerLockEntry {
                    source: Some("owner/repository".to_string()),
                    source_type: Some("github".to_string()),
                    skill_path: Some("skills/shared/SKILL.md".to_string()),
                },
            )]),
        };

        let rows =
            installed_projection_rows(&ctx, "project", Scope::AgentProject, &records, Some(&lock))
                .expect("projection");

        assert_eq!(rows.len(), 2);
        let managed = rows
            .iter()
            .find(|row| row.name == "shared")
            .expect("manager row");
        assert_eq!(managed.source_kind, "manager");
        assert_eq!(
            managed.agents,
            vec![
                AgentId::ClaudeCode.as_str().to_string(),
                AgentId::Codex.as_str().to_string()
            ]
        );
        assert_eq!(
            managed.path.as_deref(),
            Some("<project-root>/.agents/skills/shared")
        );
        let local = rows
            .iter()
            .find(|row| row.name == "local-only")
            .expect("unlocked local fallback");
        assert_eq!(local.source_kind, "local");
        assert_eq!(local.agents, vec![AgentId::Codex.as_str().to_string()]);
    }

    #[test]
    fn malformed_lock_source_and_package_path_fail_closed() {
        let ctx = context();
        for entry in [
            ManagerLockEntry {
                source: None,
                source_type: Some("github".to_string()),
                skill_path: Some("skills/shared/SKILL.md".to_string()),
            },
            ManagerLockEntry {
                source: Some("owner/repository".to_string()),
                source_type: Some("github".to_string()),
                skill_path: Some("../outside/SKILL.md".to_string()),
            },
        ] {
            let lock = ManagerLockFile {
                skills: BTreeMap::from([("shared".to_string(), entry)]),
            };
            assert!(installed_projection_rows(
                &ctx,
                "project",
                Scope::AgentProject,
                &[],
                Some(&lock)
            )
            .is_err());
        }
    }

    #[test]
    #[cfg(unix)]
    fn discovery_state_and_manager_lock_reads_reject_symlinks() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "skills-copilot-discovery-no-follow-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let project = root.join("project");
        let app_data = root.join("app-data");
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(&app_data).expect("create app data");
        let outside = root.join("outside.json");
        fs::write(
            &outside,
            r#"{"schema_version":1,"generation":1,"token_digest":"token","action_id":"action","source_revision":"revision","phase":"outcome","state":"verified"}"#,
        )
        .expect("write outside projection");

        let replay_link = app_data.join(discovery_state_relative_path());
        symlink(&outside, &replay_link).expect("link replay state");
        let lock = crate::mutation_lock::lock_app_mutations(&app_data).expect("lock app data");
        let owner = lock.owner_fs();
        assert!(matches!(
            read_discovery_state_bytes(&owner),
            Err(CommandError::Io(_)) | Err(CommandError::UnsafeConfigPath(_))
        ));

        let lock_link = project.join("skills-lock.json");
        symlink(&outside, &lock_link).expect("link manager lock");
        let ctx = AdapterContext {
            user_home: root.join("home"),
            project_root: Some(project.clone()),
            project_cwd: Some(project),
            extra_roots: Vec::new(),
        };
        assert!(matches!(
            read_manager_lock_projection(&ctx, Some("project")),
            Err(CommandError::Io(_))
        ));
        assert!(
            outside.exists(),
            "rejected links must not touch their target"
        );
        fs::remove_dir_all(root).ok();
    }
}
