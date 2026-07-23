use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct ConfigDocumentRecord {
    pub agent: String,
    pub scope: String,
    pub target: String,
    pub format: String,
    pub content: String,
    pub exists: bool,
    pub revision: String,
}

pub fn read_claude_settings(ctx: &AdapterContext) -> Result<ConfigDocumentRecord, CommandError> {
    let target = claude_global_settings_path(ctx);
    validate_config_read_target(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
    let state = read_config_state(&target)?;
    let content = if state.exists {
        state.content
    } else {
        "{}\n".to_string()
    };
    Ok(ConfigDocumentRecord {
        agent: ClaudeCodeAdapter.id().as_str().to_string(),
        scope: Scope::AgentGlobal.as_str().to_string(),
        target: target.to_string_lossy().to_string(),
        format: "json".to_string(),
        content,
        exists: state.exists,
        revision: state.revision,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSavePreviewRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
    pub current: ConfigDocumentRecord,
    pub candidate_content_digest: String,
    pub current_revision: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSaveApplyRecord {
    pub action: ActionDescriptor,
    pub document: ConfigDocumentRecord,
    pub snapshot_id: String,
    pub readback: ActionReadbackRecord,
}

pub fn preview_claude_settings_save(
    ctx: &AdapterContext,
    content: &str,
    expected_revision: &str,
) -> Result<ConfigSavePreviewRecord, CommandError> {
    validate_claude_settings_content(content)?;
    let target = claude_global_settings_path(ctx);
    validate_config_read_target(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
    let current = read_config_state(&target)?;
    ensure_expected_revision(expected_revision, &current)?;
    build_claude_settings_save_preview(ctx, &target, content, current)
}

fn build_claude_settings_save_preview(
    ctx: &AdapterContext,
    target: &Path,
    content: &str,
    current: config_consistency::ConfigState,
) -> Result<ConfigSavePreviewRecord, CommandError> {
    let target_text = target.to_string_lossy().into_owned();
    let candidate_content_digest = config_content_digest(content);
    let changed = !current.exists || current.content != content;
    let project_id = canonical_project_id(ctx.project_root.as_deref());
    let project_binding = project_id.as_deref().unwrap_or_default();
    let revision_fields = [
        ("agent", AgentId::ClaudeCode.as_str()),
        ("scope", Scope::AgentGlobal.as_str()),
        ("project_id", project_binding),
        ("target", target_text.as_str()),
        (
            "candidate_content_digest",
            candidate_content_digest.as_str(),
        ),
        ("current_revision", current.revision.as_str()),
    ];
    let source_revision = if changed {
        action_source_revision("config.save_claude_settings", &revision_fields)?
    } else {
        non_applicable_source_revision("config.save_claude_settings.no_op", &revision_fields)?
    };
    let action = action_descriptor(
        ActionKind::SaveConfig,
        ActionIntent::SaveConfig,
        ActionTargetRef {
            kind: ActionTargetKind::Config,
            id: target_text.clone(),
            agent: Some(AgentId::ClaudeCode),
            scope: Some(Scope::AgentGlobal),
        },
        project_id,
        if changed {
            vec![ActionImpact::AgentConfig, ActionImpact::AppLocalData]
        } else {
            vec![ActionImpact::ReadOnly]
        },
        "config.previewSaveClaudeSettings",
        changed.then_some("config.saveClaudeSettings"),
        source_revision,
        changed,
        ActionNetworkPosture::None,
        if changed {
            canonical_readback_domains([
                ActionReadbackDomain::AgentConfig,
                ActionReadbackDomain::CatalogSkills,
                ActionReadbackDomain::ConfigSnapshots,
                ActionReadbackDomain::SkillAggregates,
            ])
        } else {
            vec![ActionReadbackDomain::AgentConfig]
        },
        vec![format!("config:{target_text}")],
    )?;
    let binding = action_preview_binding(
        action,
        vec![ActionPrecondition {
            kind: ActionPreconditionKind::AgentConfig,
            target_id: target_text,
            expected_revision: current.revision.clone(),
        }],
    )?;
    Ok(ConfigSavePreviewRecord {
        action: binding.action,
        preconditions: binding.preconditions,
        preview_token: binding.preview_token,
        current: config_document_record(
            AgentId::ClaudeCode,
            Scope::AgentGlobal,
            target,
            ConfigFormat::Json,
            &current,
        ),
        candidate_content_digest,
        current_revision: current.revision,
        changed,
    })
}

pub fn save_claude_settings(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    content: &str,
    confirmation: &ActionConfirmation,
) -> Result<ConfigSaveApplyRecord, CommandError> {
    reject_non_applicable_confirmation(confirmation)?;
    let prepared = prepare_claude_settings_save(ctx, content, confirmation)?;
    commit_prepared_claude_settings_save(catalog, app_data_dir, prepared)
}

/// Opaque read-only preflight state. Constructing it creates no directory,
/// lock file, catalog, snapshot, or config write.
pub struct PreparedClaudeSettingsSave {
    ctx: AdapterContext,
    target: PathBuf,
    content: String,
    confirmation: ActionConfirmation,
}

pub fn prepare_claude_settings_save(
    ctx: &AdapterContext,
    content: &str,
    confirmation: &ActionConfirmation,
) -> Result<PreparedClaudeSettingsSave, CommandError> {
    prepare_claude_settings_save_with_after_preflight(ctx, content, confirmation, || {})
}

fn prepare_claude_settings_save_with_after_preflight(
    ctx: &AdapterContext,
    content: &str,
    confirmation: &ActionConfirmation,
    after_preflight: impl FnOnce(),
) -> Result<PreparedClaudeSettingsSave, CommandError> {
    reject_non_applicable_confirmation(confirmation)?;
    validate_claude_settings_content(content)?;
    let target = claude_global_settings_path(ctx);
    validate_config_read_target(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
    let preflight = read_config_state(&target)?;
    let preflight_preview = build_claude_settings_save_preview(ctx, &target, content, preflight)?;
    ensure_action_confirmed(
        &ActionPreviewBinding {
            action: preflight_preview.action.clone(),
            preconditions: preflight_preview.preconditions.clone(),
            preview_token: preflight_preview.preview_token.clone(),
        },
        Some(confirmation),
    )?;
    after_preflight();

    Ok(PreparedClaudeSettingsSave {
        ctx: ctx.clone(),
        target,
        content: content.to_string(),
        confirmation: confirmation.clone(),
    })
}

pub fn commit_prepared_claude_settings_save(
    catalog: &Catalog,
    app_data_dir: &Path,
    prepared: PreparedClaudeSettingsSave,
) -> Result<ConfigSaveApplyRecord, CommandError> {
    commit_prepared_claude_settings_save_with_after_lock(catalog, app_data_dir, prepared, || {})
}

pub(crate) fn commit_prepared_claude_settings_save_with_after_lock(
    catalog: &Catalog,
    app_data_dir: &Path,
    prepared: PreparedClaudeSettingsSave,
    after_lock: impl FnOnce(),
) -> Result<ConfigSaveApplyRecord, CommandError> {
    commit_prepared_claude_settings_save_with_hooks(
        catalog,
        app_data_dir,
        prepared,
        after_lock,
        || Ok(()),
    )
}

pub(crate) fn commit_prepared_claude_settings_save_with_hooks(
    catalog: &Catalog,
    app_data_dir: &Path,
    prepared: PreparedClaudeSettingsSave,
    after_lock: impl FnOnce(),
    after_write: impl FnOnce() -> Result<(), CommandError>,
) -> Result<ConfigSaveApplyRecord, CommandError> {
    let PreparedClaudeSettingsSave {
        ctx,
        target,
        content,
        confirmation,
    } = prepared;
    let _owner_lock = lock_app_mutations(app_data_dir)?;
    after_lock();
    let transaction = catalog.begin_immediate_transaction()?;
    let locked_state = (|| {
        validate_config_read_target(&ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
        let current = read_config_state(&target)?;
        let preview = build_claude_settings_save_preview(&ctx, &target, &content, current.clone())?;
        ensure_action_confirmed(
            &ActionPreviewBinding {
                action: preview.action.clone(),
                preconditions: preview.preconditions.clone(),
                preview_token: preview.preview_token.clone(),
            },
            Some(&confirmation),
        )?;
        if !preview.changed {
            return Err(CommandError::NoApplicableAction(
                "the confirmed config save no longer changes the target".to_string(),
            ));
        }
        Ok((current, preview))
    })();
    let (current, preview) = match locked_state {
        Ok(locked_state) => locked_state,
        Err(error) => {
            return rollback_config_transaction_without_file(transaction, "config save", error);
        }
    };
    let missing_parents = missing_parent_chain(&target);
    let compensation = ConfigCompensation {
        ctx: &ctx,
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentGlobal,
        target: &target,
        candidate_content: &content,
        original: &current,
        missing_parents: &missing_parents,
        operation: "config save",
    };
    let target_text = target.to_string_lossy().into_owned();
    let snapshot_id = generate_snapshot_id();
    let snapshot_content = redact_snapshot_content(&current.content);
    let created_at = current_time_ms();
    let snapshot = ConfigSnapshotRecord {
        id: snapshot_id.clone(),
        agent: AgentId::ClaudeCode.as_str().to_string(),
        scope: Scope::AgentGlobal.as_str().to_string(),
        project_root: None,
        target: target_text.clone(),
        content: snapshot_content,
        reason: "pre-config-edit".to_string(),
        created_at,
    };
    let apply_result = (|| {
        catalog.create_config_snapshot(ConfigSnapshotDraft {
            id: &snapshot.id,
            agent: &snapshot.agent,
            scope: &snapshot.scope,
            project_root: None,
            target: &snapshot.target,
            content: &snapshot.content,
            reason: &snapshot.reason,
            created_at_ms: snapshot.created_at,
        })?;
        write_config_atomic(
            &ctx,
            AgentId::ClaudeCode,
            Scope::AgentGlobal,
            &target,
            &content,
        )?;
        after_write()?;
        scan_agent_id_to_catalog(AgentId::ClaudeCode, &ctx, catalog)?;
        finish_claude_settings_save(catalog, &ctx, &target, &content, preview, snapshot)
    })();
    let record = match apply_result {
        Ok(record) => record,
        Err(error) => {
            return rollback_config_transaction_and_compensate(transaction, &compensation, error);
        }
    };
    commit_config_transaction(transaction, &compensation, record)
}

fn finish_claude_settings_save(
    catalog: &Catalog,
    ctx: &AdapterContext,
    target: &Path,
    content: &str,
    preview: ConfigSavePreviewRecord,
    snapshot: ConfigSnapshotRecord,
) -> Result<ConfigSaveApplyRecord, CommandError> {
    let written = read_config_state(target)?;
    if !written.exists
        || written.content != content
        || written.revision != config_consistency::config_revision(true, content)
    {
        return Err(CommandError::VerificationFailed);
    }
    let stored_snapshot = catalog
        .get_config_snapshot(&snapshot.id)?
        .ok_or_else(|| CommandError::VerificationFailed)?;
    if stored_snapshot != snapshot {
        return Err(CommandError::VerificationFailed);
    }
    let document = config_document_record(
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        target,
        ConfigFormat::Json,
        &written,
    );
    let mut observations =
        config_catalog_projection_observations(catalog, ctx, AgentId::ClaudeCode)?;
    observations.extend([
        ActionReadbackObservation {
            domain: ActionReadbackDomain::AgentConfig,
            target_id: document.target.clone(),
            revision: document.revision.clone(),
        },
        ActionReadbackObservation {
            domain: ActionReadbackDomain::ConfigSnapshots,
            target_id: stored_snapshot.id.clone(),
            revision: snapshot_binding_revision(&stored_snapshot),
        },
    ]);
    let readback = ActionReadbackRecord::verified(&preview.action, observations)?;
    Ok(ConfigSaveApplyRecord {
        action: preview.action,
        document,
        snapshot_id: stored_snapshot.id,
        readback,
    })
}

fn config_catalog_projection_observations(
    catalog: &Catalog,
    ctx: &AdapterContext,
    agent: AgentId,
) -> Result<Vec<ActionReadbackObservation>, CommandError> {
    let mut records = catalog
        .list_skill_records_for_project_context(ctx.project_root.as_deref())?
        .into_iter()
        .filter(|record| record.agent == agent.as_str() && record.state != "missing")
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.id.cmp(&right.id));
    let records_json = serde_json::to_string(&records)?;
    let instance_ids = records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let definition_ids = records
        .iter()
        .map(|record| record.definition_id.as_str())
        .collect::<BTreeSet<_>>();
    let record_ids = records
        .iter()
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    let mut details = catalog.list_skill_details_by_ids(&record_ids)?;
    details.sort_by(|left, right| left.id.cmp(&right.id));

    let mut aggregate_inputs = BTreeMap::<String, Vec<(&str, &str, bool)>>::new();
    for record in &records {
        let aggregate_key = [
            record.definition_id.as_str(),
            record.source_kind.as_deref().unwrap_or("native"),
            record.publisher.as_deref().unwrap_or_default(),
            record.package_name.as_deref().unwrap_or_default(),
            record.package_version.as_deref().unwrap_or_default(),
            record.scope.as_str(),
            record.name.as_str(),
        ]
        .join("\u{1f}");
        aggregate_inputs.entry(aggregate_key).or_default().push((
            record.id.as_str(),
            record.state.as_str(),
            record.enabled,
        ));
    }
    let mut findings = catalog
        .list_rule_findings()?
        .into_iter()
        .filter(|finding| {
            finding
                .instance_id
                .as_deref()
                .is_some_and(|id| instance_ids.contains(id))
                || finding
                    .definition_id
                    .as_deref()
                    .is_some_and(|id| definition_ids.contains(id))
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| left.id.cmp(&right.id));
    let mut conflicts = catalog
        .list_conflict_groups()?
        .into_iter()
        .filter(|conflict| {
            conflict
                .instance_ids
                .iter()
                .any(|id| instance_ids.contains(id.as_str()))
        })
        .collect::<Vec<_>>();
    conflicts.sort_by(|left, right| left.id.cmp(&right.id));
    let aggregate_json = serde_json::to_string(&(aggregate_inputs, details, findings, conflicts))?;
    let target_prefix = format!("agent:{}", agent.as_str());
    Ok(vec![
        ActionReadbackObservation {
            domain: ActionReadbackDomain::CatalogSkills,
            target_id: format!("{target_prefix}:catalog-skills"),
            revision: action_source_revision(
                "config.catalog-skills.readback",
                &[("agent", agent.as_str()), ("records", &records_json)],
            )?,
        },
        ActionReadbackObservation {
            domain: ActionReadbackDomain::SkillAggregates,
            target_id: format!("{target_prefix}:skill-aggregates"),
            revision: action_source_revision(
                "config.skill-aggregates.readback",
                &[("agent", agent.as_str()), ("aggregates", &aggregate_json)],
            )?,
        },
    ])
}

struct ConfigCompensation<'a> {
    ctx: &'a AdapterContext,
    agent: AgentId,
    scope: Scope,
    target: &'a Path,
    candidate_content: &'a str,
    original: &'a config_consistency::ConfigState,
    missing_parents: &'a [PathBuf],
    operation: &'static str,
}

fn rollback_config_transaction_without_file<T>(
    transaction: CatalogImmediateTransaction<'_>,
    operation: &'static str,
    original_error: CommandError,
) -> Result<T, CommandError> {
    match transaction.rollback() {
        Ok(()) => Err(original_error),
        Err(rollback_error) => Err(CommandError::PartialEffect {
            operation: operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "operation failed before the file write ({original_error}); catalog rollback could not be proven ({rollback_error})"
            ),
        }),
    }
}

fn rollback_config_transaction_and_compensate<T>(
    transaction: CatalogImmediateTransaction<'_>,
    compensation: &ConfigCompensation<'_>,
    original_error: CommandError,
) -> Result<T, CommandError> {
    if let Err(rollback_error) = transaction.rollback() {
        return Err(CommandError::PartialEffect {
            operation: compensation.operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "operation failed ({original_error}); catalog rollback could not be proven ({rollback_error}); the exact written candidate was preserved for inspection"
            ),
        });
    }
    compensate_config_failure(compensation, original_error)
}

fn commit_config_transaction<T>(
    transaction: CatalogImmediateTransaction<'_>,
    compensation: &ConfigCompensation<'_>,
    record: T,
) -> Result<T, CommandError> {
    match transaction.commit_classified() {
        Ok(()) => Ok(record),
        Err(CatalogCommitError::NotCommitted(error)) => {
            compensate_config_failure(compensation, error.into())
        }
        Err(CatalogCommitError::OutcomeUnknown(error)) => Err(CommandError::PartialEffect {
            operation: compensation.operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "the config candidate was written and verified, but the catalog commit outcome is unknown ({error}); the candidate was preserved for inspection"
            ),
        }),
    }
}

fn compensate_config_failure<T>(
    compensation: &ConfigCompensation<'_>,
    original_error: CommandError,
) -> Result<T, CommandError> {
    let ConfigCompensation {
        ctx,
        agent,
        scope,
        target,
        candidate_content,
        original,
        missing_parents,
        operation,
    } = compensation;
    let candidate = config_consistency::ConfigState {
        exists: true,
        content: (*candidate_content).to_string(),
        revision: config_consistency::config_revision(true, candidate_content),
    };
    match read_config_state(target) {
        Ok(observed) if observed == candidate => {}
        Ok(observed) if &observed == *original => return Err(original_error),
        Ok(observed) => {
            return Err(CommandError::PartialEffect {
                operation: (*operation).to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "original failure: {original_error}; compensation was not attempted because the target changed after the confirmed write (observed revision {})",
                    observed.revision
                ),
            });
        }
        Err(read_error) => {
            return Err(CommandError::PartialEffect {
                operation: (*operation).to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "original failure: {original_error}; compensation was not attempted because the current target could not be verified: {read_error}"
                ),
            });
        }
    }
    match restore_config_state(
        ctx,
        *agent,
        *scope,
        target,
        original,
        missing_parents,
    ) {
        Ok(()) => Err(original_error),
        Err(compensation_error) => Err(CommandError::PartialEffect {
            operation: (*operation).to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "original failure: {original_error}; restoring the original config state failed: {compensation_error}"
            ),
        }),
    }
}

fn restore_config_state(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    target: &Path,
    original: &config_consistency::ConfigState,
    missing_parents: &[PathBuf],
) -> Result<(), CommandError> {
    if original.exists {
        write_config_atomic(ctx, agent, scope, target, &original.content)?;
    } else {
        validate_config_read_target(ctx, agent, scope, target)?;
        match fs::symlink_metadata(target) {
            Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_dir() => {
                return Err(CommandError::UnsafeConfigPath(
                    "compensation target changed into a non-file".to_string(),
                ));
            }
            Ok(_) => fs::remove_file(target)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        for parent in missing_parents {
            match fs::remove_dir(parent) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    let restored = read_config_state(target)?;
    if &restored != original {
        return Err(CommandError::VerificationFailed);
    }
    if missing_parents.iter().any(|parent| parent.exists()) {
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

fn missing_parent_chain(target: &Path) -> Vec<PathBuf> {
    let mut missing = Vec::new();
    let mut cursor = target.parent();
    while let Some(parent) = cursor {
        if parent.exists() {
            break;
        }
        missing.push(parent.to_path_buf());
        cursor = parent.parent();
    }
    missing
}

fn validate_claude_settings_content(content: &str) -> Result<(), CommandError> {
    serde_json::from_str::<serde_json::Value>(content)
        .map(|_| ())
        .map_err(|error| CommandError::InvalidJson(error.to_string()))
}

fn config_document_record(
    agent: AgentId,
    scope: Scope,
    target: &Path,
    format: ConfigFormat,
    state: &config_consistency::ConfigState,
) -> ConfigDocumentRecord {
    let content = if state.exists {
        state.content.clone()
    } else {
        match format {
            ConfigFormat::Json => "{}\n".to_string(),
            ConfigFormat::Toml | ConfigFormat::Yaml | ConfigFormat::Markdown => String::new(),
        }
    };
    ConfigDocumentRecord {
        agent: agent.as_str().to_string(),
        scope: scope.as_str().to_string(),
        target: target.to_string_lossy().into_owned(),
        format: match format {
            ConfigFormat::Json => "json",
            ConfigFormat::Toml => "toml",
            ConfigFormat::Yaml => "yaml",
            ConfigFormat::Markdown => "markdown",
        }
        .to_string(),
        content,
        exists: state.exists,
        revision: state.revision.clone(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRollbackPreviewRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
    pub snapshot: ConfigSnapshotRecord,
    pub snapshot_content_digest: String,
    pub current_content: String,
    pub current_read_error: Option<String>,
    pub current_revision: String,
    pub changed: bool,
    pub redacted: bool,
    pub rollback_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRollbackApplyRecord {
    pub action: ActionDescriptor,
    pub snapshot_id: String,
    pub document: ConfigDocumentRecord,
    pub readback: ActionReadbackRecord,
}

pub fn preview_snapshot_rollback(
    catalog: &Catalog,
    snapshot_id: &str,
) -> Result<SnapshotRollbackPreviewRecord, CommandError> {
    let snapshot = catalog
        .get_config_snapshot(snapshot_id)?
        .ok_or_else(|| CommandError::SnapshotNotFound(snapshot_id.to_string()))?;
    let ctx = preview_context_from_snapshot(&snapshot)?;
    validate_snapshot_project_binding(&ctx, &snapshot)?;
    preview_snapshot_rollback_for_record(&ctx, snapshot)
}

pub fn preview_snapshot_rollback_with_context(
    catalog: &Catalog,
    ctx: &AdapterContext,
    snapshot_id: &str,
) -> Result<SnapshotRollbackPreviewRecord, CommandError> {
    let snapshot = catalog
        .get_config_snapshot(snapshot_id)?
        .ok_or_else(|| CommandError::SnapshotNotFound(snapshot_id.to_string()))?;
    validate_snapshot_project_binding(ctx, &snapshot)?;
    preview_snapshot_rollback_for_record(ctx, snapshot)
}

fn preview_snapshot_rollback_for_record(
    ctx: &AdapterContext,
    snapshot: ConfigSnapshotRecord,
) -> Result<SnapshotRollbackPreviewRecord, CommandError> {
    let target = PathBuf::from(&snapshot.target);
    let scope = scope_from_snapshot(&snapshot.scope)?;
    let agent = agent_from_snapshot(&snapshot.agent)?;
    validate_config_read_target(ctx, agent, scope, &target)?;

    let current = read_config_state(&target)?;
    let current_read_error = (!current.exists)
        .then(|| "target file does not exist; rollback will recreate it".to_string());
    let redacted = is_redacted_snapshot_content(&snapshot.content);
    let changed = redacted || !current.exists || current.content != snapshot.content;
    let applicable = changed;
    let snapshot_content_digest = config_content_digest(&snapshot.content);
    let snapshot_revision = snapshot_binding_revision(&snapshot);
    let target_text = target.to_string_lossy().into_owned();
    let project_id = canonical_project_id(ctx.project_root.as_deref());
    let project_binding = project_id.as_deref().unwrap_or_default();
    let revision_fields = [
        ("snapshot_id", snapshot.id.as_str()),
        ("snapshot_revision", snapshot_revision.as_str()),
        ("agent", agent.as_str()),
        ("scope", scope.as_str()),
        ("project_id", project_binding),
        ("target", target_text.as_str()),
        ("snapshot_content_digest", snapshot_content_digest.as_str()),
        ("current_revision", current.revision.as_str()),
    ];
    let source_revision = if applicable {
        action_source_revision("snapshot.rollback", &revision_fields)?
    } else {
        non_applicable_source_revision("snapshot.rollback.no_op", &revision_fields)?
    };
    let action = action_descriptor(
        ActionKind::RollbackConfig,
        ActionIntent::RollbackConfig,
        ActionTargetRef {
            kind: ActionTargetKind::Config,
            id: target_text.clone(),
            agent: Some(agent),
            scope: Some(scope),
        },
        project_id,
        if applicable {
            vec![ActionImpact::AgentConfig, ActionImpact::AppLocalData]
        } else {
            vec![ActionImpact::ReadOnly]
        },
        "snapshot.previewRollback",
        applicable.then_some("snapshot.rollback"),
        source_revision,
        applicable,
        ActionNetworkPosture::None,
        if applicable {
            canonical_readback_domains([
                ActionReadbackDomain::AgentConfig,
                ActionReadbackDomain::CatalogSkills,
                ActionReadbackDomain::SkillAggregates,
            ])
        } else {
            vec![ActionReadbackDomain::AgentConfig]
        },
        vec![
            format!("snapshot:{}", snapshot.id),
            format!("config:{target_text}"),
        ],
    )?;
    let binding = action_preview_binding(
        action,
        vec![
            ActionPrecondition {
                kind: ActionPreconditionKind::AgentConfig,
                target_id: target_text,
                expected_revision: current.revision.clone(),
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::CatalogRecord,
                target_id: snapshot.id.clone(),
                expected_revision: snapshot_revision,
            },
        ],
    )?;
    Ok(SnapshotRollbackPreviewRecord {
        action: binding.action,
        preconditions: binding.preconditions,
        preview_token: binding.preview_token,
        snapshot,
        snapshot_content_digest,
        current_content: current.content,
        current_read_error,
        current_revision: current.revision,
        changed,
        redacted,
        rollback_supported: !redacted,
    })
}

pub fn rollback_snapshot(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    snapshot_id: &str,
    confirmation: &ActionConfirmation,
) -> Result<SnapshotRollbackApplyRecord, CommandError> {
    reject_non_applicable_confirmation(confirmation)?;
    rollback_snapshot_with_after_lock(catalog, app_data_dir, ctx, snapshot_id, confirmation, || {})
}

pub fn validate_snapshot_rollback_confirmation(
    catalog: &Catalog,
    ctx: &AdapterContext,
    snapshot_id: &str,
    confirmation: &ActionConfirmation,
) -> Result<SnapshotRollbackPreviewRecord, CommandError> {
    reject_non_applicable_confirmation(confirmation)?;
    let snapshot = catalog
        .get_config_snapshot(snapshot_id)?
        .ok_or(CommandError::StaleActionReference)?;
    validate_rollback_reference_identity(ctx, &snapshot, confirmation)?;
    validate_snapshot_project_binding(ctx, &snapshot)
        .map_err(|error| CommandError::MismatchedActionReference(error.to_string()))?;
    let preview = preview_snapshot_rollback_for_record(ctx, snapshot)?;
    ensure_action_confirmed(
        &ActionPreviewBinding {
            action: preview.action.clone(),
            preconditions: preview.preconditions.clone(),
            preview_token: preview.preview_token.clone(),
        },
        Some(confirmation),
    )?;
    Ok(preview)
}

pub(crate) fn rollback_snapshot_with_after_lock(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    snapshot_id: &str,
    confirmation: &ActionConfirmation,
    after_lock: impl FnOnce(),
) -> Result<SnapshotRollbackApplyRecord, CommandError> {
    rollback_snapshot_with_hooks(
        catalog,
        app_data_dir,
        ctx,
        snapshot_id,
        confirmation,
        after_lock,
        || Ok(()),
    )
}

pub(crate) fn rollback_snapshot_with_hooks(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    snapshot_id: &str,
    confirmation: &ActionConfirmation,
    after_lock: impl FnOnce(),
    after_write: impl FnOnce() -> Result<(), CommandError>,
) -> Result<SnapshotRollbackApplyRecord, CommandError> {
    let preflight =
        validate_snapshot_rollback_confirmation(catalog, ctx, snapshot_id, confirmation)?;
    let agent = agent_from_snapshot(&preflight.snapshot.agent)?;
    let scope = scope_from_snapshot(&preflight.snapshot.scope)?;
    let target = PathBuf::from(&preflight.snapshot.target);
    let _owner_lock = lock_app_mutations(app_data_dir)?;
    after_lock();
    let transaction = catalog.begin_immediate_transaction()?;
    let locked_state = (|| {
        let locked_snapshot = catalog
            .get_config_snapshot(snapshot_id)?
            .ok_or(CommandError::StaleActionReference)?;
        let (locked_agent, locked_scope, locked_target) =
            validate_rollback_reference_identity(ctx, &locked_snapshot, confirmation)?;
        if locked_agent != agent || locked_scope != scope || locked_target != target {
            return Err(CommandError::MismatchedActionReference(
                "snapshot target identity changed after the app mutation lock".to_string(),
            ));
        }
        validate_snapshot_project_binding(ctx, &locked_snapshot)
            .map_err(|error| CommandError::MismatchedActionReference(error.to_string()))?;
        validate_config_read_target(ctx, agent, scope, &target).map_err(|error| {
            CommandError::MismatchedActionReference(format!(
                "config target changed after the rollback preview: {error}"
            ))
        })?;
        let current = read_config_state(&target).map_err(|_| CommandError::StaleActionReference)?;
        let locked_preview = preview_snapshot_rollback_for_record(ctx, locked_snapshot.clone())?;
        ensure_action_confirmed(
            &ActionPreviewBinding {
                action: locked_preview.action.clone(),
                preconditions: locked_preview.preconditions.clone(),
                preview_token: locked_preview.preview_token.clone(),
            },
            Some(confirmation),
        )?;
        if !locked_preview.changed {
            return Err(CommandError::NoApplicableAction(
                "the confirmed rollback no longer changes the target".to_string(),
            ));
        }
        if is_redacted_snapshot_content(&locked_snapshot.content) {
            return Err(CommandError::UnsafeConfigPath(
                "snapshot content was redacted and cannot be rolled back directly".to_string(),
            ));
        }
        Ok((
            locked_snapshot,
            locked_agent,
            locked_scope,
            current,
            locked_preview,
        ))
    })();
    let (locked_snapshot, locked_agent, locked_scope, current, locked_preview) = match locked_state
    {
        Ok(locked_state) => locked_state,
        Err(error) => {
            return rollback_config_transaction_without_file(
                transaction,
                "snapshot rollback",
                error,
            );
        }
    };
    let missing_parents = missing_parent_chain(&target);
    let candidate_content = locked_snapshot.content.clone();
    let compensation = ConfigCompensation {
        ctx,
        agent: locked_agent,
        scope: locked_scope,
        target: &target,
        candidate_content: &candidate_content,
        original: &current,
        missing_parents: &missing_parents,
        operation: "snapshot rollback",
    };
    let apply_result = (|| {
        write_config_atomic(ctx, locked_agent, locked_scope, &target, &candidate_content)?;
        after_write()?;
        scan_agent_id_to_catalog(locked_agent, ctx, catalog)?;
        finish_snapshot_rollback(
            catalog,
            ctx,
            &target,
            locked_agent,
            locked_scope,
            locked_snapshot,
            locked_preview,
        )
    })();
    let record = match apply_result {
        Ok(record) => record,
        Err(error) => {
            return rollback_config_transaction_and_compensate(transaction, &compensation, error);
        }
    };
    commit_config_transaction(transaction, &compensation, record)
}

fn validate_rollback_reference_identity(
    ctx: &AdapterContext,
    snapshot: &ConfigSnapshotRecord,
    confirmation: &ActionConfirmation,
) -> Result<(AgentId, Scope, PathBuf), CommandError> {
    let agent = agent_from_snapshot(&snapshot.agent).map_err(|error| {
        CommandError::MismatchedActionReference(format!(
            "snapshot agent no longer matches the confirmed target: {error}"
        ))
    })?;
    let scope = scope_from_snapshot(&snapshot.scope).map_err(|error| {
        CommandError::MismatchedActionReference(format!(
            "snapshot scope no longer matches the confirmed target: {error}"
        ))
    })?;
    let target = PathBuf::from(&snapshot.target);
    let reference = &confirmation.reference;
    if reference.project_id != canonical_project_id(ctx.project_root.as_deref())
        || reference.target.kind != ActionTargetKind::Config
        || reference.target.id != snapshot.target
        || reference.target.agent != Some(agent)
        || reference.target.scope != Some(scope)
    {
        return Err(CommandError::MismatchedActionReference(
            "snapshot id, project, agent, scope, or target path changed after preview".to_string(),
        ));
    }
    Ok((agent, scope, target))
}

fn finish_snapshot_rollback(
    catalog: &Catalog,
    ctx: &AdapterContext,
    target: &Path,
    agent: AgentId,
    scope: Scope,
    snapshot: ConfigSnapshotRecord,
    preview: SnapshotRollbackPreviewRecord,
) -> Result<SnapshotRollbackApplyRecord, CommandError> {
    let written = read_config_state(target)?;
    if !written.exists
        || written.content != snapshot.content
        || written.revision != config_consistency::config_revision(true, &snapshot.content)
    {
        return Err(CommandError::VerificationFailed);
    }
    let document = config_document_record(
        agent,
        scope,
        target,
        config_format_for_snapshot_agent(agent)?,
        &written,
    );
    let mut observations = config_catalog_projection_observations(catalog, ctx, agent)?;
    observations.push(ActionReadbackObservation {
        domain: ActionReadbackDomain::AgentConfig,
        target_id: document.target.clone(),
        revision: document.revision.clone(),
    });
    let readback = ActionReadbackRecord::verified(&preview.action, observations)?;
    Ok(SnapshotRollbackApplyRecord {
        action: preview.action,
        snapshot_id: snapshot.id,
        document,
        readback,
    })
}

fn config_format_for_snapshot_agent(agent: AgentId) -> Result<ConfigFormat, CommandError> {
    match agent {
        AgentId::Codex => Ok(ConfigFormat::Toml),
        AgentId::Hermes => Ok(ConfigFormat::Yaml),
        AgentId::ClaudeCode | AgentId::Opencode | AgentId::Pi | AgentId::Openclaw => {
            Ok(ConfigFormat::Json)
        }
        _ => Err(CommandError::UnsafeConfigPath(format!(
            "{} config rollback is not supported",
            agent.as_str()
        ))),
    }
}
