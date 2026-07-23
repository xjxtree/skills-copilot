use super::*;
use skills_copilot_catalog::{CatalogCommitError, CatalogError, CatalogImmediateTransaction};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct CompositeLocalDeletePlan {
    instance_id: String,
    skill_name: String,
    pub(super) skill_path: PathBuf,
    skill_directory: PathBuf,
    skill_directory_relative: PathBuf,
    catalog_revision: String,
    tree_revision: String,
}

pub(super) fn bind_composite_local_delete(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
    instance_id: &str,
    preview: &mut SkillManagerCommandPreview,
) -> Result<(), CommandError> {
    let plan = composite_local_delete_plan(catalog, app_data_dir, ctx, params, instance_id)?;
    let base_action = preview.action.as_ref().ok_or_else(|| {
        CommandError::MismatchedActionReference(
            "manager remove preview has no typed action".to_string(),
        )
    })?;
    let source_revision = action_source_revision(
        "manager.remove.composite-local-delete",
        &[
            ("manager_revision", &base_action.source_revision),
            ("local_instance_id", &plan.instance_id),
            ("local_catalog_revision", &plan.catalog_revision),
            ("local_tree_revision", &plan.tree_revision),
        ],
    )?;
    let descriptor = action_descriptor(
        base_action.kind,
        base_action.intent,
        base_action.target.clone(),
        base_action.project_id.clone(),
        base_action.impacts.clone(),
        &base_action.preview_method,
        base_action.apply_method.as_deref(),
        source_revision,
        base_action.confirmation_required,
        base_action.network,
        base_action.readback.clone(),
        base_action.evidence_refs.clone(),
    )?;
    let mut preconditions = preview.preconditions.clone();
    preconditions.extend([
        ActionPrecondition {
            kind: ActionPreconditionKind::CatalogRecord,
            target_id: plan.instance_id.clone(),
            expected_revision: plan.catalog_revision,
        },
        ActionPrecondition {
            kind: ActionPreconditionKind::SourceFile,
            target_id: plan.skill_path.to_string_lossy().to_string(),
            expected_revision: plan.tree_revision,
        },
    ]);
    let binding = action_preview_binding(descriptor, preconditions)?;
    preview.action = Some(binding.action);
    preview.preconditions = binding.preconditions;
    preview.preview_token = binding.preview_token;
    preview.summary = format!(
        "{} The same confirmed action will also remove the app-owned local source.",
        preview.summary
    );
    preview.risks.push(
        "This full uninstall removes the preview-bound app-owned local skill tree after every selected manager link is verified absent."
            .to_string(),
    );
    Ok(())
}

pub(super) fn composite_local_delete_plan(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
    instance_id: &str,
) -> Result<CompositeLocalDeletePlan, CommandError> {
    let meta = catalog
        .get_skill_instance_meta(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))?;
    let records = catalog.list_skill_records()?;
    let root = tool_global_staging_skills_root(app_data_dir);
    let canonical_root = root.canonicalize().map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "app-owned local skill root is unavailable".to_string(),
        )
    })?;
    let canonical_path = meta
        .path
        .canonicalize()
        .map_err(|_| CommandError::StaleActionReference)?;
    if meta.agent != AgentId::ToolGlobal || !canonical_path.starts_with(&canonical_root) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "full uninstall cleanup is limited to an app-owned local skill".to_string(),
        ));
    }
    let skill_directory = canonical_path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("local skill path has no parent".to_string())
    })?;
    if skill_directory.parent() != Some(canonical_root.as_path()) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "full uninstall cleanup target is outside the direct app-owned skill root".to_string(),
        ));
    }
    if !meta.name.eq_ignore_ascii_case(params.skill.trim()) {
        return Err(CommandError::MismatchedActionReference(
            "full uninstall cleanup names another skill".to_string(),
        ));
    }
    let references = local_delete_references(&meta, &records);
    if references.is_empty() {
        return Err(CommandError::NoApplicableAction(
            "the selected local source has no current agent links to remove".to_string(),
        ));
    }
    let selected_agents = manager_action_agents(&build_remove_preview(ctx, params)?.command)
        .into_iter()
        .map(AgentId::as_str)
        .collect::<BTreeSet<_>>();
    let expected_scope =
        if normalize_manager_scope(params.scope.as_deref())?.as_deref() == Some("global") {
            Scope::AgentGlobal
        } else {
            Scope::AgentProject
        };
    for reference in &references {
        if !selected_agents.contains(reference.agent.as_str())
            || reference.scope != expected_scope.as_str()
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "full uninstall must select every current supported-agent reference in the same scope"
                    .to_string(),
            ));
        }
        let record = records
            .iter()
            .find(|record| record.id == reference.instance_id)
            .ok_or(CommandError::StaleActionReference)?;
        let agent = selected_agents
            .iter()
            .find_map(|selected| {
                [
                    AgentId::ClaudeCode,
                    AgentId::Codex,
                    AgentId::Opencode,
                    AgentId::Pi,
                    AgentId::Hermes,
                    AgentId::Openclaw,
                ]
                .into_iter()
                .find(|agent| agent.as_str() == *selected && agent.as_str() == record.agent)
            })
            .ok_or(CommandError::StaleActionReference)?;
        let manager_root = manager_agent_skill_root(
            ctx,
            &manager_cwd(ctx, params.scope.as_deref())?,
            agent,
            expected_scope == Scope::AgentGlobal,
        );
        let relative_display_path = record
            .display_path
            .strip_prefix(&manager_root)
            .map_err(|_| {
                CommandError::InvalidSkillManagerRequest(
                    "full uninstall found a same-name reference outside the selected manager targets"
                        .to_string(),
                )
            })?;
        if relative_display_path.as_os_str().is_empty()
            || relative_display_path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
            || record
                .display_path
                .file_name()
                .and_then(|name| name.to_str())
                != Some("SKILL.md")
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "full uninstall found an unsafe selected manager reference path".to_string(),
            ));
        }
        let canonical_display_path = record
            .display_path
            .canonicalize()
            .map_err(|_| CommandError::StaleActionReference)?;
        let canonical_record_path = record
            .path
            .canonicalize()
            .unwrap_or_else(|_| record.path.clone());
        if canonical_display_path != canonical_path || canonical_record_path != canonical_path {
            return Err(CommandError::InvalidSkillManagerRequest(
                "full uninstall found a same-name manager reference to another source".to_string(),
            ));
        }
    }
    Ok(CompositeLocalDeletePlan {
        instance_id: meta.id.clone(),
        skill_name: meta.name.clone(),
        skill_path: canonical_path.clone(),
        skill_directory: skill_directory.to_path_buf(),
        skill_directory_relative: PathBuf::from("tool-global").join("skills").join(
            skill_directory
                .file_name()
                .ok_or(CommandError::VerificationFailed)?,
        ),
        catalog_revision: local_delete_catalog_revision(&meta, &canonical_path, &references)?,
        tree_revision: local_delete_tree_revision(&canonical_path)?,
    })
}

pub(super) struct CompositeLocalDeleteMutation {
    quarantine_relative: PathBuf,
    original_directory_relative: PathBuf,
    original_skill_path: PathBuf,
    original_tree_revision: String,
    pub(super) observations: Vec<ActionReadbackObservation>,
    active: bool,
}

impl CompositeLocalDeleteMutation {
    pub(super) fn restore(
        &mut self,
        owner: &crate::AppDataOwnerFs<'_>,
    ) -> Result<(), CommandError> {
        if !self.active {
            return Ok(());
        }
        if owner
            .snapshot_regular_tree(
                &self.original_directory_relative,
                MAX_MANAGER_TARGET_ENTRIES,
                MAX_MANAGER_TARGET_BYTES,
                MAX_MANAGER_TARGET_BYTES,
                "local delete target",
            )?
            .present
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "local delete target changed to an unowned third state during compensation"
                    .to_string(),
            ));
        }
        if local_delete_tree_revision_owner(
            owner,
            &self.quarantine_relative,
            &self.original_skill_path,
        )? != self.original_tree_revision
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "local delete quarantine changed to an unowned third state during compensation"
                    .to_string(),
            ));
        }
        owner.rename(&self.quarantine_relative, &self.original_directory_relative)?;
        if local_delete_tree_revision_owner(
            owner,
            &self.original_directory_relative,
            &self.original_skill_path,
        )? != self.original_tree_revision
        {
            return Err(CommandError::VerificationFailed);
        }
        self.active = false;
        Ok(())
    }

    pub(super) fn finish(&mut self, owner: &crate::AppDataOwnerFs<'_>) -> Result<(), CommandError> {
        if !self.active {
            return Ok(());
        }
        owner.remove_tree_if_exists(&self.quarantine_relative)?;
        self.active = false;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum CompositeLocalDeleteCommit {
    Committed,
    NotCommitted(CatalogError),
    OutcomeUnknown(CatalogError),
    RestorationFailed {
        commit_error: CatalogError,
        cleanup_error: CommandError,
    },
}

pub(super) fn commit_composite_local_delete(
    transaction: CatalogImmediateTransaction<'_>,
    owner: &crate::AppDataOwnerFs<'_>,
    cleanup: Option<&mut CompositeLocalDeleteMutation>,
) -> CompositeLocalDeleteCommit {
    match transaction.commit_classified() {
        Ok(()) => CompositeLocalDeleteCommit::Committed,
        Err(CatalogCommitError::NotCommitted(error)) => {
            if let Some(cleanup) = cleanup {
                if let Err(cleanup_error) = cleanup.restore(owner) {
                    return CompositeLocalDeleteCommit::RestorationFailed {
                        commit_error: error,
                        cleanup_error,
                    };
                }
            }
            CompositeLocalDeleteCommit::NotCommitted(error)
        }
        Err(CatalogCommitError::OutcomeUnknown(error)) => {
            CompositeLocalDeleteCommit::OutcomeUnknown(error)
        }
    }
}

pub(super) fn rollback_composite_local_delete(
    transaction: CatalogImmediateTransaction<'_>,
    owner: &crate::AppDataOwnerFs<'_>,
    cleanup: Option<&mut CompositeLocalDeleteMutation>,
    original_error: &CommandError,
) -> Result<(), CommandError> {
    crate::transaction_lifecycle::rollback_catalog_before_compensation(
        transaction,
        "skillManager.applyRemove",
        original_error,
        "the external manager result and any private local quarantine were preserved for inspection",
    )?;
    if let Some(cleanup) = cleanup {
        cleanup
            .restore(owner)
            .map_err(|cleanup_error| CommandError::PartialEffect {
                operation: "skillManager.applyRemove".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "post-execution verification failed ({original_error}); local source restoration failed ({cleanup_error})"
                ),
            })?;
    }
    Ok(())
}

pub(super) fn apply_composite_local_delete(
    catalog: &Catalog,
    _app_data_dir: &Path,
    owner: &crate::AppDataOwnerFs<'_>,
    plan: &CompositeLocalDeletePlan,
    recovery: &mut Option<CompositeLocalDeleteMutation>,
) -> Result<CompositeLocalDeleteMutation, CommandError> {
    let meta = catalog
        .get_skill_instance_meta(&plan.instance_id)?
        .ok_or(CommandError::StaleActionReference)?;
    let records = catalog.list_skill_records()?;
    if meta.agent != AgentId::ToolGlobal
        || meta.name != plan.skill_name
        || meta.path != plan.skill_path
        || !local_delete_references(&meta, &records).is_empty()
        || local_delete_tree_revision_owner(
            owner,
            &plan.skill_directory_relative,
            &plan.skill_path,
        )? != plan.tree_revision
    {
        return Err(CommandError::StaleActionReference);
    }
    let quarantine_relative = PathBuf::from("tool-global").join("skills").join(format!(
        ".agent-copilot-delete-{}-{}",
        safe_skill_name(&meta.name)?,
        unix_timestamp_millis()
    ));
    if owner
        .snapshot_regular_tree(
            &quarantine_relative,
            MAX_MANAGER_TARGET_ENTRIES,
            MAX_MANAGER_TARGET_BYTES,
            MAX_MANAGER_TARGET_BYTES,
            "local delete quarantine",
        )?
        .present
    {
        return Err(CommandError::StaleActionReference);
    }
    owner.rename(&plan.skill_directory_relative, &quarantine_relative)?;
    *recovery = Some(CompositeLocalDeleteMutation {
        quarantine_relative,
        original_directory_relative: plan.skill_directory_relative.clone(),
        original_skill_path: plan.skill_path.clone(),
        original_tree_revision: plan.tree_revision.clone(),
        observations: Vec::new(),
        active: true,
    });
    let quarantine_relative = &recovery
        .as_ref()
        .ok_or(CommandError::VerificationFailed)?
        .quarantine_relative;
    if local_delete_tree_revision_owner(owner, quarantine_relative, &plan.skill_path)?
        != plan.tree_revision
    {
        return Err(CommandError::VerificationFailed);
    }
    let missing_tree_revision =
        local_delete_tree_revision_owner(owner, &plan.skill_directory_relative, &plan.skill_path)?;
    let mutation = (|| -> Result<Vec<ActionReadbackObservation>, CommandError> {
        if local_delete_tree_revision_owner(
            owner,
            &plan.skill_directory_relative,
            &plan.skill_path,
        )? != missing_tree_revision
        {
            return Err(CommandError::VerificationFailed);
        }
        let payload = serde_json::json!({
            "deleted": true,
            "app_owned": true,
            "composite_manager_remove": true,
        });
        catalog.create_skill_event(SkillEventDraft {
            instance_id: &meta.id,
            kind: "local-delete",
            payload: &serde_json::to_string(&payload)?,
            occurred_at_ms: unix_timestamp_millis(),
        })?;
        catalog.delete_skill_instance(&meta.id)?;
        if catalog.get_skill_record(&meta.id)?.is_some() {
            return Err(CommandError::VerificationFailed);
        }
        Ok(vec![
            ActionReadbackObservation {
                domain: ActionReadbackDomain::SkillFiles,
                target_id: plan.skill_path.to_string_lossy().to_string(),
                revision: missing_tree_revision,
            },
            ActionReadbackObservation {
                domain: ActionReadbackDomain::CatalogSkills,
                target_id: plan.instance_id.clone(),
                revision: action_source_revision(
                    "catalog.skill.missing",
                    &[("instance_id", &plan.instance_id)],
                )?,
            },
        ])
    })();
    match mutation {
        Ok(observations) => {
            let mut mutation = recovery.take().ok_or(CommandError::VerificationFailed)?;
            mutation.observations = observations;
            Ok(mutation)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn test_instance(
        id: &str,
        agent: AgentId,
        scope: Scope,
        name: &str,
        path: PathBuf,
        display_path: PathBuf,
    ) -> skills_copilot_core::SkillInstance {
        skills_copilot_core::SkillInstance {
            id: id.to_string(),
            agent,
            scope,
            project_root: None,
            path,
            display_path,
            definition_id: format!("definition:{name}"),
            name: name.to_string(),
            display_name: name.to_string(),
            description: "composite removal fixture".to_string(),
            version: None,
            state: skills_copilot_core::SkillState::Loaded,
            enabled: true,
            frontmatter_raw: format!("name: {name}\ndescription: fixture"),
            body: "# Fixture".to_string(),
            scripts: Vec::new(),
            permissions: skills_copilot_core::PermissionRequest::default(),
            fingerprint: "sha256:fixture".to_string(),
            mtime: 1,
            first_seen: 1,
            last_seen: 1,
        }
    }

    #[cfg(unix)]
    #[test]
    fn preview_binds_app_owned_catalog_and_complete_source_tree() {
        use std::os::unix::fs::symlink;

        crate::initialize_action_preview_secret_for_test([0xA5; 32])
            .expect("initialize action preview test secret");
        let root = std::env::temp_dir().join(format!(
            "skill-manager-composite-remove-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = root.join("home");
        let app_data = root.join("app-data");
        let local_directory = tool_global_staging_skills_root(&app_data).join("owned-local");
        let local_skill = local_directory.join("SKILL.md");
        let manager_root = home.join(".agents/skills");
        let manager_link = manager_root.join("owned-local");
        fs::create_dir_all(&local_directory).expect("app-owned source");
        fs::create_dir_all(&manager_root).expect("manager root");
        fs::write(
            &local_skill,
            "---\nname: owned-local\ndescription: fixture\n---\n# Fixture\n",
        )
        .expect("local skill");
        fs::write(local_directory.join("reference.md"), "before").expect("attachment");
        symlink(&local_directory, &manager_link).expect("manager link");

        let canonical_skill = local_skill.canonicalize().expect("canonical local skill");
        let catalog = Catalog::in_memory().expect("catalog");
        catalog.init().expect("catalog schema");
        catalog
            .upsert_skill_instances(&[
                test_instance(
                    "tool-owned-local",
                    AgentId::ToolGlobal,
                    Scope::ToolGlobal,
                    "owned-local",
                    canonical_skill.clone(),
                    canonical_skill.clone(),
                ),
                test_instance(
                    "codex-owned-local",
                    AgentId::Codex,
                    Scope::AgentGlobal,
                    "owned-local",
                    canonical_skill.clone(),
                    manager_link.join("SKILL.md"),
                ),
            ])
            .expect("catalog fixtures");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        let params = SkillManagerRemoveParams {
            skill: "owned-local".to_string(),
            agents: vec!["codex".to_string()],
            scope: Some("global".to_string()),
            cleanup_local_instance_id: Some("tool-owned-local".to_string()),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        };

        assert!(matches!(
            preview_remove_with_manager(&ctx, &params),
            Err(CommandError::InvalidSkillManagerRequest(_))
        ));
        let preview = preview_remove_with_manager_guarded(&catalog, &app_data, &ctx, &params)
            .expect("composite preview");
        let action = preview.preview.action.as_ref().expect("typed action");
        assert_eq!(action.kind, ActionKind::ManagerRemove);
        assert!(preview.preview.preconditions.iter().any(|precondition| {
            precondition.kind == ActionPreconditionKind::CatalogRecord
                && precondition.target_id == "tool-owned-local"
        }));
        assert!(preview.preview.preconditions.iter().any(|precondition| {
            precondition.kind == ActionPreconditionKind::SourceFile
                && Path::new(&precondition.target_id) == canonical_skill
        }));

        fs::write(local_directory.join("reference.md"), "after").expect("attachment drift");
        let drifted = preview_remove_with_manager_guarded(&catalog, &app_data, &ctx, &params)
            .expect("reproject drifted tree");
        assert_ne!(
            preview.preview.preview_token, drifted.preview.preview_token,
            "an attachment change must alter the composite authorization token"
        );
        assert_ne!(
            action.source_revision,
            drifted
                .preview
                .action
                .as_ref()
                .expect("drifted typed action")
                .source_revision,
            "an attachment change must alter the composite action revision"
        );

        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
#[path = "composite_remove_fault_tests.rs"]
mod fault_tests;
