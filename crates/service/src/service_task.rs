use super::*;
use std::io::BufRead;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

impl ServiceHost {
    pub fn check_task_readiness(
        &self,
        params: TaskReadinessParams,
    ) -> Result<TaskReadinessResult, ServiceError> {
        let started_at = Instant::now();
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.checkReadiness requires a non-empty task".to_string(),
            ));
        }
        let adapter_ctx = self.effective_adapter_ctx()?;
        let task = redact_string(
            &redact_for_llm_preview(task),
            &self.redaction_roots(&adapter_ctx),
        );
        let limit = params.limit.unwrap_or(8).clamp(1, 20);
        let candidate_instance_ids = normalize_string_list(params.candidate_instance_ids.clone());
        let filters = TaskReadinessFilters {
            agent: params.agent.clone(),
            candidate_instance_ids: candidate_instance_ids.clone(),
            limit,
        };
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(empty_task_readiness_result(task, filters, false));
        };

        let skills = self.list_visible_skill_records(&catalog)?;
        let findings = list_findings(&catalog)?;
        let conflicts = list_conflicts(&catalog)?;
        let agent_filter = params.agent.as_deref().filter(|agent| !agent.is_empty());
        let adapter_diagnostics = match agent_filter {
            Some(agent) => list_adapter_diagnostics_for_agents(&adapter_ctx, &[agent]),
            None => list_adapter_diagnostics(&adapter_ctx),
        };
        let requested_ids = candidate_instance_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let task_terms = task_readiness_terms(&task);

        let mut missing_gap_notes = Vec::new();
        let visible_by_id = skills
            .iter()
            .map(|skill| (skill.id.as_str(), skill))
            .collect::<BTreeMap<_, _>>();
        for requested_id in &requested_ids {
            if !visible_by_id.contains_key(requested_id) {
                missing_gap_notes.push(format!(
                    "Requested candidate `{}` is not visible in the current catalog/project scope.",
                    redact_for_llm_preview(requested_id)
                ));
            }
        }

        let mut candidate_records = skills
            .into_iter()
            .filter(|skill| {
                agent_filter.is_none_or(|agent| skill.agent == agent)
                    && (requested_ids.is_empty() || requested_ids.contains(&skill.id.as_str()))
            })
            .collect::<Vec<_>>();
        let total_candidate_count = candidate_records.len();
        let scan_limit = task_readiness_candidate_scan_limit(limit, requested_ids.len());
        let mut skipped_stages = Vec::new();
        let mut blocker_codes = Vec::new();
        let mut aggregation_notes = Vec::new();
        let mut used_metadata_prefilter = false;
        if candidate_records.len() > scan_limit {
            let affinity_by_instance_id = candidate_records
                .iter()
                .map(|skill| {
                    (
                        skill.id.clone(),
                        task_readiness_record_affinity(skill, &task_terms),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            candidate_records.sort_by(|left, right| {
                let right_affinity = affinity_by_instance_id
                    .get(right.id.as_str())
                    .copied()
                    .unwrap_or_default();
                let left_affinity = affinity_by_instance_id
                    .get(left.id.as_str())
                    .copied()
                    .unwrap_or_default();
                right_affinity
                    .cmp(&left_affinity)
                    .then_with(|| right.enabled.cmp(&left.enabled))
                    .then_with(|| left.agent.cmp(&right.agent))
                    .then_with(|| left.name.cmp(&right.name))
                    .then_with(|| left.id.cmp(&right.id))
            });
            candidate_records.truncate(scan_limit);
            used_metadata_prefilter = true;
            skipped_stages.push("candidate-scan-overflow");
            blocker_codes.push("bounded-candidate-scan");
            let note = format!(
                "Task readiness prefiltered the top {} of {} visible candidate(s) using deterministic metadata.",
                candidate_records.len(),
                total_candidate_count
            );
            missing_gap_notes.push(note.clone());
            aggregation_notes.push(note);
        }
        if used_metadata_prefilter && requested_ids.is_empty() {
            let detail_scan_limit =
                task_readiness_detail_scan_limit(limit, candidate_records.len());
            if candidate_records.len() > detail_scan_limit {
                let prefiltered_count = candidate_records.len();
                candidate_records.truncate(detail_scan_limit);
                skipped_stages.push("detail-scan-prefilter");
                blocker_codes.push("bounded-detail-scan");
                let note = format!(
                    "Task readiness deep-evaluated the top {} of {} prefiltered candidate(s).",
                    candidate_records.len(),
                    prefiltered_count
                );
                missing_gap_notes.push(note.clone());
                aggregation_notes.push(note);
            }
        }

        let findings_by_instance = task_readiness_findings_by_instance(&findings);
        let findings_by_definition = task_readiness_findings_by_definition(&findings);
        let conflicts_by_instance = task_readiness_conflicts_by_instance(&conflicts);
        let conflicts_by_definition = task_readiness_conflicts_by_definition(&conflicts);
        let analysis_by_instance = if agent_filter.is_some() {
            BTreeMap::new()
        } else {
            let analysis = analyze_catalog(&catalog, &adapter_ctx)?;
            task_readiness_analysis_by_instance(&analysis.groups)
        };

        let candidate_ids = candidate_records
            .iter()
            .map(|skill| skill.id.clone())
            .collect::<Vec<_>>();
        let candidate_details = catalog.list_skill_details_by_ids(&candidate_ids)?;
        let details_by_id = candidate_details
            .iter()
            .map(|detail| (detail.id.as_str(), detail))
            .collect::<BTreeMap<_, _>>();

        let mut evidence = Vec::new();
        let mut candidates = Vec::new();
        for skill in &candidate_records {
            let Some(detail) = details_by_id.get(skill.id.as_str()).copied() else {
                missing_gap_notes.push(format!(
                    "Catalog row `{}` did not have detail evidence available.",
                    redact_for_llm_preview(&skill.id)
                ));
                continue;
            };
            let related_findings = task_readiness_related_findings(
                detail,
                &findings_by_instance,
                &findings_by_definition,
            );
            let related_conflicts = task_readiness_related_conflicts(
                detail,
                &conflicts_by_instance,
                &conflicts_by_definition,
            );
            let related_analysis = task_readiness_related_analysis(detail, &analysis_by_instance);
            let diagnostic = adapter_diagnostics
                .iter()
                .find(|diagnostic| diagnostic.agent == detail.agent);
            let quality = task_readiness_quality_signal(
                detail,
                &related_findings,
                &related_conflicts,
                &related_analysis,
                diagnostic,
            );
            let candidate = task_readiness_candidate(
                &task_terms,
                detail,
                TaskReadinessCandidateSignals {
                    findings: &related_findings,
                    conflicts: &related_conflicts,
                    analysis_groups: &related_analysis,
                    diagnostic,
                    quality: Some(&quality),
                },
                &mut evidence,
            );
            candidates.push(candidate);
        }

        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.agent.cmp(&right.agent))
                .then_with(|| left.skill_name.cmp(&right.skill_name))
                .then_with(|| left.instance_id.cmp(&right.instance_id))
        });
        candidates.truncate(limit);

        if candidates.is_empty() {
            if agent_filter.is_some() {
                missing_gap_notes.push(
                    "No visible skill candidates matched the requested agent/filter scope."
                        .to_string(),
                );
            } else {
                missing_gap_notes.push(
                    "No visible skill candidates matched the task in the current catalog."
                        .to_string(),
                );
            }
        }

        let blocker_risk_notes = task_readiness_blocker_notes(&candidates);
        let score = task_readiness_overall_score(&candidates);
        let band = task_readiness_band(score);
        let summary = task_readiness_summary(score, band, &candidates, &missing_gap_notes);
        let prompt_instance_ids = candidates
            .iter()
            .take(8)
            .map(|candidate| candidate.instance_id.clone())
            .collect::<Vec<_>>();

        Ok(TaskReadinessResult {
            task: task.clone(),
            score,
            band,
            summary,
            generated_by: "deterministic-service",
            catalog_available: true,
            filters,
            candidate_skills: candidates,
            missing_gap_notes,
            blocker_risk_notes,
            evidence_references: evidence,
            prompt_request: TaskReadinessPromptRequest {
                available: true,
                preview_method: "llm.previewPrompt",
                confirm_method: "llm.confirmPromptAndSend",
                action: "task_readiness",
                request: LlmPreviewPromptParams {
                    action: LlmPromptActionKind::TaskReadiness,
                    profile_id: None,
                    app_language: None,
                    skill_instance_id: None,
                    instance_ids: prompt_instance_ids,
                    agents: Vec::new(),
                analysis_kind: None,
                    user_intent: Some(task.clone()),
                },
                note: "Optional provider-backed explanation must be requested through prompt preview and explicit confirmation; task.checkReadiness never sends provider traffic."
                    .to_string(),
            },
            aggregation: aggregation_runtime_metadata(AggregationRuntimeInput {
                started_at,
                timeout_ms: TASK_AGGREGATION_TIMEOUT_MS,
                limit,
                scanned_count: candidate_records.len(),
                total_count: total_candidate_count,
                completed_stages: vec![
                    "catalog",
                    "finding-index",
                    "conflict-index",
                    "analysis-index",
                    "candidate-scan",
                    "quality-signal",
                ],
                skipped_stages,
                blocker_codes,
                fallback_used: !aggregation_notes.is_empty(),
                notes: aggregation_notes,
            }),
            safety_flags: task_readiness_safety_flags(),
        })
    }

    pub fn rank_skill_routes(
        &self,
        params: RankSkillRoutesParams,
    ) -> Result<SkillRouteRankingResult, ServiceError> {
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.rankSkillRoutes requires a non-empty task".to_string(),
            ));
        }
        let readiness = self.check_task_readiness(TaskReadinessParams {
            task: task.to_string(),
            agent: params.agent,
            candidate_instance_ids: params.candidate_instance_ids,
            limit: params.limit,
        })?;
        Ok(skill_route_ranking_from_readiness(readiness))
    }

    pub fn compare_agent_readiness(
        &self,
        params: CompareAgentReadinessParams,
    ) -> Result<AgentReadinessComparisonResult, ServiceError> {
        let started_at = Instant::now();
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.compareAgentReadiness requires a non-empty task".to_string(),
            ));
        }

        let adapter_ctx = self.effective_adapter_ctx()?;
        let task = redact_string(
            &redact_for_llm_preview(task),
            &self.redaction_roots(&adapter_ctx),
        );
        let limit_per_agent = params.limit_per_agent.unwrap_or(3).clamp(1, 10);
        let requested_agents = normalize_agent_filter_list(params.agents);
        let filters = AgentReadinessComparisonFilters {
            agents: requested_agents.clone(),
            limit_per_agent,
            include_routing_accuracy: params.include_routing_accuracy,
            include_benchmarks: params.include_benchmarks,
        };

        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(empty_agent_readiness_comparison(
                task,
                filters,
                false,
                "No local catalog is available; cross-agent readiness comparison has no candidate evidence.",
            ));
        };

        let skills = self.list_visible_skill_records(&catalog)?;
        let agents =
            agent_readiness_agents_for_comparison(&skills, &adapter_ctx, &requested_agents);
        let total_agent_count = agents.len();
        if agents.is_empty() {
            return Ok(empty_agent_readiness_comparison(
                task,
                filters,
                true,
                "No supported agent skills matched the selected filters in the current catalog.",
            ));
        }

        let accuracy_by_agent = if params.include_routing_accuracy {
            Some(agent_readiness_accuracy_context(
                self.routing_accuracy_dashboard(RoutingAccuracyDashboardParams {
                    agent: None,
                    window_days: Some(30),
                    limit: Some(100),
                    include_history: false,
                    include_recent_evidence: true,
                })?,
            ))
        } else {
            None
        };
        let benchmark_by_agent = if params.include_benchmarks {
            Some(agent_readiness_benchmark_context(
                self.evaluate_task_benchmarks(EvaluateTaskBenchmarksParams {
                    ids: Vec::new(),
                    limit: Some(25),
                })?,
            ))
        } else {
            None
        };

        let mut evidence_by_id = BTreeMap::new();
        let mut rows = Vec::new();
        let mut gap_issue_rows = Vec::new();
        for agent in agents {
            let readiness = self.check_task_readiness(TaskReadinessParams {
                task: task.clone(),
                agent: Some(agent.clone()),
                candidate_instance_ids: Vec::new(),
                limit: Some(limit_per_agent),
            })?;
            let ranking = skill_route_ranking_from_readiness(readiness.clone());
            for evidence in readiness.evidence_references.iter().cloned() {
                evidence_by_id
                    .entry(evidence.id.clone())
                    .or_insert(evidence);
            }
            let accuracy_context = accuracy_by_agent
                .as_ref()
                .and_then(|by_agent| by_agent.get(&agent).cloned());
            let benchmark_context = benchmark_by_agent
                .as_ref()
                .and_then(|by_agent| by_agent.get(&agent).cloned());
            let row = agent_readiness_row_from_results(
                &agent,
                &readiness,
                &ranking,
                accuracy_context,
                benchmark_context,
            );
            gap_issue_rows.extend(agent_readiness_gap_issue_rows(&row));
            rows.push(row);
        }

        rows.sort_by(|left, right| {
            right
                .comparison_score
                .cmp(&left.comparison_score)
                .then_with(|| right.readiness_score.cmp(&left.readiness_score))
                .then_with(|| {
                    right
                        .routing_confidence_score
                        .cmp(&left.routing_confidence_score)
                })
                .then_with(|| left.agent.cmp(&right.agent))
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = index + 1;
        }
        let recommended_agent = rows
            .iter()
            .find(|row| row.candidate_count > 0 && row.comparison_score > 0)
            .map(agent_readiness_recommendation);
        let prompt_instance_ids = rows
            .iter()
            .filter_map(|row| row.best_candidate.as_ref())
            .take(8)
            .map(|candidate| candidate.instance_id.clone())
            .collect::<Vec<_>>();
        let prompt_available = !prompt_instance_ids.is_empty();
        let summary = agent_readiness_summary(&rows, &gap_issue_rows, &recommended_agent);
        let evidence_references = evidence_by_id.into_values().collect::<Vec<_>>();
        let returned_agent_count = rows.len();
        let mut completed_stages = vec!["catalog", "agent-readiness", "agent-ranking"];
        if params.include_routing_accuracy {
            completed_stages.push("routing-accuracy-context");
        }
        if params.include_benchmarks {
            completed_stages.push("benchmark-context");
        }
        let aggregation_notes = if params.include_benchmarks {
            vec![
                "Benchmark context is capped at 25 app-local benchmark rows for bounded comparison latency."
                    .to_string(),
            ]
        } else {
            Vec::new()
        };

        Ok(AgentReadinessComparisonResult {
            generated_by: "deterministic-service",
            catalog_available: true,
            filters,
            summary,
            agent_rows: rows,
            recommended_agent,
            gap_issue_rows,
            evidence_references,
            prompt_request: AgentReadinessPromptRequest {
                available: prompt_available,
                preview_method: "llm.previewPrompt",
                confirm_method: "llm.confirmPromptAndSend",
                action: "task_readiness",
                request: LlmPreviewPromptParams {
                    action: LlmPromptActionKind::TaskReadiness,
                    profile_id: None,
                    app_language: None,
                    skill_instance_id: None,
                    instance_ids: prompt_instance_ids,
                    agents: Vec::new(),
                    analysis_kind: None,
                    user_intent: Some(task),
                },
                note: if prompt_available {
                    "Optional provider-backed explanation must be requested through prompt preview and explicit confirmation; task.compareAgentReadiness never sends provider traffic."
                        .to_string()
                } else {
                    "Prompt preview is unavailable until local catalog evidence produces cross-agent candidates."
                        .to_string()
                },
            },
            aggregation: aggregation_runtime_metadata(AggregationRuntimeInput {
                started_at,
                timeout_ms: TASK_AGGREGATION_TIMEOUT_MS,
                limit: limit_per_agent,
                scanned_count: returned_agent_count,
                total_count: total_agent_count,
                completed_stages,
                skipped_stages: Vec::new(),
                blocker_codes: Vec::new(),
                fallback_used: false,
                notes: aggregation_notes,
            }),
            safety_flags: agent_readiness_safety_flags(),
        })
    }

    pub fn build_task_cockpit(
        &self,
        params: TaskCockpitParams,
    ) -> Result<TaskCockpitResult, ServiceError> {
        const MIN_TIMEOUT_MS: u64 = 500;
        const MAX_TIMEOUT_MS: u64 = 30_000;

        let started_at = Instant::now();
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.buildCockpit requires a non-empty task".to_string(),
            ));
        }

        let limit = params.limit.unwrap_or(8).clamp(1, 25);
        let timeout_ms = params
            .timeout_ms
            .unwrap_or(TASK_COCKPIT_TIMEOUT_MS)
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS);
        let budget = Duration::from_millis(timeout_ms);
        let include_session_review = params.include_session_review.unwrap_or(true);
        let include_provider_observability = params.include_provider_observability.unwrap_or(true);
        let include_remediation_context = params.include_remediation_context.unwrap_or(true);
        let agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|agent| !agent.is_empty())
            .map(ToOwned::to_owned);
        let candidate_instance_ids = normalize_string_list(params.candidate_instance_ids);
        let mut fallback_reasons = Vec::new();

        let readiness = self.check_task_readiness(TaskReadinessParams {
            task: task.to_string(),
            agent: agent.clone(),
            candidate_instance_ids: candidate_instance_ids.clone(),
            limit: Some(limit),
        })?;
        let ranking = skill_route_ranking_from_readiness(readiness.clone());
        let comparison = self.compare_agent_readiness(CompareAgentReadinessParams {
            task: readiness.task.clone(),
            agents: agent.clone().into_iter().collect(),
            limit_per_agent: Some(limit.min(3)),
            include_routing_accuracy: false,
            include_benchmarks: false,
        })?;
        let session_review_rows = if include_session_review
            && readiness.catalog_available
            && !task_cockpit_budget_reached(started_at, budget)
        {
            self.list_agent_skill_reviews(AgentSessionListSkillReviewsParams {
                agent: agent.clone(),
                outcome: None,
                trace_import_id: None,
                limit: Some(limit),
            })?
            .reviews
            .iter()
            .take(limit)
            .map(task_cockpit_session_review_row)
            .collect::<Vec<_>>()
        } else {
            if !include_session_review {
                fallback_reasons.push(
                    "Session-review context was skipped by the cockpit request filters."
                        .to_string(),
                );
            } else if task_cockpit_budget_reached(started_at, budget) {
                fallback_reasons.push(
                    "Task cockpit reached its bounded time budget before session-review context."
                        .to_string(),
                );
            }
            Vec::new()
        };
        let provider_observability = if include_provider_observability
            && readiness.catalog_available
            && !task_cockpit_budget_reached(started_at, budget)
        {
            Some(
                self.llm_provider_observability(LlmProviderObservabilityParams {
                    profile_id: None,
                    provider: None,
                    model: None,
                    status: None,
                    action: None,
                    limit: Some(limit),
                })?,
            )
        } else {
            if !include_provider_observability {
                fallback_reasons.push(
                    "Provider-observability context was skipped by the cockpit request filters."
                        .to_string(),
                );
            } else if task_cockpit_budget_reached(started_at, budget) {
                fallback_reasons.push(
                    "Task cockpit reached its bounded time budget before provider-observability context."
                        .to_string(),
                );
            }
            None
        };
        let remediation_candidate_ids = if candidate_instance_ids.is_empty() {
            ranking
                .route_candidates
                .iter()
                .take(limit.min(5))
                .map(|candidate| candidate.instance_id.clone())
                .collect::<Vec<_>>()
        } else {
            candidate_instance_ids.clone()
        };
        let remediation_plan = if include_remediation_context
            && readiness.catalog_available
            && !task_cockpit_budget_reached(started_at, budget)
        {
            Some(self.plan_remediation(RemediationPlanParams {
                agent: agent.clone(),
                task: Some(readiness.task.clone()),
                project_root: None,
                focus: None,
                focus_areas: vec![
                    "finding".to_string(),
                    "gap".to_string(),
                    "ambiguity".to_string(),
                    "drift".to_string(),
                    "readiness".to_string(),
                ],
                limit: Some(limit.min(5)),
                candidate_instance_ids: remediation_candidate_ids.clone(),
                include_deferred: false,
            })?)
        } else {
            if !include_remediation_context {
                fallback_reasons.push(
                    "Remediation context was skipped by the cockpit request filters.".to_string(),
                );
            } else if task_cockpit_budget_reached(started_at, budget) {
                fallback_reasons.push(
                    "Task cockpit reached its bounded time budget before remediation planning."
                        .to_string(),
                );
            }
            None
        };
        let batch_review = if include_remediation_context
            && remediation_plan.is_some()
            && !task_cockpit_budget_reached(started_at, budget)
        {
            Some(self.batch_review_remediation(RemediationBatchReviewParams {
                task: Some(readiness.task.clone()),
                agent: agent.clone(),
                project_root: None,
                workspace_label: None,
                rule_id: None,
                severity: None,
                status: None,
                triage_status: None,
                candidate_instance_ids: remediation_candidate_ids,
                group_by: vec![
                    "risk".to_string(),
                    "rule".to_string(),
                    "agent".to_string(),
                    "task".to_string(),
                ],
                limit: Some(limit.min(3)),
            })?)
        } else {
            if include_remediation_context
                && remediation_plan.is_some()
                && task_cockpit_budget_reached(started_at, budget)
            {
                fallback_reasons.push(
                    "Task cockpit returned remediation plan rows and skipped batch review after reaching its bounded time budget."
                        .to_string(),
                );
            }
            None
        };

        let mut evidence_references = Vec::new();
        evidence_references.extend(readiness.evidence_references.clone());
        evidence_references.extend(ranking.evidence_references.clone());
        evidence_references.extend(comparison.evidence_references.clone());
        if let Some(remediation_plan) = remediation_plan.as_ref() {
            evidence_references.extend(remediation_plan.evidence_references.clone());
        }
        if let Some(batch_review) = batch_review.as_ref() {
            evidence_references.extend(batch_review.evidence_references.clone());
        }
        if let Some(provider_observability) = provider_observability.as_ref() {
            evidence_references.extend(provider_observability_evidence_as_task_refs(
                &provider_observability.evidence_references,
            ));
        }
        dedupe_evidence_references(&mut evidence_references);

        let mut gap_notes = Vec::new();
        gap_notes.extend(readiness.missing_gap_notes.clone());
        gap_notes.extend(ranking.likely_miss_risks.clone());
        gap_notes.extend(
            comparison
                .gap_issue_rows
                .iter()
                .filter(|row| row.severity != "blocker")
                .map(|row| row.detail.clone()),
        );
        if let Some(provider_observability) = provider_observability.as_ref() {
            gap_notes.extend(provider_observability.gap_notes.clone());
        }
        if let Some(remediation_plan) = remediation_plan.as_ref() {
            gap_notes.extend(remediation_plan.gap_notes.clone());
        }
        if let Some(batch_review) = batch_review.as_ref() {
            gap_notes.extend(batch_review.gap_notes.clone());
        }
        normalize_note_list(&mut gap_notes);

        let mut blocker_notes = Vec::new();
        blocker_notes.extend(readiness.blocker_risk_notes.clone());
        blocker_notes.extend(ranking.likely_wrong_pick_risks.clone());
        blocker_notes.extend(
            comparison
                .gap_issue_rows
                .iter()
                .filter(|row| row.severity == "blocker")
                .map(|row| row.detail.clone()),
        );
        if let Some(provider_observability) = provider_observability.as_ref() {
            blocker_notes.extend(provider_observability.blocker_notes.clone());
        }
        if let Some(remediation_plan) = remediation_plan.as_ref() {
            blocker_notes.extend(remediation_plan.blocker_notes.clone());
        }
        if let Some(batch_review) = batch_review.as_ref() {
            blocker_notes.extend(batch_review.blocker_notes.clone());
        }
        blocker_notes.extend(fallback_reasons.iter().cloned());
        normalize_note_list(&mut blocker_notes);

        let task_rows = vec![TaskCockpitTaskRow {
            id: "task",
            task: readiness.task.clone(),
            readiness_score: readiness.score,
            readiness_band: readiness.band,
            routing_confidence_score: ranking.overall_confidence_score,
            routing_confidence_band: ranking.overall_confidence_band,
            recommended_agent: comparison
                .recommended_agent
                .as_ref()
                .map(|recommendation| recommendation.agent.clone()),
            top_skill_name: ranking
                .route_candidates
                .first()
                .map(|candidate| candidate.skill_name.clone()),
            candidate_count: readiness.candidate_skills.len(),
            gap_count: gap_notes.len(),
            blocker_count: blocker_notes.len(),
            evidence_refs: evidence_references
                .iter()
                .take(limit)
                .map(|evidence| evidence.id.clone())
                .collect(),
        }];
        let agent_route_rows = comparison
            .agent_rows
            .iter()
            .take(limit)
            .map(task_cockpit_agent_route_row)
            .collect::<Vec<_>>();
        let skill_candidate_rows = ranking
            .route_candidates
            .iter()
            .take(limit)
            .map(|candidate| {
                task_cockpit_skill_candidate_row(
                    candidate,
                    readiness
                        .candidate_skills
                        .iter()
                        .find(|ready| ready.instance_id == candidate.instance_id),
                )
            })
            .collect::<Vec<_>>();
        let readiness_rows = task_cockpit_readiness_rows(&readiness, &ranking, &comparison, limit);
        let provider_observability_rows = provider_observability
            .as_ref()
            .map(|observability| task_cockpit_provider_observability_rows(observability, limit))
            .unwrap_or_default();
        let remediation_next_steps = task_cockpit_remediation_next_steps(
            remediation_plan.as_ref(),
            batch_review.as_ref(),
            limit,
        );

        let safety_flags = task_cockpit_safety_flags();
        let cockpit_sections = task_cockpit_sections(
            &readiness,
            &ranking,
            &comparison,
            &session_review_rows,
            &provider_observability_rows,
            &remediation_next_steps,
            safety_flags,
        );
        let summary = task_cockpit_summary(
            &readiness,
            &ranking,
            &comparison,
            TaskCockpitSummaryCounts {
                session_review_count: session_review_rows.len(),
                provider_observability_row_count: provider_observability_rows.len(),
                remediation_next_step_count: remediation_next_steps.len(),
                gap_count: gap_notes.len(),
                blocker_count: blocker_notes.len(),
            },
        );
        let prompt_instance_ids = skill_candidate_rows
            .iter()
            .take(8)
            .map(|candidate| candidate.instance_id.clone())
            .collect::<Vec<_>>();
        let elapsed_ms = task_cockpit_elapsed_ms(started_at);
        if elapsed_ms >= timeout_ms && fallback_reasons.is_empty() {
            fallback_reasons.push(
                "Task cockpit completed after its bounded time budget; results are shown with timeout diagnostics."
                    .to_string(),
            );
        }
        let partial = !fallback_reasons.is_empty();
        let fallback_reason = if partial {
            Some(fallback_reasons.join(" "))
        } else {
            None
        };
        let mut completed_stages = vec!["task-readiness", "routing", "agent-comparison"];
        if !session_review_rows.is_empty() {
            completed_stages.push("session-review");
        }
        if provider_observability.is_some() {
            completed_stages.push("provider-observability");
        }
        if remediation_plan.is_some() {
            completed_stages.push("remediation-plan");
        }
        if batch_review.is_some() {
            completed_stages.push("batch-review");
        }
        let mut skipped_stages = Vec::new();
        if (!include_session_review || session_review_rows.is_empty()) && partial {
            skipped_stages.push("session-review");
        }
        if (!include_provider_observability || provider_observability.is_none()) && partial {
            skipped_stages.push("provider-observability");
        }
        if (!include_remediation_context || remediation_plan.is_none()) && partial {
            skipped_stages.push("remediation-plan");
        }
        if (!include_remediation_context || (remediation_plan.is_some() && batch_review.is_none()))
            && partial
        {
            skipped_stages.push("batch-review");
        }
        let blocker_codes = if partial {
            vec!["task-cockpit-partial"]
        } else {
            Vec::new()
        };

        Ok(TaskCockpitResult {
            generated_by: "local-v2.73",
            catalog_available: readiness.catalog_available || comparison.catalog_available,
            partial,
            elapsed_ms,
            fallback_reason,
            filters: TaskCockpitFilters {
                task: readiness.task.clone(),
                agent,
                candidate_instance_ids,
                limit,
                include_session_review,
                include_provider_observability,
                include_remediation_context,
                timeout_ms,
            },
            summary,
            cockpit_sections,
            task_rows,
            agent_route_rows,
            skill_candidate_rows,
            readiness_rows,
            session_review_rows,
            provider_observability_rows,
            remediation_next_steps,
            gap_notes,
            blocker_notes,
            evidence_references,
            prompt_request: AgentReadinessPromptRequest {
                available: !prompt_instance_ids.is_empty(),
                preview_method: "llm.previewPrompt",
                confirm_method: "llm.confirmPromptAndSend",
                action: "task_cockpit",
                request: LlmPreviewPromptParams {
                    action: LlmPromptActionKind::TaskCockpit,
                    profile_id: None,
                    app_language: None,
                    skill_instance_id: None,
                    instance_ids: prompt_instance_ids,
                    agents: Vec::new(),
                analysis_kind: None,
                    user_intent: Some(readiness.task),
                },
                note: "Optional provider-backed cockpit explanation must be requested through prompt preview and explicit confirmation; task.buildCockpit never sends provider traffic."
                    .to_string(),
            },
            aggregation: aggregation_runtime_metadata(AggregationRuntimeInput {
                started_at,
                timeout_ms,
                limit,
                scanned_count: readiness.aggregation.scanned_count,
                total_count: readiness.aggregation.total_count,
                completed_stages,
                skipped_stages,
                blocker_codes,
                fallback_used: partial,
                notes: fallback_reasons,
            }),
            safety_flags,
        })
    }

    pub fn build_skill_lifecycle_timeline(
        &self,
        params: SkillLifecycleTimelineParams,
    ) -> Result<SkillLifecycleTimelineResult, ServiceError> {
        if matches!(params.limit, Some(0)) {
            return Err(ServiceError::InvalidRequest(
                "skill.lifecycleTimeline limit must be greater than zero".to_string(),
            ));
        }

        let adapter_ctx = self.effective_adapter_ctx()?;
        let roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = skill_lifecycle_filters(&params, &adapter_ctx, &roots);
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(empty_skill_lifecycle_timeline_result(filters, false));
        };

        let mut evidence_references = Vec::new();
        let mut gap_notes = Vec::new();
        let mut blocker_notes = Vec::new();
        let skills = catalog
            .list_skill_instances_for_project_context(adapter_ctx.project_root.as_deref())?
            .into_iter()
            .filter(|skill| !is_pi_plain_markdown_instance_noise(skill))
            .map(skill_lifecycle_meta_from_instance)
            .collect::<Vec<_>>();
        let skill_by_id = skills
            .iter()
            .map(|skill| (skill.instance_id.clone(), skill.clone()))
            .collect::<BTreeMap<_, _>>();
        let visible_ids = skill_lifecycle_visible_ids(&filters, &skills);

        if !skill_lifecycle_has_skill_filter(&filters) && skills.is_empty() {
            gap_notes.push(
                "No visible local skill rows are available for the current project context."
                    .to_string(),
            );
        }
        if skill_lifecycle_has_skill_filter(&filters) && visible_ids.is_empty() {
            gap_notes
                .push("No visible local skill matched the selected lifecycle filters.".to_string());
        }

        let findings = list_findings(&catalog)?;
        let conflicts = list_conflicts(&catalog)?;
        let analysis = analyze_catalog(&catalog, &adapter_ctx)?;
        let mut rows = Vec::new();

        for skill in &skills {
            if !visible_ids.contains(&skill.instance_id) {
                continue;
            }
            let evidence_id = push_task_readiness_evidence(
                &mut evidence_references,
                "skill",
                &skill.instance_id,
                format!(
                    "Catalog lifecycle metadata for `{}` ({}, {}, enabled={})",
                    redact_for_llm_preview(&skill.skill_name),
                    redact_for_llm_preview(&skill.agent),
                    redact_for_llm_preview(&skill.scope),
                    skill.enabled
                ),
                None,
                Some(skill.instance_id.clone()),
            );
            rows.push(skill_lifecycle_skill_seen_row(skill, &evidence_id));
            if skill.last_seen != skill.first_seen {
                rows.push(skill_lifecycle_skill_observed_row(skill, &evidence_id));
            }
        }

        for finding in findings
            .iter()
            .filter(|finding| skill_lifecycle_finding_matches(&filters, finding, &skill_by_id))
        {
            let skill = skill_lifecycle_finding_skill(finding, &skill_by_id);
            let evidence_id = push_task_readiness_evidence(
                &mut evidence_references,
                "finding",
                &finding.id,
                format!(
                    "{} finding `{}`: {}",
                    redact_for_llm_preview(&finding.effective_severity),
                    redact_for_llm_preview(&finding.rule_id),
                    redact_for_llm_preview(&finding.message)
                ),
                Some(finding.effective_severity.clone()),
                skill
                    .map(|skill| skill.instance_id.clone())
                    .or_else(|| finding.instance_id.clone()),
            );
            rows.push(skill_lifecycle_finding_row(finding, skill, &evidence_id));
            if let Some(updated_at) = finding.triage_updated_at {
                rows.push(skill_lifecycle_finding_triage_row(
                    finding,
                    skill,
                    updated_at,
                    &evidence_id,
                ));
            }
        }

        for conflict in conflicts
            .iter()
            .filter(|conflict| skill_lifecycle_conflict_matches(&filters, conflict, &skill_by_id))
        {
            let skill = skill_lifecycle_conflict_skill(conflict, &skill_by_id, &visible_ids);
            let evidence_id = push_task_readiness_evidence(
                &mut evidence_references,
                "conflict",
                &conflict.id,
                format!(
                    "Same-agent conflict `{}` covers {} instance(s)",
                    redact_for_llm_preview(&conflict.reason),
                    conflict.instance_ids.len()
                ),
                Some("warning".to_string()),
                skill
                    .map(|skill| skill.instance_id.clone())
                    .or_else(|| conflict.winner_id.clone()),
            );
            rows.push(skill_lifecycle_conflict_row(conflict, skill, &evidence_id));
        }

        for group in analysis.groups.iter().filter(|group| {
            skill_lifecycle_analysis_matches(&filters, group, &skill_by_id, &visible_ids)
        }) {
            let skill = skill_lifecycle_analysis_skill(group, &skill_by_id, &visible_ids);
            let evidence_id = push_task_readiness_evidence(
                &mut evidence_references,
                "analysis",
                &group.id,
                format!(
                    "{} analysis `{}`: {}",
                    redact_for_llm_preview(&group.severity),
                    redact_for_llm_preview(&group.kind),
                    redact_for_llm_preview(&group.title)
                ),
                Some(group.severity.clone()),
                skill.map(|skill| skill.instance_id.clone()),
            );
            rows.push(skill_lifecycle_analysis_row(group, skill, &evidence_id));
        }

        if filters.include_stale_drift {
            let stale = self.detect_stale_drift(DetectStaleDriftParams {
                agent: filters
                    .selected_skill_agent
                    .clone()
                    .or_else(|| filters.agent.clone()),
                candidate_instance_ids: if visible_ids.is_empty() {
                    Vec::new()
                } else {
                    visible_ids.iter().cloned().collect()
                },
                limit: Some(filters.limit.clamp(50, 100)),
                stale_days: None,
                thresholds: StaleDriftThresholds::default(),
            })?;
            extend_evidence_references(&mut evidence_references, stale.evidence_references);
            gap_notes.extend(stale.gap_notes);
            blocker_notes.extend(stale.blocker_notes);
            for stale_row in stale.stale_drift_rows.iter().filter(|row| {
                skill_lifecycle_stale_row_matches(&filters, row, &skill_by_id, &visible_ids)
            }) {
                if stale_row.stale_drift_score == 0 {
                    continue;
                }
                rows.push(skill_lifecycle_stale_drift_row(stale_row));
            }
        }

        if filters.include_remediation_history {
            if !self.remediation_history_path().exists() {
                gap_notes.push("No app-local remediation history records are saved.".to_string());
            }
            for record in self.load_remediation_history()?.iter().filter(|record| {
                skill_lifecycle_remediation_history_matches(
                    &filters,
                    record,
                    &skill_by_id,
                    &visible_ids,
                )
            }) {
                let related_instance_id = skill_lifecycle_related_instance_for_strings(
                    record
                        .source_item_refs
                        .iter()
                        .chain(record.batch_review_item_ids.iter())
                        .chain(record.evidence_refs.iter()),
                    &skill_by_id,
                );
                let evidence_id = push_task_readiness_evidence(
                    &mut evidence_references,
                    "remediation_history",
                    &record.id,
                    format!(
                        "App-local remediation history `{}` with decision `{}` and status `{}`",
                        redact_for_llm_preview(&record.title),
                        redact_for_llm_preview(&record.decision),
                        redact_for_llm_preview(&record.status)
                    ),
                    Some(record.status.clone()),
                    related_instance_id.clone(),
                );
                rows.push(skill_lifecycle_remediation_history_row(
                    record,
                    related_instance_id
                        .as_deref()
                        .and_then(|id| skill_by_id.get(id)),
                    &evidence_id,
                ));
            }
        }

        if filters.include_prompt_runs {
            if !self.llm_prompt_runs_path().exists() {
                gap_notes.push("No app-local prompt run metadata records are saved.".to_string());
            }
            for run in self.load_llm_prompt_runs()?.iter().filter(|run| {
                skill_lifecycle_prompt_run_matches(&filters, run, &skill_by_id, &visible_ids)
            }) {
                let related_instance_id = skill_lifecycle_related_instance_for_strings(
                    run.instance_ids
                        .iter()
                        .chain(run.instance_id.iter())
                        .chain(run.definition_id.iter()),
                    &skill_by_id,
                );
                let evidence_id = push_task_readiness_evidence(
                    &mut evidence_references,
                    "prompt_run",
                    &run.id,
                    format!(
                        "App-local prompt run metadata `{}` action `{}` status `{}`",
                        redact_for_llm_preview(&run.id),
                        redact_for_llm_preview(&run.action),
                        redact_for_llm_preview(&run.status)
                    ),
                    Some(run.status.clone()),
                    related_instance_id,
                );
                rows.push(skill_lifecycle_prompt_run_row(
                    run,
                    &skill_by_id,
                    &evidence_id,
                ));
            }
        }

        if filters.include_session_reviews {
            if !self.agent_session_reviews_path().exists() {
                gap_notes.push("No app-local agent session skill reviews are saved.".to_string());
            }
            for review in self.load_agent_session_reviews()?.iter().filter(|review| {
                skill_lifecycle_session_review_matches(&filters, review, &skill_by_id, &visible_ids)
            }) {
                let related_instance_id = review
                    .analysis
                    .detected_skills
                    .iter()
                    .find(|skill| skill_by_id.contains_key(&skill.instance_id))
                    .map(|skill| skill.instance_id.clone())
                    .or_else(|| {
                        skill_lifecycle_related_instance_for_strings(
                            review
                                .expected_skill_refs
                                .iter()
                                .chain(review.analysis.evidence_refs.iter()),
                            &skill_by_id,
                        )
                    });
                let evidence_id = push_task_readiness_evidence(
                    &mut evidence_references,
                    "session_review",
                    &review.id,
                    format!(
                        "App-local session review `{}` outcome `{}`",
                        redact_for_llm_preview(&review.title),
                        redact_for_llm_preview(&review.analysis.outcome)
                    ),
                    Some(review.analysis.outcome.clone()),
                    related_instance_id,
                );
                rows.push(skill_lifecycle_session_review_row(
                    review,
                    &skill_by_id,
                    &evidence_id,
                ));
            }
        }

        rows.sort_by(skill_lifecycle_row_sort);
        rows.truncate(filters.limit);
        if rows.is_empty() {
            gap_notes.push(
                "No deterministic local lifecycle events matched the current filters.".to_string(),
            );
        }
        blocker_notes.push(
            "Skill lifecycle timeline is read-only; it does not write skills, mutate agent config, create snapshots, execute scripts, send provider requests, or change triage."
                .to_string(),
        );
        normalize_note_list(&mut gap_notes);
        normalize_note_list(&mut blocker_notes);
        dedupe_evidence_references(&mut evidence_references);

        let skill_rows = skill_lifecycle_skill_rows(&rows, &skill_by_id);
        let agent_rows = skill_lifecycle_agent_rows(&rows, &skill_by_id);
        let summary = skill_lifecycle_summary(&rows, &skill_rows, &agent_rows, &filters);
        let prompt_instance_ids = skill_rows
            .iter()
            .take(12)
            .map(|row| row.instance_id.clone())
            .collect::<Vec<_>>();

        Ok(SkillLifecycleTimelineResult {
            generated_by: "local-v2.66",
            catalog_available: true,
            filters,
            summary,
            timeline_rows: rows,
            skill_rows,
            agent_rows,
            gap_notes,
            blocker_notes,
            evidence_references,
            prompt_request: AgentReadinessPromptRequest {
                available: !prompt_instance_ids.is_empty(),
                preview_method: "llm.previewPrompt",
                confirm_method: "llm.confirmPromptAndSend",
                action: "skill_lifecycle_timeline",
                request: LlmPreviewPromptParams {
                    action: LlmPromptActionKind::SkillLifecycleTimeline,
                    profile_id: None,
                    app_language: None,
                    skill_instance_id: None,
                    instance_ids: prompt_instance_ids,
                    agents: Vec::new(),
                analysis_kind: None,
                    user_intent: Some(
                        "Explain deterministic skill lifecycle timeline rows using only local redacted evidence."
                            .to_string(),
                    ),
                },
                note: "Optional provider-backed lifecycle wording must be requested through prompt preview and explicit confirmation; skill.lifecycleTimeline never sends provider traffic and remains copy-only."
                    .to_string(),
            },
            safety_flags: skill_lifecycle_timeline_safety_flags(),
        })
    }

    pub fn list_task_benchmarks(
        &self,
        params: ListTaskBenchmarksParams,
    ) -> Result<TaskBenchmarkListResult, ServiceError> {
        let mut benchmarks = self.load_task_benchmarks()?;
        if let Some(limit) = params.limit {
            benchmarks.truncate(limit);
        }
        Ok(TaskBenchmarkListResult {
            count: benchmarks.len(),
            benchmarks,
            app_local_only: true,
            provider_request_sent: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
    }

    pub fn save_task_benchmark(
        &self,
        params: SaveTaskBenchmarkParams,
    ) -> Result<SaveTaskBenchmarkResult, ServiceError> {
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.saveBenchmark requires a non-empty task".to_string(),
            ));
        }
        let title = params
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| task.chars().take(72).collect::<String>());
        let id = params
            .id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(sanitize_benchmark_id)
            .unwrap_or_else(|| generated_benchmark_id(&title, task));
        if id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.saveBenchmark requires a benchmark id containing letters, numbers, '-' or '_'"
                    .to_string(),
            ));
        }

        let mut benchmarks = self.load_task_benchmarks()?;
        let now = unix_timestamp_millis();
        let existing_index = benchmarks.iter().position(|benchmark| benchmark.id == id);
        let created_at = existing_index
            .and_then(|index| benchmarks.get(index).map(|benchmark| benchmark.created_at))
            .unwrap_or(now);
        let benchmark = TaskBenchmarkRecord {
            id: id.clone(),
            title,
            task: task.to_string(),
            expected_skill_refs: normalize_string_list(params.expected_skill_refs),
            expected_skill_names: normalize_string_list(params.expected_skill_names),
            acceptable_agents: normalize_string_list(params.acceptable_agents),
            acceptable_scopes: normalize_string_list(params.acceptable_scopes),
            success_criteria: normalize_string_list(params.success_criteria),
            created_at,
            updated_at: now,
        };
        let created = if let Some(index) = existing_index {
            benchmarks[index] = benchmark.clone();
            false
        } else {
            benchmarks.push(benchmark.clone());
            true
        };
        self.save_task_benchmarks(&benchmarks)?;
        Ok(SaveTaskBenchmarkResult {
            benchmark,
            created,
            app_local_only: true,
            provider_request_sent: false,
            agent_config_mutated: false,
        })
    }

    pub fn delete_task_benchmark(
        &self,
        params: DeleteTaskBenchmarkParams,
    ) -> Result<DeleteTaskBenchmarkResult, ServiceError> {
        let id = sanitize_benchmark_id(params.id.trim());
        if id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "task.deleteBenchmark requires a benchmark id".to_string(),
            ));
        }
        let mut benchmarks = self.load_task_benchmarks()?;
        let before = benchmarks.len();
        benchmarks.retain(|benchmark| benchmark.id != id);
        let deleted = benchmarks.len() != before;
        if deleted {
            self.save_task_benchmarks(&benchmarks)?;
        }
        Ok(DeleteTaskBenchmarkResult {
            benchmark_id: id,
            deleted,
            remaining_count: benchmarks.len(),
            app_local_only: true,
            provider_request_sent: false,
            agent_config_mutated: false,
        })
    }

    pub fn evaluate_task_benchmarks(
        &self,
        params: EvaluateTaskBenchmarksParams,
    ) -> Result<TaskBenchmarkEvaluationResult, ServiceError> {
        let requested_ids = params
            .ids
            .iter()
            .map(|id| sanitize_benchmark_id(id.trim()))
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        let mut benchmarks = self.load_task_benchmarks()?;
        if !requested_ids.is_empty() {
            benchmarks.retain(|benchmark| requested_ids.contains(&benchmark.id));
        }
        if let Some(limit) = params.limit {
            benchmarks.truncate(limit);
        }

        let mut benchmark_results = Vec::new();
        let mut catalog_available = self.catalog_path().exists();
        for benchmark in &benchmarks {
            let ranking = self.rank_skill_routes(RankSkillRoutesParams {
                task: benchmark.task.clone(),
                agent: None,
                candidate_instance_ids: Vec::new(),
                limit: Some(8),
            })?;
            catalog_available &= ranking.catalog_available;
            benchmark_results.push(task_benchmark_evaluation_item(benchmark, ranking));
        }
        if benchmarks.is_empty() {
            catalog_available = self.catalog_path().exists();
        }

        let blocker_notes = task_benchmark_blocker_notes(&benchmark_results, catalog_available);
        let prompt_request = task_benchmark_prompt_request(&benchmark_results);
        Ok(TaskBenchmarkEvaluationResult {
            generated_by: "deterministic-service",
            catalog_available,
            evaluated_count: benchmark_results.len(),
            summary: task_benchmark_summary(&benchmark_results, catalog_available),
            benchmark_results,
            blocker_notes,
            prompt_request,
            safety_flags: task_benchmark_safety_flags(),
        })
    }

    pub fn save_routing_baseline(
        &self,
        params: SaveRoutingBaselineParams,
    ) -> Result<SaveRoutingBaselineResult, ServiceError> {
        let evaluation = self.evaluate_task_benchmarks(EvaluateTaskBenchmarksParams {
            ids: params.ids,
            limit: params.limit,
        })?;
        let baseline = routing_regression_baseline_from_evaluation(evaluation);
        self.save_routing_regression_baseline(&baseline)?;
        Ok(SaveRoutingBaselineResult {
            benchmark_count: baseline.evaluated_count,
            baseline,
            generated_by: "deterministic-service",
            app_local_only: true,
            baseline_file: "task-routing-baseline.json",
            provider_request_sent: false,
            agent_config_mutated: false,
            skill_files_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
    }

    pub fn detect_routing_regression(
        &self,
        params: DetectRoutingRegressionParams,
    ) -> Result<RoutingRegressionDetectionResult, ServiceError> {
        let current_evaluation = self.evaluate_task_benchmarks(EvaluateTaskBenchmarksParams {
            ids: params.ids,
            limit: params.limit,
        })?;
        let baseline = self.load_routing_regression_baseline()?;
        let Some(baseline) = baseline else {
            let blocker_notes = vec![
                "No app-local routing baseline is saved; run task.saveRoutingBaseline before regression detection."
                    .to_string(),
            ];
            return Ok(RoutingRegressionDetectionResult {
                generated_by: "deterministic-service",
                status: "baseline_missing",
                baseline_available: false,
                catalog_available: current_evaluation.catalog_available,
                baseline_evaluated_count: 0,
                current_evaluated_count: current_evaluation.evaluated_count,
                regression_count: 0,
                missing_benchmark_count: 0,
                summary: format!(
                    "No app-local routing baseline was available; evaluated {} current benchmark(s) without writing a baseline.",
                    current_evaluation.evaluated_count
                ),
                items: Vec::new(),
                blocker_notes,
                baseline: None,
                current_evaluation,
                safety_flags: task_benchmark_safety_flags(),
            });
        };

        let comparison = routing_regression_compare(
            &baseline,
            &current_evaluation,
            params.score_drop_threshold.unwrap_or(10),
            params.confidence_drop_threshold.unwrap_or(10),
        );
        let regression_count = comparison.iter().filter(|item| item.regression).count();
        let missing_benchmark_count = comparison
            .iter()
            .filter(|item| item.status == "missing_current_benchmark")
            .count();
        let mut blocker_notes = current_evaluation.blocker_notes.clone();
        if !current_evaluation.catalog_available {
            blocker_notes.push(
                "No local catalog is available; routing regression detection cannot verify current routes."
                    .to_string(),
            );
        }
        if baseline.benchmark_results.is_empty() {
            blocker_notes.push("Saved routing baseline contains no benchmark results.".to_string());
        }
        blocker_notes.sort();
        blocker_notes.dedup();
        let status = routing_regression_status(
            regression_count,
            missing_benchmark_count,
            current_evaluation.catalog_available,
        );
        Ok(RoutingRegressionDetectionResult {
            generated_by: "deterministic-service",
            status,
            baseline_available: true,
            catalog_available: current_evaluation.catalog_available,
            baseline_evaluated_count: baseline.evaluated_count,
            current_evaluated_count: current_evaluation.evaluated_count,
            regression_count,
            missing_benchmark_count,
            summary: routing_regression_summary(
                regression_count,
                missing_benchmark_count,
                comparison.len(),
                current_evaluation.catalog_available,
            ),
            items: comparison,
            blocker_notes,
            baseline: Some(baseline),
            current_evaluation,
            safety_flags: task_benchmark_safety_flags(),
        })
    }

    pub fn routing_accuracy_dashboard(
        &self,
        params: RoutingAccuracyDashboardParams,
    ) -> Result<RoutingAccuracyDashboardResult, ServiceError> {
        let now = unix_timestamp_millis();
        let window_days = params.window_days.unwrap_or(30).clamp(1, 365);
        let limit = params.limit.unwrap_or(25).clamp(1, 250);
        let window_start_millis = now.saturating_sub(i64::from(window_days) * 86_400_000);
        let agent_filter = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|agent| !agent.is_empty())
            .map(str::to_string);

        let imports_file_available = self.trace_imports_path().exists();
        let benchmark_file_available = self.task_benchmarks_path().exists();
        let baseline_file_available = self.routing_regression_baseline_path().exists();
        let mut imports = self.load_trace_imports()?;
        imports.retain(|import| {
            import.imported_at >= window_start_millis
                && import.imported_at <= now
                && routing_accuracy_agent_matches_import(&agent_filter, import)
        });

        let detection = self.detect_routing_regression(DetectRoutingRegressionParams {
            ids: Vec::new(),
            limit: None,
            score_drop_threshold: None,
            confidence_drop_threshold: None,
        })?;

        let mut summary = RoutingAccuracyDashboardSummary::default();
        let mut agent_rows: BTreeMap<String, RoutingAccuracyAgentAggregate> = BTreeMap::new();
        let mut history_rows: BTreeMap<i64, RoutingAccuracyOutcomeCounts> = BTreeMap::new();
        let mut gap_issue_rows = Vec::new();
        let mut recent_evidence_rows = Vec::new();

        for import in &imports {
            let outcome = routing_accuracy_normalize_outcome(&import.analysis.outcome);
            routing_accuracy_increment_summary(&mut summary, outcome);
            let agent = routing_accuracy_trace_agent(import);
            agent_rows
                .entry(agent.clone())
                .or_default()
                .record_trace(outcome);
            if params.include_history {
                let unix_day = import.imported_at.div_euclid(86_400_000);
                routing_accuracy_increment_counts(
                    history_rows.entry(unix_day).or_default(),
                    outcome,
                );
            }
            if params.include_recent_evidence {
                recent_evidence_rows.push(RoutingAccuracyEvidenceRow {
                    source: "trace.importLocal",
                    agent: Some(agent),
                    title: import.title.clone(),
                    outcome: Some(outcome.to_string()),
                    detail: routing_accuracy_trace_detail(import),
                    evidence_refs: import.analysis.evidence_refs.clone(),
                    observed_at: Some(import.imported_at),
                });
            }
        }

        let benchmark_results = &detection.current_evaluation.benchmark_results;
        for item in benchmark_results
            .iter()
            .filter(|item| routing_accuracy_agent_matches_benchmark(&agent_filter, item))
        {
            summary.benchmark_count += 1;
            if matches!(
                item.expected_match_status,
                "expected_match" | "acceptable_match"
            ) {
                summary.benchmark_matched_count += 1;
            } else {
                summary.benchmark_gap_count += 1;
            }
            let agent = routing_accuracy_benchmark_agent(item);
            let agent_row = agent_rows.entry(agent.clone()).or_default();
            agent_row.benchmark_count += 1;
            if matches!(
                item.expected_match_status,
                "expected_match" | "acceptable_match"
            ) {
                agent_row.benchmark_matched_count += 1;
            } else {
                agent_row.benchmark_gap_count += 1;
            }
            if item.expected_match_status != "expected_match"
                || !item.gap_notes.is_empty()
                || !item.blocker_notes.is_empty()
            {
                gap_issue_rows.push(RoutingAccuracyIssueRow {
                    source: "task.evaluateBenchmarks",
                    severity: routing_accuracy_benchmark_severity(item),
                    agent: Some(agent.clone()),
                    title: item.title.clone(),
                    detail: routing_accuracy_benchmark_issue_detail(item),
                    evidence_refs: item.evidence_refs.clone(),
                });
            }
            if params.include_recent_evidence {
                recent_evidence_rows.push(RoutingAccuracyEvidenceRow {
                    source: "task.evaluateBenchmarks",
                    agent: Some(agent),
                    title: item.title.clone(),
                    outcome: Some(item.expected_match_status.to_string()),
                    detail: format!(
                        "Benchmark score {}/100 with route confidence {}/100.",
                        item.score, item.route_confidence_score
                    ),
                    evidence_refs: item.evidence_refs.clone(),
                    observed_at: None,
                });
            }
        }

        for item in detection
            .items
            .iter()
            .filter(|item| routing_accuracy_agent_matches_regression(&agent_filter, item))
        {
            if item.regression {
                summary.regression_count += 1;
                let agent = routing_accuracy_regression_agent(item);
                let agent_key = agent.clone().unwrap_or_else(|| "unknown".to_string());
                agent_rows.entry(agent_key).or_default().regression_count += 1;
                gap_issue_rows.push(RoutingAccuracyIssueRow {
                    source: "task.detectRoutingRegression",
                    severity: "critical",
                    agent,
                    title: item.title.clone(),
                    detail: item.reasons.join(" "),
                    evidence_refs: item.evidence_refs.clone(),
                });
            }
            if item.status == "missing_current_benchmark" {
                summary.missing_benchmark_count += 1;
            }
            if params.include_recent_evidence {
                recent_evidence_rows.push(RoutingAccuracyEvidenceRow {
                    source: "task.detectRoutingRegression",
                    agent: routing_accuracy_regression_agent(item),
                    title: item.title.clone(),
                    outcome: Some(item.status.to_string()),
                    detail: routing_accuracy_regression_detail(item),
                    evidence_refs: item.evidence_refs.clone(),
                    observed_at: None,
                });
            }
        }

        summary.trace_count = imports.len();
        summary.accuracy_rate = routing_accuracy_rate(
            summary.hit_count,
            summary.hit_count
                + summary.miss_count
                + summary.wrong_pick_count
                + summary.ambiguous_count,
        );
        summary.known_outcome_rate = routing_accuracy_rate(
            summary.hit_count
                + summary.miss_count
                + summary.wrong_pick_count
                + summary.ambiguous_count,
            summary.trace_count,
        );
        summary.summary = routing_accuracy_summary_text(&summary, detection.catalog_available);

        let mut blocker_notes = detection.blocker_notes.clone();
        if !detection.catalog_available {
            blocker_notes.push(
                "No local catalog is available; dashboard metrics are limited to app-local trace metadata and saved benchmark records."
                    .to_string(),
            );
        }
        if !imports_file_available {
            blocker_notes.push("No app-local trace imports are saved.".to_string());
        }
        if !benchmark_file_available {
            blocker_notes.push("No app-local task benchmarks are saved.".to_string());
        }
        if !baseline_file_available {
            blocker_notes.push(
                "No app-local routing regression baseline is saved; regression evidence is unavailable."
                    .to_string(),
            );
        }
        if imports.is_empty() && benchmark_results.is_empty() {
            blocker_notes
                .push("No routing accuracy evidence matched the current filters.".to_string());
        }
        blocker_notes.sort();
        blocker_notes.dedup();

        gap_issue_rows.sort_by(|left, right| {
            routing_accuracy_severity_rank(left.severity)
                .cmp(&routing_accuracy_severity_rank(right.severity))
                .then_with(|| left.source.cmp(right.source))
                .then_with(|| left.title.cmp(&right.title))
        });
        gap_issue_rows.truncate(limit);
        recent_evidence_rows.sort_by(|left, right| {
            right
                .observed_at
                .cmp(&left.observed_at)
                .then_with(|| left.source.cmp(right.source))
                .then_with(|| left.title.cmp(&right.title))
        });
        recent_evidence_rows.truncate(limit);

        let agent_rows = agent_rows
            .into_iter()
            .map(|(agent, aggregate)| aggregate.into_row(agent))
            .collect::<Vec<_>>();
        let history_rows = if params.include_history {
            history_rows
                .into_iter()
                .map(|(unix_day, outcomes)| {
                    let known =
                        outcomes.hit + outcomes.miss + outcomes.wrong_pick + outcomes.ambiguous;
                    RoutingAccuracyHistoryRow {
                        unix_day,
                        trace_count: known + outcomes.unknown,
                        accuracy_rate: routing_accuracy_rate(outcomes.hit, known),
                        outcomes,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(RoutingAccuracyDashboardResult {
            generated_by: "deterministic-service",
            catalog_available: detection.catalog_available,
            filters: RoutingAccuracyDashboardFilters {
                agent: agent_filter,
                window_days,
                limit,
                include_history: params.include_history,
                include_recent_evidence: params.include_recent_evidence,
                window_start_millis,
                window_end_millis: now,
            },
            summary,
            agent_rows,
            history_rows,
            gap_issue_rows,
            recent_evidence_rows,
            blocker_notes,
            prompt_request: routing_accuracy_prompt_request(&imports, benchmark_results),
            safety_flags: routing_accuracy_safety_flags(),
        })
    }

    pub fn review_agent_skill_use(
        &self,
        params: AgentSessionSkillReviewParams,
    ) -> Result<AgentSessionSkillReviewResult, ServiceError> {
        let content = params.content.trim().to_string();
        let requested_trace_import_ids = normalize_string_list(
            params
                .trace_import_ids
                .into_iter()
                .map(|id| sanitize_trace_import_id(id.trim()))
                .filter(|id| !id.is_empty())
                .collect(),
        );
        if content.is_empty() && requested_trace_import_ids.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "session.reviewAgentSkillUse requires transcript content or trace_import_ids"
                    .to_string(),
            ));
        }

        let adapter_ctx = self.effective_adapter_ctx()?;
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&redaction_roots);
        let imports = self.load_trace_imports()?;
        let mut referenced_imports = Vec::new();
        let mut missing_trace_import_ids = Vec::new();
        for trace_id in &requested_trace_import_ids {
            if let Some(import) = imports.iter().find(|import| import.id == *trace_id) {
                referenced_imports.push(import.clone());
            } else {
                missing_trace_import_ids.push(trace_id.clone());
            }
        }

        let mut expected_refs = params.expected_skill_refs;
        let mut expected_names = params.expected_skill_names;
        for import in &referenced_imports {
            expected_refs.extend(import.expected_skill_refs.clone());
            expected_names.extend(import.expected_skill_names.clone());
        }
        let expected_skill_refs = redact_normalized_string_list(expected_refs, &redaction_roots);
        let expected_skill_names = redact_normalized_string_list(expected_names, &redaction_roots);

        let task = params
            .task
            .as_deref()
            .map(str::trim)
            .filter(|task| !task.is_empty())
            .map(|task| truncate_chars(&redactor.redact(task), 320))
            .or_else(|| {
                referenced_imports
                    .iter()
                    .find_map(|import| import.task.clone())
            });
        let title = params
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(|title| truncate_chars(&redactor.redact(title), 180))
            .or_else(|| task.clone())
            .or_else(|| {
                referenced_imports
                    .first()
                    .map(|import| format!("Session review: {}", import.title))
            })
            .unwrap_or_else(|| "Agent session skill review".to_string());
        let source_kind = params
            .source_kind
            .as_deref()
            .map(str::trim)
            .filter(|source_kind| !source_kind.is_empty())
            .map(|source_kind| truncate_chars(&redactor.redact(source_kind), 100))
            .unwrap_or_else(|| {
                if content.is_empty() {
                    "trace-import-reference".to_string()
                } else {
                    "agent-session-transcript".to_string()
                }
            });
        let agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|agent| !agent.is_empty())
            .map(|agent| truncate_chars(&redactor.redact(agent), 80))
            .or_else(|| single_referenced_trace_agent(&referenced_imports));

        let max_excerpt_chars = params.max_excerpt_chars.unwrap_or(1_200).clamp(120, 6_000);
        let mut excerpt_parts = Vec::new();
        if !content.is_empty() {
            excerpt_parts.push(redactor.redact(&content));
        }
        excerpt_parts.extend(
            referenced_imports
                .iter()
                .map(|import| import.excerpt.clone()),
        );
        if excerpt_parts.is_empty() {
            excerpt_parts.push("No matching trace import excerpt was available.".to_string());
        }
        let excerpt = truncate_chars(&excerpt_parts.join("\n\n"), max_excerpt_chars);
        let excerpt_char_count = excerpt.chars().count();

        let mut analysis_parts = Vec::new();
        if !content.is_empty() {
            analysis_parts.push(content.clone());
        }
        analysis_parts.extend(
            referenced_imports
                .iter()
                .map(|import| import.excerpt.clone()),
        );
        let analysis_content = analysis_parts.join("\n\n");
        let analysis = self.analyze_agent_session_skill_use(
            &analysis_content,
            &expected_skill_refs,
            &expected_skill_names,
            agent.as_deref(),
            &referenced_imports,
            &missing_trace_import_ids,
        )?;
        let content_hash = trace_content_hash(&format!(
            "{}\0{}",
            content,
            requested_trace_import_ids.join("\0")
        ));
        let reviewed_at = unix_timestamp_millis();
        let redaction_summary = agent_session_review_redaction_summary_from(redactor.summary());
        let record = AgentSessionSkillReviewRecord {
            id: generated_agent_session_review_id(&title, &content_hash, reviewed_at),
            title,
            source_kind,
            agent,
            task,
            trace_import_ids: referenced_imports
                .iter()
                .map(|import| import.id.clone())
                .collect(),
            missing_trace_import_ids,
            expected_skill_refs,
            expected_skill_names,
            excerpt,
            excerpt_char_count,
            content_hash,
            redaction_summary,
            reviewed_at,
            analysis,
            safety_flags: agent_session_review_safety_flags(),
        };

        let mut reviews = self.load_agent_session_reviews()?;
        reviews.push(record.clone());
        self.save_agent_session_reviews(&reviews)?;
        Ok(AgentSessionSkillReviewResult {
            generated_by: "local-v2.62",
            review: record,
            count: reviews.len(),
            app_local_only: true,
            review_file: "agent-session-reviews.json",
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    pub fn preview_local_sessions(
        &self,
        params: LocalSessionPreviewParams,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let limit = params.limit.unwrap_or(20).clamp(1, 100);
        let max_files = params.max_files.unwrap_or(200).clamp(1, 1_000);
        let max_excerpt_chars = params.max_excerpt_chars.unwrap_or(1_000).clamp(120, 4_000);
        let requested_roots = normalize_string_list(params.authorized_roots);
        let auto_discover = params.auto_discover.unwrap_or(requested_roots.is_empty());
        let adapter_ctx = self.effective_adapter_ctx()?;
        let scope = LocalSessionScope::from_param(params.scope.as_deref());
        let project_filter_roots = local_session_project_filter_roots(
            &adapter_ctx,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
        );
        let search = params
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&redaction_roots);
        let requested_agent = params.agent.as_deref();

        let mut root_requests = requested_roots
            .iter()
            .map(|root| LocalSessionRootRequest {
                path: PathBuf::from(root),
                status: "authorized-read-only",
                source_kind: "authorized-local-session",
            })
            .collect::<Vec<_>>();
        let mut gap_notes = Vec::new();
        let mut blocker_notes = Vec::new();
        if auto_discover {
            let (mut discovered_roots, discovery_notes) = auto_local_session_roots(
                &adapter_ctx,
                requested_agent,
                scope,
                &project_filter_roots,
            );
            root_requests.append(&mut discovered_roots);
            gap_notes.extend(discovery_notes);
        }
        dedupe_local_session_root_requests(&mut root_requests);

        if root_requests.is_empty() {
            if gap_notes.is_empty() {
                gap_notes.push(
                    "No supported local agent session store was found for the selected agent."
                        .to_string(),
                );
            }
            return Ok(LocalSessionPreviewResult {
                generated_by: "local-v2.98",
                authorized: false,
                authorization_required: false,
                roots: Vec::new(),
                count: 0,
                total_candidate_count: 0,
                user_message_count: 0,
                total_message_count: 0,
                tool_call_count: 0,
                skill_call_count: 0,
                skill_usage_rows: Vec::new(),
                session_rows: Vec::new(),
                gap_notes,
                blocker_notes,
                redaction_summary: agent_session_review_redaction_summary_from(redactor.summary()),
                safety_flags: agent_session_review_safety_flags(),
                read_only: true,
                provider_request_sent: false,
                skill_files_mutated: false,
                agent_config_mutated: false,
                snapshot_created: false,
                triage_mutated: false,
                raw_prompt_persisted: false,
                raw_response_persisted: false,
                raw_trace_persisted: false,
            });
        }

        let mut root_rows = Vec::new();
        let mut session_rows = Vec::new();
        let mut seen_session_row_ids = BTreeSet::new();
        let mut skill_usage = BTreeMap::<String, LocalSessionSkillUsageAccumulator>::new();
        let skill_matchers = self.local_session_skill_matchers(requested_agent)?;
        let mut total_candidate_count = 0usize;

        for root_request in root_requests {
            let root = root_request.path.to_string_lossy().to_string();
            let redacted_root = redactor.redact(&root);
            let root_path = root_request.path;
            if !root_path.is_absolute() {
                let blocker = "Authorized session roots must be absolute paths.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }
            if !root_path.exists() {
                let blocker = "Authorized session root does not exist.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }
            if !root_path.is_dir() {
                let blocker = "Authorized session root is not a directory.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }

            let canonical_root = match root_path.canonicalize() {
                Ok(path) => path,
                Err(error) => {
                    let blocker = format!("Authorized session root could not be resolved: {error}");
                    blocker_notes.push(format!("{redacted_root}: {}", redactor.redact(&blocker)));
                    root_rows.push(LocalSessionPreviewRoot {
                        root: redacted_root,
                        status: "blocked".to_string(),
                        candidate_count: 0,
                        blocker: Some(redactor.redact(&blocker)),
                    });
                    continue;
                }
            };

            let files = collect_local_session_files(
                &canonical_root,
                max_files,
                &mut gap_notes,
                &mut redactor,
            );
            total_candidate_count += files.len();
            let mut root_candidate_count = 0usize;
            for file in files {
                let options = LocalSessionPreviewRowOptions {
                    requested_agent,
                    max_excerpt_chars,
                    source_kind: root_request.source_kind,
                    skill_matchers: &skill_matchers,
                    scope,
                    project_filter_roots: &project_filter_roots,
                    search: search.as_deref(),
                };
                match local_session_preview_row(&file, &canonical_root, options, &mut redactor) {
                    Ok(Some(entry)) => {
                        if seen_session_row_ids.insert(entry.row.id.clone()) {
                            root_candidate_count += 1;
                            update_local_session_skill_usage(&mut skill_usage, &entry);
                            session_rows.push(entry.row);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        gap_notes.push(format!(
                            "{}: {}",
                            redactor.redact(&file.to_string_lossy()),
                            redactor.redact(&error.to_string())
                        ));
                    }
                }
            }

            root_rows.push(LocalSessionPreviewRoot {
                root: redacted_root,
                status: root_request.status.to_string(),
                candidate_count: root_candidate_count,
                blocker: None,
            });
        }

        session_rows.sort_by(|left, right| {
            right
                .modified_at
                .cmp(&left.modified_at)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.id.cmp(&right.id))
        });
        session_rows.truncate(limit);
        let count = session_rows.len();
        let user_message_count = session_rows
            .iter()
            .map(|row| row.user_message_count)
            .sum::<usize>();
        let total_message_count = session_rows
            .iter()
            .map(|row| row.total_message_count)
            .sum::<usize>();
        let tool_call_count = session_rows
            .iter()
            .map(|row| row.tool_call_count)
            .sum::<usize>();
        let skill_call_count = session_rows
            .iter()
            .map(|row| row.skill_call_count)
            .sum::<usize>();
        if count == 0 && blocker_notes.is_empty() {
            gap_notes.push(
                "Discovered local session stores did not contain supported session files (.jsonl, .json, .txt, .log)."
                    .to_string(),
            );
        }
        let skill_usage_rows = local_session_skill_usage_rows(skill_usage, limit);

        Ok(LocalSessionPreviewResult {
            generated_by: "local-v2.98",
            authorized: root_rows.iter().any(|root| {
                root.status == "authorized-read-only" || root.status == "auto-discovered-read-only"
            }),
            authorization_required: false,
            roots: root_rows,
            count,
            total_candidate_count,
            user_message_count,
            total_message_count,
            tool_call_count,
            skill_call_count,
            skill_usage_rows,
            session_rows,
            gap_notes,
            blocker_notes,
            redaction_summary: agent_session_review_redaction_summary_from(redactor.summary()),
            safety_flags: agent_session_review_safety_flags(),
            read_only: true,
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    fn local_session_skill_matchers(
        &self,
        requested_agent: Option<&str>,
    ) -> Result<Vec<LocalSessionSkillMatcher>, ServiceError> {
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(Vec::new());
        };
        let skills = self.list_visible_skill_records(&catalog)?;
        Ok(skills
            .into_iter()
            .filter(|skill| {
                requested_agent.is_none_or(|agent| {
                    agent.eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                        || skill.agent.eq_ignore_ascii_case(agent)
                        || skill
                            .agent
                            .eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                })
            })
            .map(LocalSessionSkillMatcher::from)
            .collect())
    }

    pub fn list_agent_skill_reviews(
        &self,
        params: AgentSessionListSkillReviewsParams,
    ) -> Result<AgentSessionSkillReviewListResult, ServiceError> {
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let outcome = params
            .outcome
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let trace_import_id = params
            .trace_import_id
            .as_deref()
            .map(str::trim)
            .map(sanitize_trace_import_id)
            .filter(|value| !value.is_empty());
        let mut reviews = self.load_agent_session_reviews()?;
        reviews.retain(|review| {
            let agent_matches = agent
                .as_deref()
                .is_none_or(|filter| review.agent.as_deref() == Some(filter));
            let outcome_matches = outcome
                .as_deref()
                .is_none_or(|filter| review.analysis.outcome.eq_ignore_ascii_case(filter));
            let trace_matches = trace_import_id.as_deref().is_none_or(|filter| {
                review
                    .trace_import_ids
                    .iter()
                    .any(|trace_id| trace_id == filter)
            });
            agent_matches && outcome_matches && trace_matches
        });
        let total_count = reviews.len();
        reviews.truncate(limit);
        Ok(AgentSessionSkillReviewListResult {
            generated_by: "local-v2.62",
            count: reviews.len(),
            total_count,
            reviews,
            app_local_only: true,
            review_file: "agent-session-reviews.json",
            provider_request_sent: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            safety_flags: agent_session_review_safety_flags(),
        })
    }

    pub fn delete_agent_skill_review(
        &self,
        params: AgentSessionDeleteSkillReviewParams,
    ) -> Result<AgentSessionSkillReviewDeleteResult, ServiceError> {
        let id = sanitize_agent_session_review_id(params.id.trim());
        if id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "session.deleteSkillReview requires a review id".to_string(),
            ));
        }
        let mut reviews = self.load_agent_session_reviews()?;
        let before = reviews.len();
        reviews.retain(|review| review.id != id);
        let deleted = reviews.len() != before;
        if deleted {
            self.save_agent_session_reviews(&reviews)?;
        }
        Ok(AgentSessionSkillReviewDeleteResult {
            review_id: id,
            deleted,
            remaining_count: reviews.len(),
            app_local_only: true,
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    pub fn import_local_trace(
        &self,
        params: TraceImportLocalParams,
    ) -> Result<TraceImportLocalResult, ServiceError> {
        let content = params.content.trim();
        if content.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "trace.importLocal requires non-empty trace content".to_string(),
            ));
        }

        let adapter_ctx = self.effective_adapter_ctx()?;
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&redaction_roots);
        let max_excerpt_chars = params.max_excerpt_chars.unwrap_or(800).clamp(80, 4_000);
        let excerpt = truncate_chars(&redactor.redact(content), max_excerpt_chars);
        let excerpt_char_count = excerpt.chars().count();
        let expected_skill_refs =
            redact_normalized_string_list(params.expected_skill_refs, &redaction_roots);
        let expected_skill_names =
            redact_normalized_string_list(params.expected_skill_names, &redaction_roots);
        let task = params
            .task
            .as_deref()
            .map(str::trim)
            .filter(|task| !task.is_empty())
            .map(|task| redactor.redact(task));
        let title = params
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(|title| redactor.redact(title))
            .or_else(|| task.clone())
            .unwrap_or_else(|| "Imported local trace".to_string());
        let source_kind = params
            .source_kind
            .as_deref()
            .map(str::trim)
            .filter(|source_kind| !source_kind.is_empty())
            .map(|source_kind| redactor.redact(source_kind))
            .unwrap_or_else(|| "local-transcript".to_string());
        let agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|agent| !agent.is_empty())
            .map(|agent| redactor.redact(agent));
        let redaction_summary = trace_import_redaction_summary_from(redactor.summary());
        let content_hash = trace_content_hash(content);
        let imported_at = unix_timestamp_millis();
        let analysis = self.analyze_imported_trace(
            content,
            &expected_skill_refs,
            &expected_skill_names,
            agent.as_deref(),
        )?;
        let record = TraceImportRecord {
            id: generated_trace_import_id(&title, &content_hash, imported_at),
            title,
            source_kind,
            agent,
            task,
            expected_skill_refs,
            expected_skill_names,
            excerpt,
            excerpt_char_count,
            redaction_summary,
            content_hash,
            imported_at,
            analysis,
            safety_flags: trace_import_safety_flags(),
        };

        let mut imports = self.load_trace_imports()?;
        imports.push(record.clone());
        self.save_trace_imports(&imports)?;
        Ok(TraceImportLocalResult {
            generated_by: "deterministic-service",
            import: record,
            count: imports.len(),
            app_local_only: true,
            import_file: "trace-imports.json",
            provider_request_sent: false,
            raw_trace_persisted: false,
        })
    }

    pub fn list_trace_imports(
        &self,
        params: TraceListImportsParams,
    ) -> Result<TraceImportListResult, ServiceError> {
        let mut imports = self.load_trace_imports()?;
        if let Some(limit) = params.limit {
            imports.truncate(limit);
        }
        Ok(TraceImportListResult {
            count: imports.len(),
            imports,
            app_local_only: true,
            provider_request_sent: false,
            raw_trace_persisted: false,
        })
    }

    pub fn delete_trace_import(
        &self,
        params: TraceDeleteImportParams,
    ) -> Result<TraceDeleteImportResult, ServiceError> {
        let id = sanitize_trace_import_id(params.id.trim());
        if id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "trace.deleteImport requires an import id".to_string(),
            ));
        }
        let mut imports = self.load_trace_imports()?;
        let before = imports.len();
        imports.retain(|record| record.id != id);
        let deleted = imports.len() != before;
        if deleted {
            self.save_trace_imports(&imports)?;
        }
        Ok(TraceDeleteImportResult {
            import_id: id,
            deleted,
            remaining_count: imports.len(),
            app_local_only: true,
            provider_request_sent: false,
            raw_trace_persisted: false,
        })
    }
}

#[derive(Debug, Clone)]
struct LocalSessionRootRequest {
    path: PathBuf,
    status: &'static str,
    source_kind: &'static str,
}

#[derive(Debug, Clone)]
struct LocalSessionPreviewEntry {
    row: LocalSessionPreviewRow,
    skill_mentions: Vec<LocalSessionSkillMention>,
}

#[derive(Debug, Clone, Copy)]
struct LocalSessionPreviewRowOptions<'a> {
    requested_agent: Option<&'a str>,
    max_excerpt_chars: usize,
    source_kind: &'static str,
    skill_matchers: &'a [LocalSessionSkillMatcher],
    scope: LocalSessionScope,
    project_filter_roots: &'a [PathBuf],
    search: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalSessionScope {
    Project,
    All,
}

impl LocalSessionScope {
    fn from_param(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase())
            .as_deref()
        {
            Some("project")
            | Some("current")
            | Some("current_project")
            | Some("current-folder")
            | Some("current_folder") => Self::Project,
            _ => Self::All,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone)]
struct LocalSessionContentDraft {
    kind: String,
    title: String,
    text: String,
    timestamp: Option<i64>,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalSessionTimeBounds {
    started_at: Option<i64>,
    ended_at: Option<i64>,
}

impl LocalSessionTimeBounds {
    fn push(&mut self, timestamp: Option<i64>) {
        let Some(timestamp) = timestamp else {
            return;
        };
        self.started_at = Some(
            self.started_at
                .map_or(timestamp, |current| current.min(timestamp)),
        );
        self.ended_at = Some(
            self.ended_at
                .map_or(timestamp, |current| current.max(timestamp)),
        );
    }
}

#[derive(Debug, Clone)]
struct LocalSessionSkillMatcher {
    skill_id: String,
    skill_name: String,
    agent: String,
    needles: Vec<String>,
}

impl From<SkillRecord> for LocalSessionSkillMatcher {
    fn from(skill: SkillRecord) -> Self {
        let mut needles = Vec::new();
        push_session_skill_needle(&mut needles, &skill.name);
        push_session_skill_needle(&mut needles, &skill.definition_id);
        push_session_skill_needle(&mut needles, &skill.id);
        Self {
            skill_id: skill.id,
            skill_name: skill.name,
            agent: skill.agent,
            needles,
        }
    }
}

#[derive(Debug, Clone)]
struct LocalSessionSkillMention {
    skill_id: String,
    skill_name: String,
    agent: String,
    count: usize,
    matched_invocations: Vec<String>,
    evidence_ref: String,
}

#[derive(Debug, Default)]
struct LocalSessionSkillUsageAccumulator {
    skill_id: String,
    skill_name: String,
    agent: String,
    call_count: usize,
    session_count: usize,
    latest_modified_at: Option<i64>,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct LocalSessionParsedMetadata {
    title: Option<String>,
    project_root: Option<String>,
    session_id: Option<String>,
}

fn auto_local_session_roots(
    adapter_ctx: &AdapterContext,
    requested_agent: Option<&str>,
    scope: LocalSessionScope,
    project_roots: &[PathBuf],
) -> (Vec<LocalSessionRootRequest>, Vec<String>) {
    let mut roots = Vec::new();
    let mut notes = Vec::new();
    let home = &adapter_ctx.user_home;

    if local_session_agent_matches(requested_agent, AgentId::ClaudeCode.as_str()) {
        let claude_projects = home.join(".claude/projects");
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                let encoded = encode_claude_project_session_dir(project);
                pushed_project_root |= push_existing_session_root(
                    &mut roots,
                    claude_projects.join(encoded),
                    "auto-discovered-read-only",
                    "auto-local-session",
                );
            }
        }
        push_existing_session_root(
            &mut roots,
            home.join(".claude/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
        if scope == LocalSessionScope::All || project_roots.is_empty() || !pushed_project_root {
            push_existing_session_root(
                &mut roots,
                claude_projects,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Codex.as_str()) {
        push_existing_session_root(
            &mut roots,
            home.join(".codex/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Opencode.as_str()) {
        push_existing_session_root(
            &mut roots,
            home.join(".local/share/opencode/storage"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Pi.as_str()) {
        let pi_sessions = home.join(".pi/agent/sessions");
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                for encoded in encode_pi_project_session_dirs(project) {
                    pushed_project_root |= push_existing_session_root(
                        &mut roots,
                        pi_sessions.join(encoded),
                        "auto-discovered-read-only",
                        "auto-local-session",
                    );
                }
            }
        }
        if scope == LocalSessionScope::All || project_roots.is_empty() || !pushed_project_root {
            push_existing_session_root(
                &mut roots,
                pi_sessions,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
        push_existing_session_root(
            &mut roots,
            home.join(".pi/context-mode/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Hermes.as_str()) {
        let state_db = home.join(".hermes/state.db");
        if state_db.exists() {
            notes.push(
                "Hermes session storage is SQLite-backed; automatic session parsing is deferred until the schema is confirmed."
                    .to_string(),
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Openclaw.as_str()) {
        let openclaw_root = home.join(".openclaw");
        if openclaw_root.exists() {
            notes.push(
                "OpenClaw session storage is not yet format-confirmed for automatic local parsing."
                    .to_string(),
            );
        }
    }

    if roots.is_empty() && notes.is_empty() {
        notes.push(
            "No supported local session store was detected for Claude Code, Codex, opencode, or Pi."
                .to_string(),
        );
    }

    (roots, notes)
}

fn local_session_project_filter_roots(
    adapter_ctx: &AdapterContext,
    requested_project_root: Option<&str>,
    requested_current_cwd: Option<&str>,
) -> Vec<PathBuf> {
    let explicit_candidates = [requested_project_root, requested_current_cwd]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !explicit_candidates.is_empty() {
        return normalized_local_session_project_roots(explicit_candidates);
    }

    let mut candidates = Vec::new();
    if let Some(value) = adapter_ctx.project_root.as_ref() {
        candidates.push(value.to_string_lossy().to_string());
    }
    if let Some(value) = adapter_ctx.project_cwd.as_ref() {
        candidates.push(value.to_string_lossy().to_string());
    }
    normalized_local_session_project_roots(candidates)
}

fn normalized_local_session_project_roots(candidates: Vec<String>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.is_absolute() {
            continue;
        }
        let normalized = local_session_normalized_path(&path);
        if !roots
            .iter()
            .any(|root| local_session_normalized_path(root) == normalized)
        {
            roots.push(path);
        }
    }
    roots
}

fn local_session_agent_matches(requested_agent: Option<&str>, agent: &str) -> bool {
    requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none_or(|requested| {
            requested.eq_ignore_ascii_case(agent)
                || requested.eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                || requested.eq_ignore_ascii_case("all")
        })
}

fn push_existing_session_root(
    roots: &mut Vec<LocalSessionRootRequest>,
    path: PathBuf,
    status: &'static str,
    source_kind: &'static str,
) -> bool {
    if path.is_dir() {
        roots.push(LocalSessionRootRequest {
            path,
            status,
            source_kind,
        });
        true
    } else {
        false
    }
}

fn dedupe_local_session_root_requests(roots: &mut Vec<LocalSessionRootRequest>) {
    let mut seen = BTreeSet::new();
    roots.retain(|root| {
        let key = root
            .path
            .canonicalize()
            .unwrap_or_else(|_| root.path.clone())
            .to_string_lossy()
            .to_string();
        seen.insert(key)
    });
}

fn encode_claude_project_session_dir(project: &Path) -> String {
    encode_project_path_session_component(project)
}

fn encode_pi_project_session_dirs(project: &Path) -> Vec<String> {
    let dash_path = encode_project_path_session_component(project);
    let trimmed = dash_path.trim_matches('-');
    let mut candidates = vec![
        dash_path.clone(),
        format!("{dash_path}-"),
        format!("-{trimmed}-"),
        format!("--{trimmed}--"),
    ];
    candidates.sort();
    candidates.dedup();
    candidates
}

fn encode_project_path_session_component(project: &Path) -> String {
    project
        .to_string_lossy()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' => '-',
            other => other,
        })
        .collect()
}

fn collect_local_session_files(
    root: &Path,
    max_files: usize,
    gap_notes: &mut Vec<String>,
    redactor: &mut PromptRedactor<'_>,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_path_buf()];

    while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                gap_notes.push(format!(
                    "{}: {}",
                    redactor.redact(&directory.to_string_lossy()),
                    redactor.redact(&error.to_string())
                ));
                continue;
            }
        };
        for entry in entries.flatten() {
            if files.len() >= max_files {
                gap_notes.push(format!(
                    "Local session preview stopped after {} candidate file(s) for bounded read latency.",
                    max_files
                ));
                return files;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file()
                && is_supported_local_session_file(&path)
                && !is_ignored_local_session_file(&path)
            {
                match path.canonicalize() {
                    Ok(canonical) if canonical.starts_with(root) => files.push(canonical),
                    Ok(canonical) => gap_notes.push(format!(
                        "{}: skipped because it resolves outside the authorized root.",
                        redactor.redact(&canonical.to_string_lossy())
                    )),
                    Err(error) => gap_notes.push(format!(
                        "{}: {}",
                        redactor.redact(&path.to_string_lossy()),
                        redactor.redact(&error.to_string())
                    )),
                }
            }
        }
    }

    files
}

fn is_supported_local_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jsonl" | "json" | "txt" | "log"
            )
        })
        .unwrap_or(false)
}

fn is_ignored_local_session_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".meta.json"))
    {
        return true;
    }
    path.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|name| {
            matches!(
                name,
                "memory" | "subagents" | "message" | "part" | "tool-results"
            )
        })
    })
}

fn local_session_preview_row(
    path: &Path,
    root: &Path,
    options: LocalSessionPreviewRowOptions<'_>,
    redactor: &mut PromptRedactor<'_>,
) -> Result<Option<LocalSessionPreviewEntry>, ServiceError> {
    if !path.starts_with(root) {
        return Ok(None);
    }
    let file_content = read_local_session_file_content(path)?;
    if file_content.is_empty() {
        return Ok(None);
    }
    let content = enrich_local_session_content(path, root, &file_content);
    let mut metadata = local_session_parsed_metadata(path, &file_content, &content);
    if let Some(project_root) =
        local_session_storage_project_root(path, root, options.project_filter_roots)
    {
        metadata.project_root = Some(project_root);
    }
    if metadata.title.is_none() {
        metadata.title = metadata
            .session_id
            .as_deref()
            .and_then(|session_id| codex_session_index_title(path, session_id));
    }
    if !local_session_matches_scope(
        options.scope,
        options.project_filter_roots,
        metadata.project_root.as_deref(),
    ) {
        return Ok(None);
    }
    let excerpt = truncate_chars(
        &redact_local_session_content(redactor, &content),
        options.max_excerpt_chars,
    );
    let excerpt_char_count = excerpt.chars().count();
    let content_hash = trace_content_hash(&content);
    let short_hash = content_hash.chars().take(12).collect::<String>();
    let row_id = local_session_row_id(path);
    let redacted_path = redactor.redact(&path.to_string_lossy());
    let title = metadata
        .title
        .as_deref()
        .map(|title| truncate_chars(&redactor.redact(title), 120))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| local_session_title(path, &short_hash));
    if !local_session_matches_search(
        options.search,
        &title,
        metadata.project_root.as_deref(),
        &redacted_path,
        &excerpt,
    ) {
        return Ok(None);
    }
    let skill_mentions = detect_local_session_skill_mentions(
        &content,
        options.skill_matchers,
        &format!("session.content_hash:{short_hash}"),
    );
    let content_items = local_session_content_items(
        &content,
        &short_hash,
        options.max_excerpt_chars,
        &skill_mentions,
        redactor,
    );
    let modified_at = local_session_modified_at(path);
    let (started_at, ended_at) = local_session_time_bounds(&content, &content_items, modified_at);
    let metrics = local_session_metrics(&content, &content_items);
    let agent = options
        .requested_agent
        .map(str::trim)
        .filter(|agent| !agent.is_empty())
        .map(|agent| truncate_chars(&redactor.redact(agent), 80))
        .or_else(|| infer_local_session_agent(path));
    let redacted_project_root = metadata
        .project_root
        .as_deref()
        .map(|project| truncate_chars(&redactor.redact(project), 180));
    Ok(Some(LocalSessionPreviewEntry {
        row: LocalSessionPreviewRow {
            id: row_id,
            title,
            source_kind: options.source_kind.to_string(),
            scope: options.scope.as_str().to_string(),
            agent,
            project_root: redacted_project_root,
            redacted_path: redacted_path.clone(),
            modified_at,
            started_at,
            ended_at,
            excerpt,
            excerpt_char_count,
            user_message_count: metrics.user_message_count,
            total_message_count: metrics.total_message_count,
            tool_call_count: metrics.tool_call_count,
            skill_call_count: metrics.skill_call_count,
            content_hash,
            evidence_refs: vec![
                format!("session.path:{redacted_path}"),
                format!("session.content_hash:{short_hash}"),
            ],
            content_items,
        },
        skill_mentions,
    }))
}

const LOCAL_SESSION_MAX_READ_BYTES: usize = 512_000;
const LOCAL_SESSION_MAX_LINE_BYTES: usize = 64_000;

fn read_local_session_file_content(path: &Path) -> Result<String, ServiceError> {
    let file = fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut content = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }
        if should_skip_local_session_sidecar_line(&line) {
            continue;
        }
        if content.len() + line.len() > LOCAL_SESSION_MAX_READ_BYTES {
            if let Some(compacted) = compact_oversized_local_session_json_line(&line) {
                if content.len() + compacted.len() <= LOCAL_SESSION_MAX_READ_BYTES {
                    content.push_str(&compacted);
                    continue;
                }
            }
            if content.is_empty() {
                content.push_str(&truncate_chars(&line, LOCAL_SESSION_MAX_READ_BYTES));
            }
            break;
        }
        if line.len() > LOCAL_SESSION_MAX_LINE_BYTES {
            if let Some(compacted) = compact_oversized_local_session_json_line(&line) {
                content.push_str(&compacted);
                continue;
            }
            content.push_str(&truncate_chars(&line, LOCAL_SESSION_MAX_LINE_BYTES));
            content.push('\n');
            break;
        }
        content.push_str(&line);
    }

    Ok(content)
}

fn compact_oversized_local_session_json_line(line: &str) -> Option<String> {
    let mut value = serde_json::from_str::<Value>(line).ok()?;
    prune_large_local_session_json_values(&mut value);
    let mut compacted = serde_json::to_string(&value).ok()?;
    compacted.push('\n');
    Some(compacted)
}

fn prune_large_local_session_json_values(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                prune_large_local_session_json_values(item);
            }
        }
        Value::Object(map) => {
            for (key, nested) in map.iter_mut() {
                if matches!(
                    key.as_str(),
                    "base64" | "blob" | "bytes" | "data" | "image" | "image_data"
                ) && nested.as_str().is_some_and(|text| text.len() > 512)
                {
                    *nested = Value::String("<omitted-local-session-blob>".to_string());
                    continue;
                }
                prune_large_local_session_json_values(nested);
            }
        }
        Value::String(text) if text.len() > 4_096 => {
            *text = truncate_chars(text, 4_096);
        }
        _ => {}
    }
}

fn should_skip_local_session_sidecar_line(line: &str) -> bool {
    let prefix = line.chars().take(4_096).collect::<String>();
    [
        "attachment",
        "file-history-snapshot",
        "last-prompt",
        "mode",
        "permission-mode",
        "queue-operation",
    ]
    .iter()
    .any(|record_type| {
        prefix.contains(&format!("\"type\":\"{record_type}\""))
            || prefix.contains(&format!("\"type\": \"{record_type}\""))
    })
}

fn local_session_row_id(path: &Path) -> String {
    let path_key = local_session_normalized_path(path);
    let path_hash = trace_content_hash(&path_key)
        .chars()
        .take(16)
        .collect::<String>();
    format!("local-session-{path_hash}")
}

fn enrich_local_session_content(path: &Path, root: &Path, file_content: &str) -> String {
    let Some(agent) = infer_local_session_agent(path) else {
        return file_content.to_string();
    };
    if agent != AgentId::Opencode.as_str() && agent != "opencode" {
        return file_content.to_string();
    }
    let Ok(value) = serde_json::from_str::<Value>(file_content) else {
        return file_content.to_string();
    };
    let Some(session_id) = value.get("id").and_then(Value::as_str) else {
        return file_content.to_string();
    };
    let Some(storage_root) = opencode_storage_root(path) else {
        return file_content.to_string();
    };

    let mut chunks = vec![file_content.to_string()];
    let message_root = storage_root.join("message").join(session_id);
    if let Some(message_root) = authorized_local_session_extra_dir(root, &message_root) {
        let Ok(entries) = fs::read_dir(&message_root) else {
            return chunks.join("\n");
        };
        let mut message_paths = entries
            .flatten()
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_file())
                    .map(|_| entry.path())
            })
            .collect::<Vec<_>>();
        message_paths.sort();
        for message_path in message_paths.into_iter().take(240) {
            let Some(message_path) = authorized_local_session_extra_file(root, &message_path)
            else {
                continue;
            };
            if let Ok(message) = fs::read_to_string(&message_path) {
                chunks.push(message.clone());
                if let Ok(message_value) = serde_json::from_str::<Value>(&message) {
                    if let Some(message_id) = message_value.get("id").and_then(Value::as_str) {
                        append_opencode_parts(&storage_root, root, message_id, &mut chunks);
                    }
                }
            }
        }
    }
    chunks.join("\n")
}

fn append_opencode_parts(
    storage_root: &Path,
    root: &Path,
    message_id: &str,
    chunks: &mut Vec<String>,
) {
    let part_root = storage_root.join("part").join(message_id);
    let Some(part_root) = authorized_local_session_extra_dir(root, &part_root) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&part_root) else {
        return;
    };
    let mut part_paths = entries
        .flatten()
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_file())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    part_paths.sort();
    for part_path in part_paths.into_iter().take(240) {
        let Some(part_path) = authorized_local_session_extra_file(root, &part_path) else {
            continue;
        };
        if let Ok(part) = fs::read_to_string(part_path) {
            chunks.push(part);
        }
    }
}

fn authorized_local_session_extra_dir(root: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn authorized_local_session_extra_file(root: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn opencode_storage_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory.file_name().and_then(|name| name.to_str()) == Some("storage") {
            return Some(directory.to_path_buf());
        }
        current = directory.parent();
    }
    None
}

fn local_session_parsed_metadata(
    path: &Path,
    file_content: &str,
    content: &str,
) -> LocalSessionParsedMetadata {
    let mut metadata = LocalSessionParsedMetadata::default();
    let mut parsed_json = false;
    if path
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|stem| stem.starts_with("ses_"))
    {
        metadata.session_id = path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_string);
    }

    for text in [file_content, content] {
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                if metadata.title.is_none() && !parsed_json {
                    metadata.title = local_session_text_title_candidate(line);
                }
                continue;
            };
            parsed_json = true;
            merge_local_session_metadata(&value, &mut metadata);
        }
    }
    if metadata.title.is_none() && !parsed_json {
        metadata.title = local_session_text_title_candidate(content);
    }
    metadata
}

fn merge_local_session_metadata(value: &Value, metadata: &mut LocalSessionParsedMetadata) {
    match value {
        Value::Array(items) => {
            for item in items {
                merge_local_session_metadata(item, metadata);
            }
        }
        Value::Object(map) => {
            if let Some(title) = json_session_title_candidate(map) {
                let is_ai_title = map.get("type").and_then(Value::as_str) == Some("ai-title");
                if metadata.title.is_none() || is_ai_title {
                    metadata.title = Some(title);
                }
            }
            if metadata.project_root.is_none() {
                metadata.project_root = json_session_project_candidate(map);
            }
            if metadata.session_id.is_none() {
                metadata.session_id = json_session_id_candidate(map);
            }
            for (key, nested) in map {
                if matches!(
                    key.as_str(),
                    "content" | "text" | "arguments" | "output" | "description"
                ) {
                    continue;
                }
                merge_local_session_metadata(nested, metadata);
            }
        }
        _ => {}
    }
}

fn json_session_title_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    if map.get("type").and_then(Value::as_str) == Some("ai-title") {
        return map
            .get("aiTitle")
            .and_then(Value::as_str)
            .and_then(local_session_text_title_candidate);
    }
    for key in ["aiTitle", "title", "display", "task", "thread_name"] {
        if let Some(title) = map.get(key).and_then(Value::as_str) {
            if let Some(title) = local_session_text_title_candidate(title) {
                return Some(title);
            }
        }
    }
    if let Some(payload) = map.get("payload").and_then(Value::as_object) {
        if payload.get("type").and_then(Value::as_str) == Some("user_message") {
            if let Some(message) = payload.get("message").and_then(Value::as_str) {
                return local_session_text_title_candidate(message);
            }
        }
    }
    let role = json_session_role(map);
    if role.is_some_and(|role| matches!(role, "user" | "human" | "customer")) {
        if let Some(text) = json_session_text_for_kind(map, "user_message") {
            return local_session_text_title_candidate(&text);
        }
    }
    None
}

fn local_session_text_title_candidate(text: &str) -> Option<String> {
    let candidates = normalize_local_session_title_candidates(text);
    if candidates
        .first()
        .is_some_and(|candidate| is_internal_local_session_title_block(candidate))
    {
        return None;
    }
    for candidate in candidates {
        if !candidate.is_empty() && !is_unhelpful_local_session_title(&candidate) {
            return Some(truncate_chars(&candidate, 120));
        }
    }
    None
}

fn normalize_local_session_title_candidates(text: &str) -> Vec<String> {
    let mut value = text.trim().replace('\r', "\n");
    for prefix in [
        "<command-message>",
        "</command-message>",
        "<command-name>",
        "</command-name>",
        "<command-args>",
        "</command-args>",
    ] {
        value = value.replace(prefix, " ");
    }
    if let Some(stripped) = value.strip_prefix("Task:") {
        value = stripped.trim().to_string();
    }
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_matches(['"', '\'', '`', ' ']).to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_internal_local_session_title_block(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("# agents.md instructions")
        || lower.starts_with("<permissions instructions>")
        || lower.starts_with("<environment_context>")
        || lower.starts_with("<local-command-caveat>")
        || lower.starts_with("<command-")
        || lower.starts_with("<skill name=")
        || lower.starts_with("<turn_")
        || lower.starts_with("you are a delegated subagent")
        || lower.starts_with("you are codex")
        || lower.starts_with("shared instruction entrypoint")
}

fn is_unhelpful_local_session_title(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let trimmed = value.trim();
    lower.starts_with("# agents.md instructions")
        || lower.starts_with("<permissions instructions>")
        || lower.starts_with("<environment_context>")
        || lower.starts_with("<local-command-caveat>")
        || lower.starts_with("<command-")
        || lower.starts_with("<skill name=")
        || lower.starts_with("<turn_")
        || lower.starts_with("you are a delegated subagent")
        || lower.starts_with("you are codex")
        || lower.starts_with("shared instruction entrypoint")
        || lower == "normal"
        || lower == "head"
        || lower == "main"
        || lower == "null"
        || lower == "clear"
        || lower == "cls"
        || is_image_placeholder_local_session_title(trimmed)
        || trimmed.starts_with("$HOME")
        || trimmed.starts_with('/')
        || is_version_like_local_session_title(trimmed)
}

fn is_image_placeholder_local_session_title(value: &str) -> bool {
    let trimmed = value.trim();
    if !trimmed.starts_with("[Image #") {
        return false;
    }
    let remainder = trimmed
        .replace("[Image #", "")
        .replace(']', "")
        .replace(char::is_whitespace, "");
    !remainder.is_empty()
        && remainder
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn is_version_like_local_session_title(value: &str) -> bool {
    let value = value.trim();
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
        && value.contains('.')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

fn is_internal_local_session_message(kind: &str, text: &str) -> bool {
    matches!(kind, "user_message" | "agent_reply")
        && local_session_text_title_candidate(text).is_none()
}

fn local_session_storage_project_root(
    path: &Path,
    root: &Path,
    project_filter_roots: &[PathBuf],
) -> Option<String> {
    if project_filter_roots.is_empty() {
        return None;
    }
    let root_text = root.to_string_lossy();
    let path_text = path.to_string_lossy();
    for project in project_filter_roots {
        let project_text = local_session_normalized_path(project);
        let claude_marker = format!(
            "/.claude/projects/{}",
            encode_claude_project_session_dir(project)
        );
        if path_or_root_contains_session_marker(&path_text, &root_text, &claude_marker) {
            return Some(project_text);
        }
        for encoded in encode_pi_project_session_dirs(project) {
            let pi_marker = format!("/.pi/agent/sessions/{encoded}");
            if path_or_root_contains_session_marker(&path_text, &root_text, &pi_marker) {
                return Some(project_text);
            }
        }
    }
    None
}

fn path_or_root_contains_session_marker(path_text: &str, root_text: &str, marker: &str) -> bool {
    [path_text, root_text]
        .into_iter()
        .map(|value| value.replace('\\', "/"))
        .any(|value| value.ends_with(marker) || value.contains(&format!("{marker}/")))
}

fn codex_session_index_title(path: &Path, session_id: &str) -> Option<String> {
    let codex_root = local_session_agent_store_root(path, ".codex")?;
    for index_file_name in ["session_index.jsonl", "history.jsonl"] {
        let index_path = codex_root.join(index_file_name);
        let Ok(index_content) = fs::read_to_string(index_path) else {
            continue;
        };
        for line in index_content.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(map) = value.as_object() else {
                continue;
            };
            let row_id = map
                .get("id")
                .or_else(|| map.get("session_id"))
                .and_then(Value::as_str);
            if row_id != Some(session_id) {
                continue;
            }
            for key in ["thread_name", "title", "text"] {
                if let Some(title) = map.get(key).and_then(Value::as_str) {
                    if let Some(title) = local_session_text_title_candidate(title) {
                        return Some(title);
                    }
                }
            }
        }
    }
    None
}

fn local_session_agent_store_root(path: &Path, directory_name: &str) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor.file_name().and_then(|name| name.to_str()) == Some(directory_name)
        })
        .map(Path::to_path_buf)
}

fn json_session_project_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in [
        "cwd",
        "current_cwd",
        "current_dir",
        "directory",
        "worktree",
        "workspace",
        "project",
        "projectRoot",
        "project_root",
    ] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if Path::new(value).is_absolute() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn json_session_id_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in ["sessionId", "session_id", "sessionID", "id"] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn local_session_matches_scope(
    scope: LocalSessionScope,
    project_filter_roots: &[PathBuf],
    session_project_root: Option<&str>,
) -> bool {
    if scope == LocalSessionScope::All {
        return true;
    }
    let Some(session_project_root) = session_project_root else {
        return false;
    };
    if project_filter_roots.is_empty() {
        return false;
    }
    let session_path = PathBuf::from(session_project_root);
    project_filter_roots
        .iter()
        .any(|project| local_session_paths_match(project, &session_path))
}

fn local_session_paths_match(project: &Path, session_path: &Path) -> bool {
    let left = local_session_normalized_path(project);
    let right = local_session_normalized_path(session_path);
    left == right || right.starts_with(&(left + "/"))
}

fn local_session_normalized_path(path: &Path) -> String {
    let normalized = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    normalized
        .strip_prefix("//?/")
        .unwrap_or(&normalized)
        .trim_end_matches('/')
        .to_string()
}

fn local_session_matches_search(
    search: Option<&str>,
    title: &str,
    project_root: Option<&str>,
    redacted_path: &str,
    excerpt: &str,
) -> bool {
    let Some(search) = search else {
        return true;
    };
    let search = search.trim();
    if search.is_empty() {
        return true;
    }
    title.to_ascii_lowercase().contains(search)
        || project_root.is_some_and(|project| project.to_ascii_lowercase().contains(search))
        || redacted_path.to_ascii_lowercase().contains(search)
        || excerpt.to_ascii_lowercase().contains(search)
}

fn push_session_skill_needle(needles: &mut Vec<String>, value: &str) {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() < 3 {
        return;
    }
    if needles.iter().any(|needle| needle == &normalized) {
        return;
    }
    needles.push(normalized);
}

fn detect_local_session_skill_mentions(
    content: &str,
    skill_matchers: &[LocalSessionSkillMatcher],
    evidence_ref: &str,
) -> Vec<LocalSessionSkillMention> {
    if skill_matchers.is_empty() {
        return Vec::new();
    }
    let invocations = extract_skill_invocation_names(content);
    if invocations.is_empty() {
        return Vec::new();
    }
    skill_matchers
        .iter()
        .filter_map(|matcher| {
            let matched_invocations = invocations
                .iter()
                .filter(|invocation| {
                    matcher
                        .needles
                        .iter()
                        .any(|needle| invocation.as_str() == needle)
                })
                .cloned()
                .collect::<Vec<_>>();
            let count = matched_invocations.len();
            (count > 0).then(|| LocalSessionSkillMention {
                skill_id: matcher.skill_id.clone(),
                skill_name: matcher.skill_name.clone(),
                agent: matcher.agent.clone(),
                count,
                matched_invocations,
                evidence_ref: evidence_ref.to_string(),
            })
        })
        .collect()
}

fn extract_skill_invocation_names(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut names = Vec::new();
    for pattern in ["/skill:", "/skill ", "skill:"] {
        let mut start = 0usize;
        while let Some(relative) = lower[start..].find(pattern) {
            let pattern_start = start + relative;
            let offset = pattern_start + pattern.len();
            if !is_skill_invocation_left_boundary(&lower, pattern_start) {
                start = next_skill_invocation_search_start(&lower, offset);
                continue;
            }
            if pattern == "skill:"
                && pattern_start > 0
                && lower.as_bytes().get(pattern_start - 1) == Some(&b'/')
            {
                start = next_skill_invocation_search_start(&lower, offset);
                continue;
            }
            let name = read_skill_invocation_name(&lower[offset..]);
            if !name.is_empty() {
                names.push(name);
            }
            start = next_skill_invocation_search_start(&lower, offset);
            if start >= lower.len() {
                break;
            }
        }
    }
    names.sort();
    names
}

fn next_skill_invocation_search_start(value: &str, offset: usize) -> usize {
    if offset >= value.len() {
        return value.len();
    }
    value[offset..]
        .char_indices()
        .nth(1)
        .map(|(relative, _)| offset + relative)
        .unwrap_or(value.len())
}

fn is_skill_invocation_left_boundary(value: &str, pattern_start: usize) -> bool {
    if pattern_start == 0 {
        return true;
    }
    value[..pattern_start]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
}

fn read_skill_invocation_name(value: &str) -> String {
    let mut name = String::new();
    for character in value.trim_start().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.') {
            name.push(character);
        } else {
            break;
        }
    }
    name.trim_matches(|character: char| {
        matches!(
            character,
            '-' | '_' | ':' | '.' | '/' | '\\' | '"' | '\'' | '`'
        )
    })
    .to_string()
}

fn update_local_session_skill_usage(
    usage: &mut BTreeMap<String, LocalSessionSkillUsageAccumulator>,
    entry: &LocalSessionPreviewEntry,
) {
    for mention in &entry.skill_mentions {
        let accumulator = usage.entry(mention.skill_id.clone()).or_insert_with(|| {
            LocalSessionSkillUsageAccumulator {
                skill_id: mention.skill_id.clone(),
                skill_name: mention.skill_name.clone(),
                agent: mention.agent.clone(),
                ..Default::default()
            }
        });
        accumulator.call_count += mention.count;
        accumulator.session_count += 1;
        accumulator.latest_modified_at =
            max_optional_millis(accumulator.latest_modified_at, entry.row.modified_at);
        if !accumulator
            .evidence_refs
            .iter()
            .any(|reference| reference == &mention.evidence_ref)
            && accumulator.evidence_refs.len() < 6
        {
            accumulator.evidence_refs.push(mention.evidence_ref.clone());
        }
    }
}

fn max_optional_millis(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn local_session_skill_usage_rows(
    usage: BTreeMap<String, LocalSessionSkillUsageAccumulator>,
    limit: usize,
) -> Vec<LocalSessionSkillUsageRow> {
    let mut rows = usage
        .into_values()
        .map(|row| LocalSessionSkillUsageRow {
            skill_id: row.skill_id,
            skill_name: row.skill_name,
            agent: row.agent,
            call_count: row.call_count,
            session_count: row.session_count,
            latest_modified_at: row.latest_modified_at,
            evidence_refs: row.evidence_refs,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .call_count
            .cmp(&left.call_count)
            .then_with(|| right.session_count.cmp(&left.session_count))
            .then_with(|| right.latest_modified_at.cmp(&left.latest_modified_at))
            .then_with(|| left.skill_name.cmp(&right.skill_name))
    });
    rows.truncate(limit);
    rows
}

fn local_session_content_items(
    content: &str,
    short_hash: &str,
    max_item_chars: usize,
    skill_mentions: &[LocalSessionSkillMention],
    redactor: &mut PromptRedactor<'_>,
) -> Vec<LocalSessionContentItem> {
    const MAX_SESSION_CONTENT_ITEMS: usize = 240;
    let mut drafts = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => {
                let timestamp = json_session_timestamp_millis(&value);
                collect_json_session_content_drafts(&value, timestamp, &mut drafts);
            }
            Err(_) => collect_text_session_content_drafts(trimmed, None, &mut drafts),
        }
        if drafts.len() >= MAX_SESSION_CONTENT_ITEMS {
            break;
        }
    }

    let mut matched_invocations = BTreeSet::new();
    for mention in skill_mentions {
        for invocation in &mention.matched_invocations {
            matched_invocations.insert(invocation.clone());
        }
        let text = if mention.count > 1 {
            format!("{} ({} calls)", mention.skill_name, mention.count)
        } else {
            mention.skill_name.clone()
        };
        drafts.push(LocalSessionContentDraft {
            kind: "skill_call".to_string(),
            title: format!("Skill: {}", mention.skill_name),
            text,
            timestamp: None,
            evidence_refs: vec![mention.evidence_ref.clone()],
        });
    }
    let mut unmatched_invocations = BTreeMap::<String, usize>::new();
    for invocation in extract_skill_invocation_names(content) {
        if matched_invocations.contains(&invocation) {
            continue;
        }
        *unmatched_invocations.entry(invocation).or_default() += 1;
    }
    for (invocation, count) in unmatched_invocations {
        let text = if count > 1 {
            format!("{invocation} ({count} calls)")
        } else {
            invocation.clone()
        };
        drafts.push(LocalSessionContentDraft {
            kind: "skill_call".to_string(),
            title: format!("Skill: {invocation}"),
            text,
            timestamp: None,
            evidence_refs: vec![format!("session.content_hash:{short_hash}")],
        });
    }

    if drafts.is_empty() {
        drafts.push(LocalSessionContentDraft {
            kind: "agent_reply".to_string(),
            title: "Session excerpt".to_string(),
            text: content.to_string(),
            timestamp: None,
            evidence_refs: vec![format!("session.content_hash:{short_hash}")],
        });
    }

    drafts
        .into_iter()
        .take(MAX_SESSION_CONTENT_ITEMS)
        .enumerate()
        .map(|(index, draft)| {
            let redacted = truncate_chars(
                &redact_local_session_content(redactor, &draft.text),
                max_item_chars,
            );
            LocalSessionContentItem {
                id: format!("session-item-{short_hash}-{index}"),
                kind: draft.kind,
                title: truncate_chars(&redactor.redact(&draft.title), 120),
                char_count: redacted.chars().count(),
                text: redacted,
                timestamp: draft.timestamp,
                evidence_refs: draft.evidence_refs,
            }
        })
        .collect()
}

fn local_session_time_bounds(
    content: &str,
    content_items: &[LocalSessionContentItem],
    fallback_timestamp: Option<i64>,
) -> (Option<i64>, Option<i64>) {
    let mut bounds = LocalSessionTimeBounds::default();
    for item in content_items {
        bounds.push(item.timestamp);
    }

    if bounds.started_at.is_none() || bounds.ended_at.is_none() {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                bounds.push(json_session_timestamp_millis(&value));
            }
        }
    }

    let started_at = bounds.started_at.or(fallback_timestamp);
    let ended_at = bounds.ended_at.or(started_at).or(fallback_timestamp);
    (started_at, ended_at)
}

fn json_session_timestamp_millis(value: &Value) -> Option<i64> {
    let Value::Object(map) = value else {
        return json_session_timestamp_value_millis(value);
    };
    for key in [
        "timestamp",
        "created_at",
        "createdAt",
        "updated_at",
        "updatedAt",
        "completed_at",
        "completedAt",
        "time",
    ] {
        if let Some(timestamp) = map.get(key).and_then(json_session_timestamp_value_millis) {
            return Some(timestamp);
        }
    }
    None
}

fn json_session_timestamp_value_millis(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => parse_local_session_timestamp_millis(text),
        Value::Number(number) => number
            .as_i64()
            .and_then(normalize_local_session_epoch_millis)
            .or_else(|| {
                number
                    .as_f64()
                    .and_then(normalize_local_session_epoch_millis_from_float)
            }),
        _ => None,
    }
}

fn parse_local_session_timestamp_millis(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(epoch) = trimmed.parse::<i64>() {
        return normalize_local_session_epoch_millis(epoch);
    }
    if let Ok(epoch) = trimmed.parse::<f64>() {
        return normalize_local_session_epoch_millis_from_float(epoch);
    }
    let parsed = OffsetDateTime::parse(trimmed, &Rfc3339).ok()?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    i64::try_from(millis).ok()
}

fn normalize_local_session_epoch_millis(value: i64) -> Option<i64> {
    let magnitude = value.checked_abs().unwrap_or(i64::MAX);
    if magnitude >= 10_000_000_000 {
        Some(value)
    } else {
        value.checked_mul(1_000)
    }
}

fn normalize_local_session_epoch_millis_from_float(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let millis = if value.abs() >= 10_000_000_000.0 {
        value
    } else {
        value * 1_000.0
    };
    if millis < i64::MIN as f64 || millis > i64::MAX as f64 {
        return None;
    }
    Some(millis.round() as i64)
}

fn redact_local_session_content(redactor: &mut PromptRedactor<'_>, value: &str) -> String {
    let owner_redacted = redact_unix_listing_owners(value);
    redactor.redact(&owner_redacted)
}

fn redact_unix_listing_owners(value: &str) -> String {
    value
        .lines()
        .map(redact_unix_listing_owner_line_with_escaped_newlines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_unix_listing_owner_line_with_escaped_newlines(line: &str) -> String {
    line.split("\\n")
        .map(redact_unix_listing_owner_line)
        .collect::<Vec<_>>()
        .join("\\n")
}

fn redact_unix_listing_owner_line(line: &str) -> String {
    let leading_len = line.len() - line.trim_start().len();
    let leading = &line[..leading_len];
    let tokens = line[leading_len..].split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 4
        || !is_unix_listing_mode(tokens[0])
        || !tokens[1].chars().all(|ch| ch.is_ascii_digit())
    {
        return line.to_string();
    }

    let mut redacted = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    redacted[2] = "<user>".to_string();
    redacted[3] = "<group>".to_string();
    format!("{leading}{}", redacted.join(" "))
}

fn is_unix_listing_mode(token: &str) -> bool {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() < 10 {
        return false;
    }
    if !matches!(chars[0], '-' | 'd' | 'l' | 'b' | 'c' | 'p' | 's') {
        return false;
    }
    chars[1..10]
        .iter()
        .all(|ch| matches!(ch, 'r' | 'w' | 'x' | 's' | 'S' | 't' | 'T' | '-'))
}

fn collect_json_session_content_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_json_session_content_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(map) => {
            let timestamp = json_session_timestamp_millis(value).or(inherited_timestamp);
            let role = json_session_role(map).map(str::to_string);
            let mut pushed_direct_tool = false;
            if let Some(kind) = json_session_content_kind(map, role.as_deref()) {
                let text = json_session_text_for_kind(map, kind);
                if let Some(text) = text {
                    if !text.trim().is_empty() && !is_internal_local_session_message(kind, &text) {
                        let resolved_kind = local_session_content_kind_for_text(kind, &text, map);
                        pushed_direct_tool = resolved_kind == "tool_call";
                        push_local_session_text_drafts(
                            drafts,
                            resolved_kind,
                            json_session_title(map, resolved_kind),
                            text,
                            timestamp,
                            Vec::new(),
                        );
                    }
                }
            }

            if !pushed_direct_tool {
                collect_json_tool_call_drafts(map, timestamp, drafts);
            }

            for (key, nested) in map {
                if matches!(
                    key.as_str(),
                    "author"
                        | "content"
                        | "text"
                        | "message"
                        | "delta"
                        | "tool_calls"
                        | "toolCalls"
                        | "tool_use"
                        | "toolUse"
                        | "function_call"
                        | "parts"
                ) {
                    continue;
                }
                collect_json_session_content_drafts(nested, timestamp, drafts);
            }
        }
        Value::String(text) => {
            collect_text_session_content_drafts(text, inherited_timestamp, drafts)
        }
        _ => {}
    }
}

fn collect_json_tool_call_drafts(
    map: &serde_json::Map<String, Value>,
    timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    for key in ["content", "parts"] {
        if let Some(nested) = map.get(key) {
            collect_json_tool_part_drafts(nested, timestamp, drafts);
        }
    }
    if let Some(message) = map.get("message").and_then(Value::as_object) {
        for key in ["content", "parts"] {
            if let Some(nested) = message.get(key) {
                collect_json_tool_part_drafts(nested, timestamp, drafts);
            }
        }
    }
    for key in [
        "tool_calls",
        "toolCalls",
        "tool_use",
        "toolUse",
        "function_call",
    ] {
        let Some(value) = map.get(key) else {
            continue;
        };
        match value {
            Value::Array(items) => {
                for item in items {
                    drafts.push(LocalSessionContentDraft {
                        kind: "tool_call".to_string(),
                        title: json_tool_title(item),
                        text: compact_json_session_text(item),
                        timestamp,
                        evidence_refs: Vec::new(),
                    });
                }
            }
            Value::Object(_) | Value::String(_) => {
                drafts.push(LocalSessionContentDraft {
                    kind: "tool_call".to_string(),
                    title: json_tool_title(value),
                    text: compact_json_session_text(value),
                    timestamp,
                    evidence_refs: Vec::new(),
                });
            }
            _ => {}
        }
    }
}

fn collect_json_tool_part_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_json_tool_part_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(map) => {
            let timestamp = json_session_timestamp_millis(value).or(inherited_timestamp);
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
                if is_json_tool_type(&normalized) {
                    drafts.push(LocalSessionContentDraft {
                        kind: "tool_call".to_string(),
                        title: json_tool_title(value),
                        text: json_tool_payload_text(value),
                        timestamp,
                        evidence_refs: Vec::new(),
                    });
                    return;
                }
            }
            for key in ["content", "parts"] {
                if let Some(nested) = map.get(key) {
                    collect_json_tool_part_drafts(nested, timestamp, drafts);
                }
            }
            if let Some(message) = map.get("message").and_then(Value::as_object) {
                for key in ["content", "parts"] {
                    if let Some(nested) = message.get(key) {
                        collect_json_tool_part_drafts(nested, timestamp, drafts);
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_text_session_content_drafts(
    line: &str,
    timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    let lower = line.to_ascii_lowercase();
    let (kind, title, text) = if let Some(text) =
        strip_session_line_prefix(line, &["user:", "human:", "用户：", "用户:"])
    {
        ("user_message", "User", text)
    } else if let Some(text) =
        strip_session_line_prefix(line, &["assistant:", "agent:", "助手：", "助手:"])
    {
        ("agent_reply", "Agent", text)
    } else if let Some(text) = strip_session_line_prefix(
        line,
        &["thinking:", "reasoning:", "thought:", "思考：", "思考:"],
    ) {
        ("thinking", "Thinking", text)
    } else if let Some(text) =
        strip_session_line_prefix(line, &["tool:", "function:", "工具：", "工具:"])
    {
        ("tool_call", "Tool", text)
    } else if is_tool_result_text(line)
        || lower.contains("tool_call")
        || lower.contains("tool_use")
        || lower.contains("function_call")
    {
        ("tool_call", "Tool", line)
    } else if !extract_skill_invocation_names(line).is_empty() {
        ("skill_call", "Skill", line)
    } else {
        return;
    };
    push_local_session_text_drafts(
        drafts,
        kind,
        title.to_string(),
        text.trim().to_string(),
        timestamp,
        Vec::new(),
    );
}

fn strip_session_line_prefix<'a>(line: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    for prefix in prefixes {
        if lower.starts_with(&prefix.to_ascii_lowercase()) {
            return Some(line[prefix.len()..].trim());
        }
    }
    None
}

fn push_local_session_text_drafts(
    drafts: &mut Vec<LocalSessionContentDraft>,
    kind: &str,
    title: String,
    text: String,
    timestamp: Option<i64>,
    evidence_refs: Vec<String>,
) {
    for (segment_kind, segment_text) in split_inline_thinking_segments(kind, &text) {
        drafts.push(LocalSessionContentDraft {
            kind: segment_kind.to_string(),
            title: if segment_kind == kind {
                title.clone()
            } else {
                json_fallback_session_title(segment_kind)
            },
            text: segment_text,
            timestamp,
            evidence_refs: evidence_refs.clone(),
        });
    }
}

fn split_inline_thinking_segments(default_kind: &str, text: &str) -> Vec<(&'static str, String)> {
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    let lower = text.to_ascii_lowercase();

    while let Some(start_offset) = lower[cursor..].find("<think>") {
        let start = cursor + start_offset;
        let before = text[cursor..start].trim();
        if !before.is_empty() {
            segments.push((stable_session_kind(default_kind), before.to_string()));
        }

        let inner_start = start + "<think>".len();
        if let Some(end_offset) = lower[inner_start..].find("</think>") {
            let end = inner_start + end_offset;
            let thinking = text[inner_start..end].trim();
            if !thinking.is_empty() {
                segments.push(("thinking", thinking.to_string()));
            }
            cursor = end + "</think>".len();
        } else {
            let thinking = text[inner_start..].trim();
            if !thinking.is_empty() {
                segments.push(("thinking", thinking.to_string()));
            }
            cursor = text.len();
            break;
        }
    }

    let after = text[cursor..].trim();
    if !after.is_empty() {
        segments.push((stable_session_kind(default_kind), after.to_string()));
    }

    if segments.is_empty() && !text.trim().is_empty() {
        segments.push((stable_session_kind(default_kind), text.trim().to_string()));
    }
    segments
}

fn stable_session_kind(kind: &str) -> &'static str {
    match kind {
        "user_message" => "user_message",
        "agent_reply" => "agent_reply",
        "thinking" => "thinking",
        "tool_call" => "tool_call",
        "skill_call" => "skill_call",
        _ => "agent_reply",
    }
}

fn json_fallback_session_title(kind: &str) -> String {
    match kind {
        "user_message" => "User".to_string(),
        "agent_reply" => "Agent".to_string(),
        "thinking" => "Thinking".to_string(),
        "tool_call" => "Tool".to_string(),
        "skill_call" => "Skill".to_string(),
        _ => "Session".to_string(),
    }
}

fn json_session_content_kind(
    map: &serde_json::Map<String, Value>,
    role: Option<&str>,
) -> Option<&'static str> {
    if let Some(kind) = map.get("type").and_then(Value::as_str) {
        let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
        match normalized.as_str() {
            "user" | "human" => return Some("user_message"),
            value if is_json_thinking_type(value) => return Some("thinking"),
            value if is_json_tool_type(value) => return Some("tool_call"),
            _ => {}
        }
    }
    if json_thinking_payload_text(&Value::Object(map.clone())).is_some() {
        return Some("thinking");
    }
    if json_session_text(map)
        .as_deref()
        .is_some_and(is_tool_result_text)
    {
        return Some("tool_call");
    }
    role.and_then(|role| match role.to_ascii_lowercase().as_str() {
        "user" | "human" | "customer" => Some("user_message"),
        "assistant" | "agent" | "model" => Some("agent_reply"),
        "tool" | "function" | "toolresult" | "tool_result" => Some("tool_call"),
        "system" | "developer" => None,
        _ => None,
    })
}

fn json_session_title(map: &serde_json::Map<String, Value>, kind: &str) -> String {
    for key in ["name", "tool_name", "function_name", "title"] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return value.trim().to_string();
            }
        }
    }
    match kind {
        "user_message" => "User".to_string(),
        "agent_reply" => "Agent".to_string(),
        "thinking" => "Thinking".to_string(),
        "tool_call" => "Tool".to_string(),
        "skill_call" => "Skill".to_string(),
        _ => "Session".to_string(),
    }
}

fn json_session_text(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in [
        "content",
        "text",
        "message",
        "delta",
        "thinking",
        "reasoning",
        "summary",
        "result",
    ] {
        if let Some(value) = map.get(key).and_then(json_value_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn json_session_text_for_kind(map: &serde_json::Map<String, Value>, kind: &str) -> Option<String> {
    if kind == "tool_call" || is_json_tool_object(map) {
        return json_tool_payload_text(&Value::Object(map.clone())).into();
    }
    if kind == "thinking" {
        if let Some(text) = json_thinking_payload_text(&Value::Object(map.clone())) {
            return Some(text);
        }
    }

    for key in [
        "content",
        "text",
        "message",
        "delta",
        "thinking",
        "reasoning",
        "summary",
        "result",
    ] {
        if let Some(value) = map.get(key).and_then(json_non_tool_message_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    json_session_text(map)
        .filter(|text| !is_tool_result_text(text))
        .filter(|_| !json_session_contains_tool_payload(map))
}

fn json_session_contains_tool_payload(map: &serde_json::Map<String, Value>) -> bool {
    map.values().any(json_value_contains_tool_payload)
}

fn json_value_contains_tool_payload(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(json_value_contains_tool_payload),
        Value::Object(map) => {
            is_json_tool_object(map)
                || [
                    "tool_calls",
                    "toolCalls",
                    "tool_use",
                    "toolUse",
                    "function_call",
                ]
                .iter()
                .any(|key| map.contains_key(*key))
                || map.values().any(json_value_contains_tool_payload)
        }
        _ => false,
    }
}

fn local_session_content_kind_for_text<'a>(
    kind: &'a str,
    text: &str,
    map: &serde_json::Map<String, Value>,
) -> &'a str {
    if kind == "agent_reply"
        && (json_session_has_tool_process_signal(map) || is_agent_process_note_text(text))
    {
        "thinking"
    } else {
        kind
    }
}

fn json_session_has_tool_process_signal(map: &serde_json::Map<String, Value>) -> bool {
    map.values().any(json_value_has_tool_process_signal)
}

fn json_value_has_tool_process_signal(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(json_value_has_tool_process_signal),
        Value::Object(map) => {
            for key in [
                "stop_reason",
                "stopReason",
                "finish_reason",
                "finishReason",
                "finish_details",
                "finishDetails",
            ] {
                if let Some(value) = map.get(key) {
                    if json_value_is_tool_process_signal(value) {
                        return true;
                    }
                }
            }
            map.values().any(json_value_has_tool_process_signal)
        }
        _ => false,
    }
}

fn json_value_is_tool_process_signal(value: &Value) -> bool {
    match value {
        Value::String(text) => is_tool_process_signal_text(text),
        Value::Object(map) => map.values().any(json_value_is_tool_process_signal),
        Value::Array(items) => items.iter().any(json_value_is_tool_process_signal),
        _ => false,
    }
}

fn is_tool_process_signal_text(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace(['_', '-', ' '], "");
    matches!(
        normalized.as_str(),
        "tool"
            | "tooluse"
            | "toolcall"
            | "toolcalls"
            | "functioncall"
            | "functioncalls"
            | "requiresaction"
    )
}

fn is_agent_process_note_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 320 {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    let english_starters = [
        "i need to",
        "i'll ",
        "i will ",
        "i should ",
        "i'm going to",
        "i apologize",
        "let me ",
        "sorry",
    ];
    let english_tool_terms = [
        "tool",
        "websearch",
        "webfetch",
        "search",
        "research",
        "investigate",
        "load",
        "gather",
    ];
    if english_starters
        .iter()
        .any(|starter| lower.starts_with(starter))
        && english_tool_terms.iter().any(|term| lower.contains(term))
    {
        return true;
    }

    let chinese_starters = ["我需要", "我会", "我先", "我来", "让我", "抱歉"];
    let chinese_tool_terms = ["工具", "搜索", "联网", "调研", "加载", "调用", "查询"];
    chinese_starters
        .iter()
        .any(|starter| trimmed.starts_with(starter))
        && chinese_tool_terms.iter().any(|term| trimmed.contains(term))
}

fn json_thinking_payload_text(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            let texts = items
                .iter()
                .filter_map(json_thinking_payload_text)
                .collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            let is_thinking_object = map
                .get("type")
                .and_then(Value::as_str)
                .map(|kind| {
                    is_json_thinking_type(&kind.to_ascii_lowercase().replace(['_', '-'], ""))
                })
                .unwrap_or(false);

            if is_thinking_object {
                for key in [
                    "thinking",
                    "reasoning",
                    "thought",
                    "text",
                    "content",
                    "summary",
                    "message",
                    "delta",
                ] {
                    if let Some(text) = map.get(key).and_then(json_value_text) {
                        if !text.trim().is_empty() {
                            return Some(text);
                        }
                    }
                }
            }

            for key in ["thinking", "reasoning", "thought"] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }

            for key in ["content", "parts", "message", "delta"] {
                if let Some(text) = map.get(key).and_then(json_thinking_payload_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn json_non_tool_message_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => (!is_tool_result_text(text)).then(|| text.clone()),
        Value::Array(items) => {
            let texts = items
                .iter()
                .filter_map(json_non_tool_message_text)
                .collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            if is_json_tool_object(map) {
                return None;
            }
            for key in [
                "text",
                "input",
                "summary",
                "content",
                "message",
                "delta",
                "thinking",
                "reasoning",
            ] {
                if let Some(text) = map.get(key).and_then(json_non_tool_message_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_json_tool_object(map: &serde_json::Map<String, Value>) -> bool {
    if map
        .get("type")
        .and_then(Value::as_str)
        .map(|kind| is_json_tool_type(&kind.to_ascii_lowercase().replace(['_', '-'], "")))
        .unwrap_or(false)
    {
        return true;
    }
    ["tool_use_id", "toolUseId", "tool_call_id", "toolCallId"]
        .iter()
        .any(|key| map.contains_key(*key))
}

fn is_json_thinking_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "thinking" | "thinkingtext" | "reasoning" | "reasoningtext" | "thought"
    )
}

fn is_json_tool_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "toolcall"
            | "tooluse"
            | "functioncall"
            | "toolresult"
            | "tooluseresult"
            | "tooluseerror"
            | "functionresult"
    )
}

fn is_tool_result_text(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    [
        "<tool_use_error>",
        "</tool_use_error>",
        "<tooluseerror>",
        "</tooluseerror>",
        "<tool_result>",
        "</tool_result>",
        "<toolresult>",
        "</toolresult>",
    ]
    .iter()
    .any(|marker| lower.starts_with(marker) || lower.contains(marker))
}

fn json_tool_payload_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Object(map) => {
            for key in [
                "content",
                "text",
                "message",
                "result",
                "error",
                "output",
                "input",
                "arguments",
            ] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return text;
                    }
                }
            }
            compact_json_session_text(value)
        }
        _ => compact_json_session_text(value),
    }
}

fn json_value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let texts = items.iter().filter_map(json_value_text).collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            for key in [
                "text",
                "content",
                "message",
                "input",
                "arguments",
                "name",
                "thinking",
                "summary",
            ] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn json_tool_title(value: &Value) -> String {
    if let Value::Object(map) = value {
        for key in ["name", "tool_name", "function_name"] {
            if let Some(name) = map.get(key).and_then(Value::as_str) {
                if !name.trim().is_empty() {
                    return name.trim().to_string();
                }
            }
        }
        if let Some(function) = map.get("function").and_then(Value::as_object) {
            if let Some(name) = function.get("name").and_then(Value::as_str) {
                if !name.trim().is_empty() {
                    return name.trim().to_string();
                }
            }
        }
        if let Some(kind) = map.get("type").and_then(Value::as_str) {
            let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
            return match normalized.as_str() {
                "toolresult" | "tooluseresult" | "functionresult" => "Tool result".to_string(),
                "tooluseerror" => "Tool error".to_string(),
                _ => "Tool".to_string(),
            };
        }
    }
    "Tool".to_string()
}

fn compact_json_session_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalSessionMetrics {
    user_message_count: usize,
    total_message_count: usize,
    tool_call_count: usize,
    skill_call_count: usize,
}

fn local_session_metrics(
    content: &str,
    content_items: &[LocalSessionContentItem],
) -> LocalSessionMetrics {
    let mut metrics = LocalSessionMetrics::default();
    for item in content_items {
        match item.kind.as_str() {
            "user_message" => {
                metrics.user_message_count += 1;
                metrics.total_message_count += 1;
            }
            "agent_reply" | "thinking" => {
                metrics.total_message_count += 1;
            }
            "tool_call" => {
                metrics.tool_call_count += 1;
            }
            "skill_call" => {}
            _ => {}
        }
    }

    if metrics.total_message_count == 0 {
        metrics.total_message_count = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
    }
    metrics.skill_call_count = count_skill_invocation_mentions(content);
    metrics
}

fn json_session_role(map: &serde_json::Map<String, Value>) -> Option<&str> {
    map.get("role")
        .and_then(Value::as_str)
        .or_else(|| map.get("sender").and_then(Value::as_str))
        .or_else(|| {
            map.get("message")
                .and_then(Value::as_object)
                .and_then(|message| message.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("item"))
                .and_then(Value::as_object)
                .and_then(|item| item.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("author")
                .and_then(Value::as_object)
                .and_then(|author| author.get("role"))
                .and_then(Value::as_str)
        })
}

fn count_skill_invocation_mentions(text: &str) -> usize {
    extract_skill_invocation_names(text).len()
}

fn local_session_title(path: &Path, short_hash: &str) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| truncate_chars(name, 120))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Local session {short_hash}"))
}

fn infer_local_session_agent(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().to_ascii_lowercase();
    if normalized.contains(".claude") {
        Some("claude-code".to_string())
    } else if normalized.contains(".codex") {
        Some("codex".to_string())
    } else if normalized.contains("opencode") {
        Some("opencode".to_string())
    } else if normalized.contains(".pi/") {
        Some("pi".to_string())
    } else {
        None
    }
}

fn local_session_modified_at(path: &Path) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as i64)
}
