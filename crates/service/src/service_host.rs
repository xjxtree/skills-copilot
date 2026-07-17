use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};

impl ServiceHost {
    pub fn from_env() -> Result<Self, ServiceError> {
        let user_home = env::var_os("SKILLS_COPILOT_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(PathBuf::from))
            .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
            .ok_or_else(|| {
                ServiceError::InvalidRequest("HOME or USERPROFILE is not set".to_string())
            })?;
        let app_data_dir = env::var_os("SKILLS_COPILOT_APP_DATA_DIR")
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| resolve_default_app_data_dir(&user_home))?;
        let project_cwd = env::var_os("SKILLS_COPILOT_PROJECT_CWD").map(PathBuf::from);
        let project_root = env::var_os("SKILLS_COPILOT_PROJECT_ROOT")
            .map(PathBuf::from)
            .or_else(|| project_cwd.as_deref().map(infer_project_root));
        let adapter_ctx = AdapterContext {
            user_home,
            project_root: project_root.clone(),
            project_cwd: project_cwd.or(project_root),
            extra_roots: Vec::new(),
        };
        Ok(Self {
            app_data_dir,
            adapter_ctx,
        })
    }

    pub fn handle(&self, request: ServiceRequest) -> ServiceResponse {
        let id = request.id.clone();
        match self.handle_result(request) {
            Ok(result) => ServiceResponse {
                id,
                ok: true,
                result: Some(result),
                error: None,
            },
            Err(error) => ServiceResponse {
                id,
                ok: false,
                result: None,
                error: Some(ServiceErrorRecord {
                    code: error.code().to_string(),
                    message: error.to_string(),
                }),
            },
        }
    }

    pub(crate) fn handle_result(&self, request: ServiceRequest) -> Result<Value, ServiceError> {
        match request.method.as_str() {
            "app.version" => serde_json::to_value(self.app_version()).map_err(Into::into),
            "app.stateSnapshot" => {
                serde_json::to_value(self.app_state_snapshot()?).map_err(Into::into)
            }
            "app.search" => {
                let params: AppSearchParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.search_app(params)?).map_err(Into::into)
            }
            "service.status" => serde_json::to_value(self.status()).map_err(Into::into),
            "adapter.listCapabilities" => {
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(list_adapter_capabilities(&adapter_ctx)).map_err(Into::into)
            }
            "adapter.listDiagnostics" => {
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(list_adapter_diagnostics(&adapter_ctx)).map_err(Into::into)
            }
            "session.previewLocalSessions" => {
                let params: LocalSessionPreviewParams = if request.params.is_null() {
                    LocalSessionPreviewParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                serde_json::to_value(self.preview_local_sessions(params)?).map_err(Into::into)
            }
            "session.listLocalSessionMessages" => {
                let params: LocalSessionMessagePageParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.list_local_session_messages(params)?).map_err(Into::into)
            }
            "llm.status" => serde_json::to_value(self.llm_status()).map_err(Into::into),
            "llm.listProviderProfiles" => {
                serde_json::to_value(self.list_llm_provider_profiles()?).map_err(Into::into)
            }
            "llm.saveProviderProfile" => {
                let params: SaveProviderProfileParams = serde_json::from_value(request.params)?;
                serde_json::to_value(save_provider_profile(&self.app_data_dir, params)?)
                    .map_err(Into::into)
            }
            "llm.deleteProviderProfile" => {
                let params: DeleteProviderProfileParams = serde_json::from_value(request.params)?;
                serde_json::to_value(delete_provider_profile(&self.app_data_dir, params)?)
                    .map_err(Into::into)
            }
            "llm.testProviderConnection" => {
                let params: TestProviderConnectionParams = serde_json::from_value(request.params)?;
                serde_json::to_value(test_provider_connection(&self.app_data_dir, params)?)
                    .map_err(Into::into)
            }
            "llm.previewPrompt" => {
                let params: LlmPreviewPromptParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.preview_llm_prompt(params)?).map_err(Into::into)
            }
            "llm.confirmPromptAndSend" => {
                let params: LlmConfirmPromptAndSendParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.confirm_llm_prompt_and_send(params)?).map_err(Into::into)
            }
            "llm.listPromptRuns" => {
                let params: LlmPromptRunListParams = if request.params.is_null() {
                    LlmPromptRunListParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                serde_json::to_value(self.list_llm_prompt_runs(params)?).map_err(Into::into)
            }
            "llm.providerObservability" => {
                let params: LlmProviderObservabilityParams = if request.params.is_null() {
                    LlmProviderObservabilityParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                serde_json::to_value(self.llm_provider_observability(params)?).map_err(Into::into)
            }
            "llm.listProviderActivity" => {
                let params: ListProviderActivityParams = if request.params.is_null() {
                    ListProviderActivityParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                serde_json::to_value(self.list_provider_activity(params)?).map_err(Into::into)
            }
            "llm.listModelTaskMatches" => {
                let params: ModelTaskMatchListParams = if request.params.is_null() {
                    ModelTaskMatchListParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                serde_json::to_value(self.list_model_task_matches(params)?).map_err(Into::into)
            }
            "llm.recordModelTaskMatch" => {
                let params: ModelTaskMatchRecordParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.record_model_task_match(params)?).map_err(Into::into)
            }
            "llm.deleteModelTaskMatch" => {
                let params: ModelTaskMatchDeleteParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.delete_model_task_match(params)?).map_err(Into::into)
            }
            "llm.prepareAction" => {
                let params: LlmPrepareActionParams = serde_json::from_value(request.params)?;
                serde_json::to_value(self.prepare_llm_action(params)?).map_err(Into::into)
            }
            "rules.listTuning" => {
                let catalog = self.open_catalog_for_read()?;
                let tuning: Vec<RuleTuningRecord> = list_rule_tuning(&catalog)?;
                serde_json::to_value(tuning).map_err(Into::into)
            }
            "rules.setSeverityOverride" => {
                let params: SetSeverityOverrideParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let tuning: RuleTuningRecord = set_rule_severity_override(
                    &catalog,
                    &params.rule_id,
                    params.agent.as_deref(),
                    params.scope.as_deref(),
                    &params.severity,
                )?;
                serde_json::to_value(tuning).map_err(Into::into)
            }
            "rules.clearSeverityOverride" => {
                let params: RuleTuningScopeParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let cleared: bool = clear_rule_severity_override(
                    &catalog,
                    &params.rule_id,
                    params.agent.as_deref(),
                    params.scope.as_deref(),
                )?;
                serde_json::to_value(cleared).map_err(Into::into)
            }
            "rules.setSuppression" => {
                let params: SetSuppressionParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let tuning: RuleTuningRecord = set_rule_suppression(
                    &catalog,
                    &params.rule_id,
                    params.agent.as_deref(),
                    params.scope.as_deref(),
                    &params.reason,
                    params.note.as_deref(),
                )?;
                serde_json::to_value(tuning).map_err(Into::into)
            }
            "rules.clearSuppression" => {
                let params: RuleTuningScopeParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let cleared: bool = clear_rule_suppression(
                    &catalog,
                    &params.rule_id,
                    params.agent.as_deref(),
                    params.scope.as_deref(),
                )?;
                serde_json::to_value(cleared).map_err(Into::into)
            }
            "batch.previewSkillToggles" => {
                let params: BatchPreviewSkillTogglesParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let preview: BatchTogglePreviewRecord = preview_skill_toggles(
                    &catalog,
                    &adapter_ctx,
                    &params.instance_ids,
                    params.target_enabled,
                )?;
                serde_json::to_value(preview).map_err(Into::into)
            }
            "batch.applySkillToggles" => {
                let params: BatchApplySkillTogglesParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let applied: BatchToggleApplyRecord = apply_skill_toggles(
                    &catalog,
                    &adapter_ctx,
                    &params.instance_ids,
                    params.target_enabled,
                    &params.preview_token,
                )?;
                serde_json::to_value(applied).map_err(Into::into)
            }
            "script.previewExecution" => {
                let params: ScriptExecutionRequest = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let preview: ScriptExecutionPreviewRecord =
                    preview_script_execution(&adapter_ctx, &params)?;
                serde_json::to_value(preview).map_err(Into::into)
            }
            "script.execute" => {
                let params: ScriptExecutionRequest = serde_json::from_value(request.params)?;
                if !params.confirmed {
                    return Err(ServiceError::ConfirmationRequired(
                        "script.execute requires confirmed=true on each request; use script.previewExecution to inspect the command, cwd, env, network, files, risks, and confirmation fields before confirming.".to_string(),
                    ));
                }
                let adapter_ctx = self.effective_adapter_ctx()?;
                let attempt: ScriptExecutionAttemptRecord = record_blocked_script_execution(
                    &adapter_ctx,
                    &self.app_data_dir.join("audit"),
                    &self.script_execution_audit_path(),
                    &params,
                )?;
                serde_json::to_value(attempt).map_err(Into::into)
            }
            "skillManager.listTools" => {
                serde_json::to_value(list_skill_management_tools()).map_err(Into::into)
            }
            "skillManager.search" => {
                let params: SkillManagerSearchParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(search_skills_with_manager(&adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.listInstalled" => {
                let params: SkillManagerListInstalledParams = if request.params.is_null() {
                    SkillManagerListInstalledParams::default()
                } else {
                    serde_json::from_value(request.params)?
                };
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(list_installed_skills_with_manager(&adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.previewInstall" => {
                let params: SkillManagerInstallParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_install_with_manager(&adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.applyInstall" => {
                let params: SkillManagerInstallParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_install_with_manager(&catalog, &adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.previewRemove" => {
                let params: SkillManagerRemoveParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_remove_with_manager(&adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.applyRemove" => {
                let params: SkillManagerRemoveParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_remove_with_manager(&catalog, &adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.previewUpdate" => {
                let params: SkillManagerUpdateParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_update_with_manager(&adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.applyUpdate" => {
                let params: SkillManagerUpdateParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_update_with_manager(&catalog, &adapter_ctx, &params)?)
                    .map_err(Into::into)
            }
            "skillManager.previewLocalCreate" => {
                let params: SkillManagerLocalCreateParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_local_create_with_manager(
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.applyLocalCreate" => {
                let params: SkillManagerLocalCreateParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_local_create_with_manager(
                    &catalog,
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.previewLocalArchiveImport" => {
                let params: SkillManagerLocalArchiveImportParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_local_archive_import(
                    &catalog,
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.applyLocalArchiveImport" => {
                let params: SkillManagerLocalArchiveImportParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_local_archive_import(
                    &catalog,
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.previewLocalArchiveUpdate" => {
                let params: SkillManagerLocalArchiveUpdateParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(preview_local_archive_update(
                    &catalog,
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.applyLocalArchiveUpdate" => {
                let params: SkillManagerLocalArchiveUpdateParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                serde_json::to_value(apply_local_archive_update(
                    &catalog,
                    &self.app_data_dir,
                    &adapter_ctx,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "skillManager.deleteLocal" => {
                let params: SkillManagerDeleteLocalParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                serde_json::to_value(delete_local_skill_with_manager(
                    &catalog,
                    &self.app_data_dir,
                    &params,
                )?)
                .map_err(Into::into)
            }
            "project.getContext" => {
                let state: ProjectContextState = load_project_context_state(&self.app_data_dir)?;
                serde_json::to_value(state).map_err(Into::into)
            }
            "project.setContext" => {
                let params: ProjectContextParams = serde_json::from_value(request.params)?;
                let state: ProjectContextState = set_project_context(&self.app_data_dir, params)?;
                serde_json::to_value(state).map_err(Into::into)
            }
            "project.clearContext" => {
                let state: ProjectContextState = clear_project_context(&self.app_data_dir)?;
                serde_json::to_value(state).map_err(Into::into)
            }
            "project.validateContext" => {
                let params: ProjectContextParams = serde_json::from_value(request.params)?;
                let context: ProjectContext = validate_project_context_for_response(params);
                serde_json::to_value(context).map_err(Into::into)
            }
            "catalog.listSkills" => {
                let catalog = self.open_catalog_for_read()?;
                serde_json::to_value(self.list_visible_skill_records(&catalog)?).map_err(Into::into)
            }
            "catalog.getSkill" => {
                let params: GetSkillParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let mut detail: SkillDetailRecord = get_skill(&catalog, &params.instance_id)?;
                apply_current_config_overrides_to_skill_detail(&adapter_ctx, &mut detail)?;
                serde_json::to_value(detail).map_err(Into::into)
            }
            "catalog.analysis" => {
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let analysis: CrossAgentAnalysisRecord = analyze_catalog(&catalog, &adapter_ctx)?;
                serde_json::to_value(analysis).map_err(Into::into)
            }
            "skill.listEvents" => {
                let params: ListSkillEventsParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let events: Vec<SkillEventRecord> =
                    list_skill_events(&catalog, &params.instance_id, params.limit)?;
                serde_json::to_value(events).map_err(Into::into)
            }
            "skill.listEventsPage" => {
                let params: ListSkillEventsPageParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let result = skill_event_page_result(&catalog, params)?;
                serde_json::to_value(result).map_err(Into::into)
            }
            "catalog.listFindings" => {
                let catalog = self.open_catalog_for_read()?;
                let findings: Vec<RuleFindingRecord> = list_findings(&catalog)?;
                serde_json::to_value(findings).map_err(Into::into)
            }
            "catalog.listFindingTriage" => {
                let catalog = self.open_catalog_for_read()?;
                let triage: Vec<FindingTriageRecord> = list_finding_triage(&catalog)?;
                serde_json::to_value(triage).map_err(Into::into)
            }
            "catalog.setFindingTriage" => {
                let params: SetFindingTriageParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let triage: FindingTriageRecord = set_finding_triage(
                    &catalog,
                    &params.triage_key,
                    &params.status,
                    params.note.as_deref(),
                )?;
                serde_json::to_value(triage).map_err(Into::into)
            }
            "catalog.clearFindingTriage" => {
                let params: ClearFindingTriageParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let cleared: bool = clear_finding_triage(&catalog, &params.triage_key)?;
                serde_json::to_value(cleared).map_err(Into::into)
            }
            "catalog.listConflicts" => {
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let conflicts: Vec<ConflictGroupRecord> =
                    list_conflicts_for_context(&catalog, &adapter_ctx)?;
                serde_json::to_value(conflicts).map_err(Into::into)
            }
            "catalog.importSkill" => {
                let params: ImportSkillParams = serde_json::from_value(request.params)?;
                if let Some(github_url) = params.github_url.as_deref() {
                    import_github_skill_to_tool_global_deferred(github_url)?;
                }
                let source_path = params.source_path.ok_or_else(|| {
                    ServiceError::InvalidRequest(
                        "catalog.importSkill requires source_path for local imports".to_string(),
                    )
                })?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let result: ToolGlobalImportResult = import_local_skill_to_tool_global(
                    &catalog,
                    &adapter_ctx,
                    &self.tool_global_staging_root(),
                    Path::new(&source_path),
                )?;
                serde_json::to_value(result).map_err(Into::into)
            }
            "catalog.scanClaude" => {
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let started_at = unix_timestamp_millis();
                let scan_report = scan_claude_catalog_report(&adapter_ctx, &catalog)?;
                let scanned_count = scan_report.scanned_count;
                let skills = self.list_visible_skill_records(&catalog)?;
                let findings: Vec<RuleFindingRecord> = list_findings(&catalog)?;
                let conflicts: Vec<ConflictGroupRecord> =
                    list_conflicts_for_context(&catalog, &adapter_ctx)?;
                let snapshots: Vec<ConfigSnapshotRecord> = list_snapshots(&catalog, &adapter_ctx)?;
                let adapter_diagnostics = list_adapter_diagnostics(&adapter_ctx);
                let agent_summaries = self.agent_refresh_summaries(
                    std::slice::from_ref(&scan_report),
                    &skills,
                    &adapter_diagnostics,
                );
                let activity = self.scan_activity(
                    "catalog.scanClaude",
                    "Claude Code",
                    scan_report.roots_considered.clone(),
                    started_at,
                    ScanActivityCounts {
                        scanned_count,
                        skill_count: skills.len(),
                        finding_count: findings.len(),
                        conflict_count: conflicts.len(),
                        snapshot_count: snapshots.len(),
                    },
                    Some(agent_summaries),
                );
                serde_json::to_value(ScanResult {
                    scanned_count,
                    skills,
                    activity,
                })
                .map_err(Into::into)
            }
            "catalog.scanAll" => {
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let started_at = unix_timestamp_millis();
                let scan_report = scan_all_catalog_report(&adapter_ctx, &catalog)?;
                let scanned_count = scan_report.scanned_count;
                let skills = self.list_visible_skill_records(&catalog)?;
                let findings: Vec<RuleFindingRecord> = list_findings(&catalog)?;
                let conflicts: Vec<ConflictGroupRecord> =
                    list_conflicts_for_context(&catalog, &adapter_ctx)?;
                let snapshots: Vec<ConfigSnapshotRecord> = list_snapshots(&catalog, &adapter_ctx)?;
                let adapter_diagnostics = list_adapter_diagnostics(&adapter_ctx);
                let agent_summaries = self.agent_refresh_summaries(
                    &scan_report.agents,
                    &skills,
                    &adapter_diagnostics,
                );
                let roots = scan_report
                    .agents
                    .iter()
                    .flat_map(|agent| agent.roots_considered.iter().cloned())
                    .collect();
                let scan_label = scan_all_label(&scan_report.agents);
                let activity = self.scan_activity(
                    "catalog.scanAll",
                    &scan_label,
                    roots,
                    started_at,
                    ScanActivityCounts {
                        scanned_count,
                        skill_count: skills.len(),
                        finding_count: findings.len(),
                        conflict_count: conflicts.len(),
                        snapshot_count: snapshots.len(),
                    },
                    Some(agent_summaries),
                );
                serde_json::to_value(ScanResult {
                    scanned_count,
                    skills,
                    activity,
                })
                .map_err(Into::into)
            }
            "skill.exportBundle" => {
                let params: ExportSkillBundleParams = serde_json::from_value(request.params)?;
                let output_dir = params
                    .output_dir
                    .unwrap_or_else(|| self.app_data_dir.join("exports"));
                let exported: ExportedSkillBundle =
                    match (params.instance_id.as_deref(), params.source_path.as_deref()) {
                        (Some(instance_id), None) => {
                            let catalog = self.open_catalog()?;
                            export_skill_bundle(&catalog, instance_id, &output_dir)?
                        }
                        (None, Some(source_path)) => {
                            export_staging_skill_bundle(source_path, &output_dir)?
                        }
                        _ => {
                            return Err(ServiceError::InvalidRequest(
                            "skill.exportBundle requires exactly one of instance_id or source_path"
                                .to_string(),
                        ));
                        }
                    };
                serde_json::to_value(exported).map_err(Into::into)
            }
            "config.toggleSkill" => {
                let params: ToggleSkillParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let record: SkillRecord =
                    toggle_skill(&catalog, &adapter_ctx, &params.instance_id, params.on)?;
                serde_json::to_value(record).map_err(Into::into)
            }
            "skill.install" => {
                let params: InstallSkillParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let target_agent = parse_agent_param(&params.target_agent)?;
                let target_scope = parse_scope_param(&params.target_scope)?;
                let preview: SkillInstallPreviewRecord = install_skill_from_tool_global(
                    &catalog,
                    &adapter_ctx,
                    &params.instance_id,
                    target_agent,
                    target_scope,
                    params.project_path.as_deref(),
                    params.confirmed,
                )?;
                serde_json::to_value(preview).map_err(Into::into)
            }
            "config.readClaudeSettings" => {
                let adapter_ctx = self.effective_adapter_ctx()?;
                let document: ConfigDocumentRecord = read_claude_settings(&adapter_ctx)?;
                serde_json::to_value(document).map_err(Into::into)
            }
            "config.readAgentConfig" => {
                let params: ReadAgentConfigParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let documents: Vec<ConfigDocumentRecord> =
                    read_agent_config(&adapter_ctx, &params.agent, params.scope.as_deref())?;
                serde_json::to_value(documents).map_err(Into::into)
            }
            "config.saveClaudeSettings" => {
                let params: SaveClaudeSettingsParams = serde_json::from_value(request.params)?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let prepared = prepare_claude_settings_save(
                    &adapter_ctx,
                    &params.content,
                    &params.expected_revision,
                )?;
                let catalog = self.open_catalog()?;
                let document: ConfigDocumentRecord =
                    commit_prepared_claude_settings_save(&catalog, prepared)?;
                serde_json::to_value(document).map_err(Into::into)
            }
            "snapshot.list" => {
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let snapshots: Vec<ConfigSnapshotRecord> = list_snapshots(&catalog, &adapter_ctx)?;
                serde_json::to_value(snapshots).map_err(Into::into)
            }
            "snapshot.listAgentConfig" => {
                let params: ListAgentConfigSnapshotsParams =
                    serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let scope = params.scope.as_deref().filter(|scope| !scope.is_empty());
                let snapshots: Vec<ConfigSnapshotRecord> =
                    list_agent_config_snapshots(&catalog, &adapter_ctx, &params.agent, scope)?;
                serde_json::to_value(snapshots).map_err(Into::into)
            }
            "snapshot.listAgentConfigPage" => {
                let params: ListAgentConfigPageParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let result = config_snapshot_page_result(&catalog, &adapter_ctx, params)?;
                serde_json::to_value(result).map_err(Into::into)
            }
            "snapshot.previewRollback" => {
                let params: SnapshotParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog_for_read()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let preview: SnapshotRollbackPreviewRecord =
                    preview_snapshot_rollback_with_context(
                        &catalog,
                        &adapter_ctx,
                        &params.snapshot_id,
                    )?;
                serde_json::to_value(preview).map_err(Into::into)
            }
            "snapshot.rollback" => {
                let params: RollbackSnapshotParams = serde_json::from_value(request.params)?;
                let catalog = self.open_catalog()?;
                let adapter_ctx = self.effective_adapter_ctx()?;
                let scanned_count = rollback_snapshot(
                    &catalog,
                    &adapter_ctx,
                    &params.snapshot_id,
                    &params.preview_token,
                )?;
                serde_json::to_value(scanned_count).map_err(Into::into)
            }
            method => Err(ServiceError::UnknownMethod(method.to_string())),
        }
    }

    pub fn app_version(&self) -> AppVersion {
        AppVersion {
            protocol_version: SERVICE_PROTOCOL_VERSION,
            version: skills_copilot_commands::app_version(),
        }
    }

    pub fn app_state_snapshot(&self) -> Result<AppStateSnapshot, ServiceError> {
        let catalog = self.open_catalog_for_read()?;
        let adapter_ctx = self.effective_adapter_ctx()?;
        let skills = self.list_visible_skill_records(&catalog)?;
        let findings = list_findings(&catalog)?;
        let conflicts = list_conflicts_for_context(&catalog, &adapter_ctx)?;
        let analysis = analyze_catalog(&catalog, &adapter_ctx)?;
        let health = skill_health_summary(&catalog, &adapter_ctx)?;
        Ok(AppStateSnapshot {
            status: self.status(),
            skills,
            findings,
            conflicts,
            analysis,
            health,
            snapshots: list_snapshots(&catalog, &adapter_ctx)?,
        })
    }

    pub(crate) fn list_visible_skill_records(
        &self,
        catalog: &Catalog,
    ) -> Result<Vec<SkillRecord>, ServiceError> {
        let adapter_ctx = self.effective_adapter_ctx()?;
        let mut skills =
            catalog.list_skill_records_for_project_context(adapter_ctx.project_root.as_deref())?;
        apply_current_config_overrides_to_skill_records(&adapter_ctx, &mut skills)?;
        Ok(skills
            .into_iter()
            .filter(|skill| !is_pi_plain_markdown_catalog_noise(skill))
            .collect())
    }

    pub fn status(&self) -> ServiceStatus {
        let adapter_ctx = self.status_adapter_ctx();
        ServiceStatus {
            protocol_version: SERVICE_PROTOCOL_VERSION,
            version: skills_copilot_commands::app_version(),
            app_data_dir: display_path(&self.app_data_dir),
            catalog_path: display_path(&self.catalog_path()),
            user_home: display_path(&adapter_ctx.user_home),
            supported_methods: supported_methods(),
            refresh: RefreshStatus {
                scan_progress: "summary-only",
                watcher_state: "manual-refresh",
                watcher_detail: "The current stdio sidecar reports completed refresh summaries; native automatic watcher events are not running in this process.",
                recovery_actions: vec!["Retry the last refresh", "Run Scan to rebuild the agent catalog"],
            },
            project_context: project_context_summary(&self.app_data_dir, self.env_project_context()),
            adapter_capabilities: list_adapter_capabilities(&adapter_ctx),
            adapter_diagnostics: list_adapter_diagnostics(&adapter_ctx),
            llm: self.llm_status(),
            script_execution: self.script_execution_status(),
        }
    }

    pub(crate) fn open_catalog(&self) -> Result<Catalog, ServiceError> {
        create_private_dir_all(&self.app_data_dir)?;
        let catalog = Catalog::open(&self.catalog_path())?;
        catalog.init()?;
        Ok(catalog)
    }

    pub(crate) fn open_catalog_for_read(&self) -> Result<Catalog, ServiceError> {
        let path = self.catalog_path();
        if path.exists() {
            return Catalog::open_read_only_after_migration(&path).map_err(Into::into);
        }
        let catalog = Catalog::in_memory()?;
        catalog.init()?;
        Ok(catalog)
    }

    pub(crate) fn open_existing_catalog_read_only(&self) -> Result<Option<Catalog>, ServiceError> {
        let catalog_path = self.catalog_path();
        if !catalog_path.exists() {
            return Ok(None);
        }
        Ok(Some(Catalog::open_read_only_after_migration(
            &catalog_path,
        )?))
    }

    pub(crate) fn catalog_path(&self) -> PathBuf {
        self.app_data_dir.join("catalog.sqlite")
    }

    pub(crate) fn script_execution_audit_path(&self) -> PathBuf {
        self.app_data_dir
            .join("audit")
            .join("script-execution.jsonl")
    }

    pub(crate) fn llm_prompt_runs_path(&self) -> PathBuf {
        self.app_data_dir.join("prompt-runs.json")
    }

    pub(crate) fn model_task_matches_path(&self) -> PathBuf {
        self.app_data_dir.join("model-task-matches.json")
    }

    pub(crate) fn load_llm_prompt_runs(&self) -> Result<Vec<LlmPromptRunRecord>, ServiceError> {
        let path = self.llm_prompt_runs_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let mut runs: Vec<LlmPromptRunRecord> = serde_json::from_str(&content)?;
        runs.sort_by(llm_prompt_run_record_sort);
        Ok(runs)
    }

    pub(crate) fn load_model_task_matches(
        &self,
    ) -> Result<Vec<ModelTaskMatchRecord>, ServiceError> {
        let path = self.model_task_matches_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let mut records: Vec<ModelTaskMatchRecord> = serde_json::from_str(&content)?;
        records.sort_by(model_task_match_record_sort);
        Ok(records)
    }

    pub(crate) fn save_model_task_matches(
        &self,
        records: &[ModelTaskMatchRecord],
    ) -> Result<(), ServiceError> {
        let path = self.model_task_matches_path();
        let mut sorted = records.to_vec();
        sorted.sort_by(model_task_match_record_sort);
        let content = serde_json::to_string_pretty(&sorted)?;
        write_private_text_file(&path, &content)?;
        Ok(())
    }

    pub(crate) fn load_llm_prompt_runs_for_observability(
        &self,
        redaction_roots: &[(String, &'static str)],
    ) -> (
        Vec<LlmPromptRunRecord>,
        Vec<LlmProviderObservabilityStatusRow>,
    ) {
        let path = self.llm_prompt_runs_path();
        if !path.exists() {
            return (
                Vec::new(),
                vec![provider_observability_status_row(
                    "file:prompt-runs",
                    "prompt-runs.json",
                    "absent",
                    "info",
                    "No app-local prompt run history file exists yet.",
                    0,
                    vec!["app-data:prompt-runs.json".to_string()],
                )],
            );
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                return (
                    Vec::new(),
                    vec![provider_observability_status_row(
                        "file:prompt-runs",
                        "prompt-runs.json",
                        "read_error",
                        "warning",
                        format!(
                            "Could not read app-local prompt run history: {}",
                            observability_redact(&error.to_string(), redaction_roots, 300)
                        ),
                        0,
                        vec!["app-data:prompt-runs.json".to_string()],
                    )],
                );
            }
        };
        match serde_json::from_str::<Vec<LlmPromptRunRecord>>(&content) {
            Ok(mut runs) => {
                runs.sort_by(llm_prompt_run_record_sort);
                let count = runs.len();
                (
                    runs,
                    vec![provider_observability_status_row(
                        "file:prompt-runs",
                        "prompt-runs.json",
                        "loaded",
                        "info",
                        format!("Loaded {count} app-local prompt run metadata record(s)."),
                        count,
                        vec!["app-data:prompt-runs.json".to_string()],
                    )],
                )
            }
            Err(error) => (
                Vec::new(),
                vec![provider_observability_status_row(
                    "file:prompt-runs",
                    "prompt-runs.json",
                    "parse_error",
                    "warning",
                    format!(
                        "Could not parse app-local prompt run history: {}",
                        observability_redact(&error.to_string(), redaction_roots, 300)
                    ),
                    0,
                    vec!["app-data:prompt-runs.json".to_string()],
                )],
            ),
        }
    }

    pub(crate) fn load_provider_call_metadata_for_observability(
        &self,
        redaction_roots: &[(String, &'static str)],
    ) -> (
        Vec<ProviderCallMetadata>,
        Vec<LlmProviderObservabilityStatusRow>,
    ) {
        let path = provider_call_metadata_path(&self.app_data_dir);
        if !path.exists() {
            return (
                Vec::new(),
                vec![provider_observability_status_row(
                    "file:provider-call-metadata",
                    "provider-call-metadata.jsonl",
                    "absent",
                    "info",
                    "No app-local provider call metadata file exists yet.",
                    0,
                    vec!["app-data:llm/provider-call-metadata.jsonl".to_string()],
                )],
            );
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                return (
                    Vec::new(),
                    vec![provider_observability_status_row(
                        "file:provider-call-metadata",
                        "provider-call-metadata.jsonl",
                        "read_error",
                        "warning",
                        format!(
                            "Could not read app-local provider call metadata: {}",
                            observability_redact(&error.to_string(), redaction_roots, 300)
                        ),
                        0,
                        vec!["app-data:llm/provider-call-metadata.jsonl".to_string()],
                    )],
                );
            }
        };
        let mut rows = Vec::new();
        let mut parse_error_count = 0usize;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<ProviderCallMetadata>(trimmed) {
                Ok(mut metadata) => {
                    metadata.timestamp = provider::normalize_epoch_millis(metadata.timestamp);
                    rows.push(metadata);
                }
                Err(_) => {
                    parse_error_count += 1;
                }
            }
        }
        rows.sort_by(|left, right| {
            right
                .timestamp
                .cmp(&left.timestamp)
                .then_with(|| left.profile_id.cmp(&right.profile_id))
                .then_with(|| left.action_type.cmp(&right.action_type))
        });
        let mut status_rows = vec![provider_observability_status_row(
            "file:provider-call-metadata",
            "provider-call-metadata.jsonl",
            "loaded",
            "info",
            format!(
                "Loaded {} app-local provider call metadata record(s).",
                rows.len()
            ),
            rows.len(),
            vec!["app-data:llm/provider-call-metadata.jsonl".to_string()],
        )];
        if parse_error_count > 0 {
            status_rows.push(provider_observability_status_row(
                "file:provider-call-metadata:parse-errors",
                "provider-call-metadata.jsonl",
                "parse_error",
                "warning",
                format!("Skipped {parse_error_count} malformed provider call metadata line(s)."),
                parse_error_count,
                vec!["app-data:llm/provider-call-metadata.jsonl".to_string()],
            ));
        }
        (rows, status_rows)
    }

    pub(crate) fn load_provider_profiles_for_observability(
        &self,
        redaction_roots: &[(String, &'static str)],
    ) -> (
        Vec<ProviderProfileRecord>,
        Vec<LlmProviderObservabilityStatusRow>,
    ) {
        let path = provider_profiles_path(&self.app_data_dir);
        match list_provider_profiles(&self.app_data_dir) {
            Ok(result) => {
                let status = if path.exists() { "loaded" } else { "absent" };
                let message = if path.exists() {
                    format!(
                        "Loaded {} configured provider profile metadata record(s) without reading credentials.",
                        result.profiles.len()
                    )
                } else {
                    "No provider profile metadata file exists yet.".to_string()
                };
                let count = result.profiles.len();
                (
                    result.profiles,
                    vec![provider_observability_status_row(
                        "file:provider-profiles",
                        "provider-profiles.json",
                        status,
                        "info",
                        message,
                        count,
                        vec!["app-data:llm/provider-profiles.json".to_string()],
                    )],
                )
            }
            Err(error) => (
                Vec::new(),
                vec![provider_observability_status_row(
                    "file:provider-profiles",
                    "provider-profiles.json",
                    "parse_error",
                    "warning",
                    format!(
                        "Could not read provider profile metadata without credential access: {}",
                        observability_redact(&error.to_string(), redaction_roots, 300)
                    ),
                    0,
                    vec!["app-data:llm/provider-profiles.json".to_string()],
                )],
            ),
        }
    }

    pub(crate) fn save_llm_prompt_runs(
        &self,
        runs: &[LlmPromptRunRecord],
    ) -> Result<(), ServiceError> {
        let path = self.llm_prompt_runs_path();
        let mut sorted = runs.to_vec();
        sorted.sort_by(llm_prompt_run_record_sort);
        let content = serde_json::to_string_pretty(&sorted)?;
        write_private_text_file(&path, &content)?;
        Ok(())
    }

    pub(crate) fn record_llm_prompt_run(
        &self,
        params: &LlmConfirmPromptAndSendParams,
        preview: &LlmPreviewPromptResult,
        send: &provider::SendProviderPromptResult,
    ) -> Result<(), ServiceError> {
        let adapter_ctx = self.effective_adapter_ctx()?;
        let roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&roots);
        let task = params
            .request
            .user_intent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 500));
        let error_message = send
            .error_message
            .as_deref()
            .map(|value| truncate_chars(&redactor.redact(value), 500));
        let draft_output = send
            .output_text
            .as_deref()
            .map(|value| truncate_chars(&redactor.redact(value), 12_000));
        let request_redaction = redactor.summary();
        let completed_at = unix_timestamp_millis();
        let estimated_total_tokens = preview
            .estimated_input_tokens
            .saturating_add(preview.estimated_output_tokens);
        let mut instance_ids = params.request.instance_ids.clone();
        if let Some(instance_id) = params.request.skill_instance_id.as_deref() {
            if !instance_id.trim().is_empty() && !instance_ids.iter().any(|id| id == instance_id) {
                instance_ids.push(instance_id.to_string());
            }
        }
        instance_ids = normalize_string_list(instance_ids);

        let record = LlmPromptRunRecord {
            id: generated_llm_prompt_run_id(
                &params.preview_id,
                &params.confirmation_id,
                completed_at,
            ),
            preview_id: params.preview_id.clone(),
            confirmation_id: params.confirmation_id.clone(),
            action: params.request.action.as_str().to_string(),
            request_kind: params.request.action.as_str().to_string(),
            analysis_kind: None,
            scope: inferred_llm_prompt_scope(&params.request),
            instance_id: params
                .request
                .skill_instance_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            instance_ids,
            definition_id: None,
            agent: None,
            task,
            profile_id: send.profile_id.clone(),
            provider: send.provider_type.as_str().to_string(),
            model: send.model.clone(),
            destination_host: send.destination_host.clone(),
            status: send.status.clone(),
            error_code: send.error_code.clone(),
            error_message,
            duration_ms: u64::try_from(send.duration_ms).unwrap_or(u64::MAX),
            estimated_input_tokens: preview.estimated_input_tokens,
            estimated_output_tokens: preview.estimated_output_tokens,
            estimated_total_tokens,
            estimated_cost_usd: preview.estimated_cost_usd,
            draft_output,
            draft_requires_user_copy: true,
            provider_request_sent: send.provider_request_sent,
            credential_accessed: send.credential_accessed,
            raw_secret_returned: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            redaction_summary: llm_prompt_run_redaction_summary_from(
                preview.redaction.clone(),
                request_redaction,
            ),
            created_at: completed_at,
            completed_at,
            safety_flags: llm_prompt_run_safety_flags(
                send.provider_request_sent,
                send.credential_accessed,
            ),
        };

        let mut runs = self.load_llm_prompt_runs()?;
        runs.push(record);
        self.save_llm_prompt_runs(&runs)?;
        Ok(())
    }

    pub(crate) fn trace_redaction_roots(
        &self,
        adapter_ctx: &AdapterContext,
    ) -> Vec<(String, &'static str)> {
        let mut roots = self.redaction_roots(adapter_ctx);
        roots.push((env::temp_dir().to_string_lossy().to_string(), "<temp-dir>"));
        roots.sort_by_key(|right| std::cmp::Reverse(right.0.len()));
        roots.dedup_by(|left, right| left.0 == right.0);
        roots
    }

    pub(crate) fn tool_global_staging_root(&self) -> PathBuf {
        self.app_data_dir.join("tool-global")
    }

    pub(crate) fn redaction_roots(
        &self,
        adapter_ctx: &AdapterContext,
    ) -> Vec<(String, &'static str)> {
        fn push_root(
            roots: &mut Vec<(String, &'static str)>,
            path: &Path,
            placeholder: &'static str,
        ) {
            fn push_root_text(
                roots: &mut Vec<(String, &'static str)>,
                value: String,
                placeholder: &'static str,
            ) {
                roots.push((value.clone(), placeholder));
                let normalized = normalized_redaction_path_text(&value);
                if normalized != value {
                    roots.push((normalized, placeholder));
                }
            }

            push_root_text(roots, path.to_string_lossy().to_string(), placeholder);
            if let Ok(canonical) = path.canonicalize() {
                push_root_text(roots, canonical.to_string_lossy().to_string(), placeholder);
            }
        }

        let mut roots = Vec::new();
        push_root(&mut roots, &self.app_data_dir, "<app-data-dir>");
        push_root(&mut roots, &adapter_ctx.user_home, "$HOME");
        if let Some(project_root) = adapter_ctx.project_root.as_ref() {
            push_root(&mut roots, project_root, "<project-root>");
        }
        if let Some(project_cwd) = adapter_ctx.project_cwd.as_ref() {
            push_root(&mut roots, project_cwd, "<project-cwd>");
        }
        roots.sort_by_key(|right| std::cmp::Reverse(right.0.len()));
        roots.dedup_by(|left, right| left.0 == right.0);
        roots
    }

    pub(crate) fn effective_adapter_ctx(&self) -> Result<AdapterContext, ServiceError> {
        if self.has_env_project_context() {
            return Ok(self.adapter_ctx.clone());
        }

        let Some((root_path, current_cwd)) = stored_active_adapter_paths(&self.app_data_dir)?
        else {
            return Ok(self.adapter_ctx.clone());
        };

        let mut ctx = self.adapter_ctx.clone();
        ctx.project_root = Some(root_path);
        ctx.project_cwd = Some(current_cwd);
        Ok(ctx)
    }

    pub(crate) fn has_env_project_context(&self) -> bool {
        self.adapter_ctx.project_root.is_some() || self.adapter_ctx.project_cwd.is_some()
    }

    pub(crate) fn env_project_context(&self) -> Option<ProjectContext> {
        let root = self.adapter_ctx.project_root.as_ref()?;
        let cwd = self.adapter_ctx.project_cwd.as_deref().unwrap_or(root);
        Some(context_from_paths(root, cwd, true))
    }

    pub(crate) fn status_adapter_ctx(&self) -> AdapterContext {
        self.effective_adapter_ctx()
            .unwrap_or_else(|_| self.adapter_ctx.clone())
    }

    pub(crate) fn scan_activity(
        &self,
        operation: &'static str,
        scan_label: &str,
        roots: Vec<PathBuf>,
        started_at: i64,
        counts: ScanActivityCounts,
        agent_summaries: Option<Vec<AgentRefreshSummary>>,
    ) -> RefreshActivity {
        let roots_count = roots.len();
        let diagnostic_redaction_roots = self.scan_diagnostic_redaction_roots(&roots, &[]);
        let redacted_roots = roots
            .iter()
            .map(|path| {
                observability_redact(&lexical_path_text(path), &diagnostic_redaction_roots, 320)
            })
            .collect();
        let has_partial_scan = agent_summaries.as_ref().is_some_and(|summaries| {
            summaries
                .iter()
                .any(|summary| !summary.roots_partial.is_empty())
        });
        let mut log_entries = vec![
            RefreshLogEntry {
                level: "info",
                message: format!("Queued {scan_label} scan across {roots_count} root(s)."),
            },
            RefreshLogEntry {
                level: "info",
                message: format!(
                    "Catalog refresh completed with {} skill(s) and {} runtime conflict group(s).",
                    counts.skill_count, counts.conflict_count
                ),
            },
        ];
        if counts.scanned_count == 0 {
            log_entries.push(RefreshLogEntry {
                level: "warning",
                message: format!(
                    "No skills were discovered for {scan_label}. Check the configured roots, then retry Scan."
                ),
            });
        }
        if let Some(summaries) = &agent_summaries {
            log_entries.extend(summaries.iter().map(|summary| {
                let level = if summary.status == "completed" {
                    "info"
                } else {
                    "warning"
                };
                let skipped_detail = skipped_roots_detail(&summary.roots_skipped);
                let issue_detail = summary
                    .scan_issues
                    .first()
                    .map(|issue| {
                        format!(
                            "; first scan issue {} at {}: {}",
                            issue.kind,
                            issue.path,
                            issue.detail.trim_end_matches('.')
                        )
                    })
                    .unwrap_or_default();
                RefreshLogEntry {
                    level,
                    message: format!(
                        "{} discovered {} skill(s); catalog now has {} skill(s), {} broken, across {} complete root(s), {} partial root(s), and {} skipped root(s){}{}.",
                        summary.display_label,
                        summary.scanned_count,
                        summary.catalog_count,
                        summary.broken_count,
                        summary.roots_scanned.len(),
                        summary.roots_partial.len(),
                        summary.roots_skipped.len(),
                        skipped_detail,
                        issue_detail,
                    ),
                }
            }));
        }

        let mut recovery_actions = vec![
            "Retry Scan if the catalog looks stale.".to_string(),
            "Use Reload to re-read the current catalog without touching agent files.".to_string(),
        ];
        if has_partial_scan {
            recovery_actions.insert(
                0,
                "Review partial-root diagnostics; unseen rows under partial roots were preserved."
                    .to_string(),
            );
        }

        RefreshActivity {
            operation,
            status: if has_partial_scan {
                "completed-partial"
            } else {
                "completed"
            },
            started_at,
            finished_at: unix_timestamp_millis(),
            scanned_count: counts.scanned_count,
            skill_count: counts.skill_count,
            finding_count: counts.finding_count,
            conflict_count: counts.conflict_count,
            snapshot_count: counts.snapshot_count,
            roots: redacted_roots,
            log_entries,
            recovery_actions,
            agent_summaries,
        }
    }

    pub(crate) fn agent_refresh_summaries(
        &self,
        agent_reports: &[AgentCatalogScanReport],
        skills: &[SkillRecord],
        adapter_diagnostics: &[AdapterDiagnosticsRecord],
    ) -> Vec<AgentRefreshSummary> {
        let all_considered_roots = agent_reports
            .iter()
            .flat_map(|report| report.roots_considered.iter().cloned())
            .collect::<Vec<_>>();
        let all_root_aliases = agent_reports
            .iter()
            .flat_map(|report| report.root_aliases.iter().cloned())
            .collect::<Vec<_>>();
        let diagnostic_redaction_roots =
            self.scan_diagnostic_redaction_roots(&all_considered_roots, &all_root_aliases);
        agent_reports
            .iter()
            .map(|agent_report| {
                let agent = agent_report.agent.as_str();
                let diagnostics = adapter_diagnostics
                    .iter()
                    .find(|diagnostics| diagnostics.agent == agent);
                let catalog_count = skills.iter().filter(|skill| skill.agent == agent).count();
                let broken_count = skills
                    .iter()
                    .filter(|skill| skill.agent == agent && skill.state == "broken")
                    .count();
                let recovery_actions = if !agent_report.partial_roots.is_empty() {
                    vec![format!(
                        "Review {} partial-root diagnostics, then retry Scan; unseen catalog rows under partial roots were preserved.",
                        agent_report.display_name
                    )]
                } else if agent_report
                    .issues
                    .iter()
                    .any(|issue| issue.kind == "dangling_symlink")
                {
                    vec![format!(
                        "Review the {} dangling Agent link and remove only that link after confirming its source is unavailable; the remaining root was fully reconciled.",
                        agent_report.display_name
                    )]
                } else if agent_report.scanned_roots.is_empty() {
                    vec![format!(
                        "Create a {} skills root or check skipped-root permissions, then retry Scan.",
                        agent_report.display_name
                    )]
                } else if !agent_report.skipped_roots.is_empty() {
                    vec![format!(
                        "Review {} skipped-root diagnostics, then retry Scan.",
                        agent_report.display_name
                    )]
                } else {
                    Vec::new()
                };
                let status = if !agent_report.partial_roots.is_empty() {
                    "completed-partial"
                } else if agent_report.scanned_roots.is_empty() {
                    "completed-no-roots-scanned"
                } else if !agent_report.skipped_roots.is_empty() {
                    "completed-with-skipped-roots"
                } else {
                    "completed"
                };
                let redact_path = |path: &Path| {
                    observability_redact(
                        &lexical_path_text(path),
                        &diagnostic_redaction_roots,
                        320,
                    )
                };
                AgentRefreshSummary {
                    agent: agent.to_string(),
                    display_label: agent_report.display_name.to_string(),
                    status,
                    scanned_count: agent_report.scanned_count,
                    catalog_count,
                    broken_count,
                    roots_considered: agent_report
                        .roots_considered
                        .iter()
                        .map(|path| redact_path(path))
                        .collect(),
                    roots_scanned: agent_report
                        .scanned_roots
                        .iter()
                        .map(|path| redact_path(path))
                        .collect(),
                    roots_partial: agent_report
                        .partial_roots
                        .iter()
                        .map(|path| redact_path(path))
                        .collect(),
                    roots_skipped: agent_report
                        .skipped_roots
                        .iter()
                        .map(|path| redact_path(path))
                        .collect(),
                    scan_issues: agent_report
                        .issues
                        .iter()
                        .map(|issue| AgentRefreshScanIssue {
                            kind: issue.kind,
                            path: redact_path(&issue.path),
                            detail: public_scan_issue_detail(issue.kind).to_string(),
                        })
                        .collect(),
                    config_detected: diagnostics
                        .is_some_and(|diagnostics| diagnostics.config.detected_count > 0),
                    config_paths: diagnostics
                        .map(|diagnostics| {
                            diagnostics
                                .config
                                .paths
                                .iter()
                                .map(|path| path.path.clone())
                                .collect()
                        })
                        .unwrap_or_default(),
                    writable_status: diagnostics
                        .map(|diagnostics| diagnostics.access.writable_status.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    writable_reason: diagnostics
                        .and_then(|diagnostics| diagnostics.access.writable_reason)
                        .map(str::to_string),
                    read_only_reason: diagnostics
                        .map(|diagnostics| diagnostics.access.read_only_reason.clone())
                        .unwrap_or_else(|| "Adapter diagnostics were unavailable.".to_string()),
                    blockers: diagnostics
                        .map(|diagnostics| {
                            diagnostics
                                .blockers
                                .iter()
                                .map(|blocker| (*blocker).to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                    recovery_actions,
                }
            })
            .collect()
    }

    fn scan_diagnostic_redaction_roots(
        &self,
        adapter_roots: &[PathBuf],
        root_aliases: &[AgentCatalogScanPathAlias],
    ) -> Vec<(String, &'static str)> {
        let adapter_ctx = self.status_adapter_ctx();
        let privacy_roots = [
            (&self.app_data_dir, "<app-data-dir>"),
            (&adapter_ctx.user_home, "$HOME"),
        ];
        let mut roots = privacy_roots
            .iter()
            .map(|(path, placeholder)| (lexical_path_text(path), *placeholder))
            .collect::<Vec<_>>();
        if let Some(project_root) = adapter_ctx.project_root.as_ref() {
            roots.push((lexical_path_text(project_root), "<project-root>"));
        }
        if let Some(project_cwd) = adapter_ctx.project_cwd.as_ref() {
            roots.push((lexical_path_text(project_cwd), "<project-cwd>"));
        }

        let private_paths = roots
            .iter()
            .map(|(path, _)| PathBuf::from(path))
            .collect::<Vec<_>>();
        let mut push_external_root = |path: &Path| {
            let path = lexical_path_text(path);
            let normalized = Path::new(&path);
            if !path.is_empty()
                && !private_paths
                    .iter()
                    .any(|private_root| normalized.starts_with(private_root))
            {
                roots.push((path, "<adapter-root>"));
            }
        };
        for path in adapter_roots {
            push_external_root(path);
        }
        for alias in root_aliases {
            push_external_root(&alias.declared);
            push_external_root(&alias.canonical);
        }
        roots.sort_by_key(|right| std::cmp::Reverse(right.0.len()));
        let mut seen = BTreeSet::new();
        roots.retain(|(path, _)| !path.is_empty() && seen.insert(path.clone()));
        roots
    }
}

fn public_scan_issue_detail(kind: &str) -> &'static str {
    match kind {
        "root_unavailable" => "A declared scan root was unavailable or not a directory.",
        "root_outside_allowlist" => {
            "A resolved path was outside the explicit same-scope adapter roots."
        }
        "dangling_symlink" => {
            "An Agent skill link points to an unavailable source; the link was skipped and the rest of the root was reconciled."
        }
        "directory_unreadable" => "A scan directory could not be read.",
        "entry_unreadable" => "A directory entry could not be inspected or resolved.",
        "file_unreadable" => "A skill file could not be inspected or read.",
        "file_too_large" => "A skill file exceeded the per-file scan size limit.",
        "budget_exceeded" => "The scan stopped after reaching a configured work budget.",
        _ => "The scanner reported a degraded condition.",
    }
}

fn lexical_path_text(path: &Path) -> String {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let can_pop = normalized
                    .file_name()
                    .is_some_and(|name| name != std::ffi::OsStr::new(".."));
                if can_pop {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push("..");
                }
            }
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::Normal(value) => normalized.push(value),
        }
    }
    normalized_redaction_path_text(&normalized.to_string_lossy())
}

fn normalized_redaction_path_text(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = normalized.strip_prefix("//?/") {
        rest.to_string()
    } else {
        normalized
    }
}

fn config_snapshot_page_result(
    catalog: &Catalog,
    adapter_ctx: &AdapterContext,
    params: ListAgentConfigPageParams,
) -> Result<ConfigSnapshotPageResult, ServiceError> {
    const METHOD: &str = "snapshot.listAgentConfigPage";
    let scope = params.scope.as_deref().filter(|scope| !scope.is_empty());
    let project_root = adapter_ctx
        .project_root
        .as_deref()
        .map(Path::canonicalize)
        .transpose()?;
    let limit = params.limit.unwrap_or(100).clamp(1, 100);
    let query_digest = tagged_digest(
        METHOD,
        &(
            params.agent.as_str(),
            scope,
            project_root.as_deref().map(|path| path.to_string_lossy()),
        ),
    )?;
    let cursor = params
        .cursor
        .as_deref()
        .map(|text| decode_cursor(text, METHOD, &query_digest))
        .transpose()?;
    let before = cursor
        .as_ref()
        .map(|cursor| (cursor.sort_value, cursor.stable_id.as_str()));
    let requested_revision = params.source_revision.as_deref();
    let cursor_revision = cursor
        .as_ref()
        .map(|cursor| cursor.source_revision.as_str());
    let snapshot = list_agent_config_snapshot_page_snapshot(
        catalog,
        &params.agent,
        scope,
        project_root.as_deref(),
        before,
        limit,
        |current| validate_catalog_source_revision(requested_revision, cursor_revision, current),
    )
    .map_err(map_history_page_error)?;
    let source_revision = snapshot.source_revision;
    let total_count = snapshot.total_count;
    let mut records = snapshot.records;
    let has_more = records.len() > limit;
    if has_more {
        records.truncate(limit);
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|record| {
                encode_cursor(&KeysetCursor {
                    version: 1,
                    method: METHOD.to_string(),
                    query_digest: query_digest.clone(),
                    source_revision: source_revision.clone(),
                    sort_value: record.created_at,
                    stable_id: record.id.clone(),
                    tie_breaker_digest: None,
                    accepted_count: None,
                    processed_prefix_digest: None,
                    resolved_start_at: None,
                    resolved_end_at: None,
                })
            })
            .transpose()?
    } else {
        None
    };
    Ok(ConfigSnapshotPageResult {
        page: ListPageMetadata::enumerable(records.len(), Some(total_count), next_cursor),
        records,
        source_revision,
    })
}

fn skill_event_page_result(
    catalog: &Catalog,
    params: ListSkillEventsPageParams,
) -> Result<SkillEventPageResult, ServiceError> {
    const METHOD: &str = "skill.listEventsPage";
    let limit = params.limit.unwrap_or(100).clamp(1, 100);
    let query_digest = tagged_digest(METHOD, &params.instance_id)?;
    let cursor = params
        .cursor
        .as_deref()
        .map(|text| decode_cursor(text, METHOD, &query_digest))
        .transpose()?;
    let before = cursor
        .as_ref()
        .map(|cursor| {
            cursor
                .stable_id
                .parse::<i64>()
                .map(|id| (cursor.sort_value, id))
                .map_err(|_| ServiceError::InvalidRequest("event cursor id is invalid".to_string()))
        })
        .transpose()?;
    let requested_revision = params.source_revision.as_deref();
    let cursor_revision = cursor
        .as_ref()
        .map(|cursor| cursor.source_revision.as_str());
    let snapshot =
        list_skill_event_page_snapshot(catalog, &params.instance_id, before, limit, |current| {
            validate_catalog_source_revision(requested_revision, cursor_revision, current)
        })
        .map_err(map_history_page_error)?;
    let source_revision = snapshot.source_revision;
    let total_count = snapshot.total_count;
    let mut records = snapshot.records;
    let has_more = records.len() > limit;
    if has_more {
        records.truncate(limit);
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|record| {
                encode_cursor(&KeysetCursor {
                    version: 1,
                    method: METHOD.to_string(),
                    query_digest: query_digest.clone(),
                    source_revision: source_revision.clone(),
                    sort_value: record.occurred_at,
                    stable_id: record.id.to_string(),
                    tie_breaker_digest: None,
                    accepted_count: None,
                    processed_prefix_digest: None,
                    resolved_start_at: None,
                    resolved_end_at: None,
                })
            })
            .transpose()?
    } else {
        None
    };
    Ok(SkillEventPageResult {
        page: ListPageMetadata::enumerable(records.len(), Some(total_count), next_cursor),
        records,
        source_revision,
    })
}

fn validate_catalog_source_revision(
    requested: Option<&str>,
    cursor_revision: Option<&str>,
    current: &str,
) -> Result<(), CatalogError> {
    if requested.is_some_and(|revision| revision != current)
        || cursor_revision.is_some_and(|revision| revision != current)
    {
        return Err(CatalogError::SourceChanged);
    }
    Ok(())
}

fn map_history_page_error(error: CommandError) -> ServiceError {
    match error {
        CommandError::Catalog(CatalogError::SourceChanged) => ServiceError::SourceChanged,
        error => ServiceError::Command(error),
    }
}

fn tagged_digest<T: Serialize>(domain: &str, value: &T) -> Result<String, ServiceError> {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(serde_json::to_vec(value)?);
    Ok(format!("sha256:{}", hex_prefix(&hasher.finalize(), 64)))
}
