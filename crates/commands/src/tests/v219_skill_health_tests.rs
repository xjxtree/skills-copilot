use super::*;

#[test]
fn health_summary_counts_triage_risk_and_analysis_groups() {
    let mut scripted = health_skill(
        "scripted",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    scripted.scripts.push(SkillScript {
        name: "setup".to_string(),
        path: PathBuf::from("/tmp/claude/review/scripts/setup.sh"),
        interpreter: Some("bash".to_string()),
        description: None,
        fingerprint: "script-fp".to_string(),
    });
    let scripted_project = health_skill(
        "scripted-project",
        AgentId::ClaudeCode,
        Scope::AgentProject,
        "review-diff",
        true,
        SkillState::Loaded,
    );

    let mut permissioned = health_skill(
        "permissioned",
        AgentId::Codex,
        Scope::AgentGlobal,
        "review-diff",
        false,
        SkillState::Disabled,
    );
    permissioned.permissions.network = NetworkAccess::Full;
    permissioned.permissions.exec = true;

    let broken = health_skill(
        "broken",
        AgentId::Hermes,
        Scope::AgentGlobal,
        "broken-skill",
        false,
        SkillState::Broken,
    );
    let missing = health_skill(
        "missing",
        AgentId::Openclaw,
        Scope::AgentProject,
        "missing-skill",
        false,
        SkillState::Missing,
    );

    let instances = vec![scripted, scripted_project, permissioned, broken, missing];
    let findings = vec![
        health_finding(
            "finding-script",
            Some("scripted"),
            None,
            "script.no-shebang",
            "info",
        ),
        health_finding(
            "finding-permission",
            Some("permissioned"),
            None,
            "permissions.exec-needs-human",
            "warning",
        ),
        health_finding(
            "finding-permission-duplicate",
            Some("permissioned"),
            None,
            "permissions.exec-needs-human",
            "warning",
        ),
        health_finding(
            "finding-malformed",
            Some("broken"),
            None,
            "frontmatter.required-fields",
            "error",
        ),
    ];
    let conflicts = vec![ConflictGroupRecord {
        id: "conflict-review-diff".to_string(),
        definition_id: "def.review-diff".to_string(),
        reason: "content-drift".to_string(),
        winner_id: Some("scripted".to_string()),
        instance_ids: vec!["scripted".to_string(), "scripted-project".to_string()],
    }];
    let analysis = analyze_skill_instances(&instances);

    let health = build_skill_health_summary(&instances, &findings, &conflicts, &analysis);

    assert_eq!(health.total_count, 5);
    assert_eq!(health.enabled_count, 2);
    assert_eq!(health.disabled_count, 3);
    assert_eq!(health.broken_count, 1);
    assert_eq!(health.missing_count, 1);
    assert_eq!(health.malformed_count, 2);
    assert_eq!(health.findings_by_severity.error_count, 1);
    assert_eq!(health.findings_by_severity.warning_count, 1);
    assert_eq!(health.findings_by_severity.info_count, 1);
    assert_eq!(health.finding_count, 3);
    assert_eq!(health.conflict_count, 1);
    assert_eq!(health.risky_script_count, 1);
    assert_eq!(health.risky_permission_count, 1);
    assert!(health.analysis_groups.total_count >= 2);
    assert_eq!(health.analysis_groups.duplicate_name_count, 1);
    assert_eq!(health.analysis_groups.malformed_count, 1);

    let codex = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "codex")
        .expect("codex health summary");
    assert_eq!(codex.total_count, 1);
    assert_eq!(codex.disabled_count, 1);
    assert_eq!(codex.finding_count, 1);
    assert_eq!(codex.conflict_count, 0);
    assert_eq!(codex.risky_permission_count, 1);
    assert!(codex.analysis_group_count >= 1);
}

#[test]
fn health_summary_dedupes_findings_and_counts_only_same_agent_runtime_conflicts() {
    let claude_user = health_skill(
        "claude-user-review",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let claude_project = health_skill(
        "claude-project-review",
        AgentId::ClaudeCode,
        Scope::AgentProject,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let codex_review = health_skill(
        "codex-review",
        AgentId::Codex,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let opencode_review = health_skill(
        "opencode-review",
        AgentId::Opencode,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let instances = vec![claude_user, claude_project, codex_review, opencode_review];
    let duplicate_finding = health_finding(
        "finding-1",
        Some("claude-user-review"),
        None,
        "body.too-long",
        "warning",
    );
    let mut duplicate_finding_with_new_id = duplicate_finding.clone();
    duplicate_finding_with_new_id.id = "finding-1-duplicate-row".to_string();
    let findings = vec![
        duplicate_finding,
        duplicate_finding_with_new_id,
        health_finding(
            "finding-2",
            Some("codex-review"),
            None,
            "permissions.exec-needs-human",
            "warning",
        ),
    ];
    let conflicts = vec![
        ConflictGroupRecord {
            id: "same-agent-claude-runtime".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "content-drift".to_string(),
            winner_id: Some("claude-user-review".to_string()),
            instance_ids: vec![
                "claude-user-review".to_string(),
                "claude-project-review".to_string(),
            ],
        },
        ConflictGroupRecord {
            id: "stale-cross-agent-duplicate".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "cross-agent-duplicate-name".to_string(),
            winner_id: None,
            instance_ids: vec![
                "claude-user-review".to_string(),
                "codex-review".to_string(),
                "opencode-review".to_string(),
            ],
        },
    ];
    let analysis = analyze_skill_instances(&instances);

    let health = build_skill_health_summary(&instances, &findings, &conflicts, &analysis);

    assert_eq!(health.finding_count, 2);
    assert_eq!(health.findings_by_severity.warning_count, 2);
    assert_eq!(health.conflict_count, 1);
    assert_eq!(health.analysis_groups.duplicate_name_count, 1);

    let claude = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "claude-code")
        .expect("claude health summary");
    assert_eq!(claude.finding_count, 1);
    assert_eq!(claude.conflict_count, 1);

    let codex = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "codex")
        .expect("codex health summary");
    assert_eq!(codex.finding_count, 1);
    assert_eq!(codex.conflict_count, 0);
}

#[test]
fn list_conflicts_returns_only_same_agent_runtime_name_collisions() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let (instances, conflicts) = runtime_and_analysis_conflict_fixture();
    for instance in &instances {
        catalog
            .upsert_skill_instance(instance)
            .expect("upsert fixture skill");
    }
    let conflict_drafts = conflicts
        .iter()
        .map(|conflict| ConflictGroupDraft {
            id: conflict.id.clone(),
            definition_id: conflict.definition_id.clone(),
            reason: conflict.reason.clone(),
            winner_id: conflict.winner_id.clone(),
            instance_ids: conflict.instance_ids.clone(),
        })
        .collect::<Vec<_>>();
    catalog
        .refresh_definitions_and_conflicts(
            &[SkillDefinitionDraft {
                id: "def.review-diff".to_string(),
                canonical_name: "review-diff".to_string(),
                description: "fixture skill".to_string(),
                active_instance: Some("claude-user-review".to_string()),
                has_multiple_instances: true,
                has_conflict: true,
            }],
            &conflict_drafts,
        )
        .expect("refresh conflicts");

    let visible_conflicts = list_conflicts(&catalog).expect("list command conflicts");

    assert_eq!(
        visible_conflicts
            .iter()
            .map(|conflict| conflict.id.as_str())
            .collect::<Vec<_>>(),
        vec!["same-agent-claude-runtime"],
        "conflict APIs expose same-agent runtime/name collisions only"
    );
}

#[test]
fn health_summary_keeps_analysis_only_rows_out_of_conflict_counts() {
    let (instances, conflicts) = runtime_and_analysis_conflict_fixture();
    let analysis = analyze_skill_instances(&instances);

    assert_eq!(analysis.summary.duplicate_name_groups, 1);
    assert_eq!(analysis.summary.path_overlap_groups, 1);
    assert_eq!(analysis.summary.enabled_mismatch_groups, 1);

    let health = build_skill_health_summary(&instances, &[], &conflicts, &analysis);

    assert_eq!(health.conflict_count, 1);
    assert_eq!(health.analysis_groups.duplicate_name_count, 1);
    assert_eq!(health.analysis_groups.path_overlap_count, 1);
    assert_eq!(health.analysis_groups.enabled_mismatch_count, 1);

    let claude = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "claude-code")
        .expect("claude health summary");
    let codex = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "codex")
        .expect("codex health summary");
    let opencode = health
        .agent_summaries
        .iter()
        .find(|summary| summary.agent == "opencode")
        .expect("opencode health summary");

    assert_eq!(claude.conflict_count, 1);
    assert_eq!(
        codex.conflict_count, 0,
        "same-agent source overlap remains analysis-only"
    );
    assert_eq!(
        opencode.conflict_count, 0,
        "cross-agent enabled-state mismatch remains analysis-only"
    );
}

#[test]
fn refresh_rule_outputs_dedupes_same_skill_rule_message_and_remediation() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let duplicate = Finding {
        instance_id: Some("skill-1".to_string()),
        definition_id: Some("def.skill-1".to_string()),
        rule_id: "body.too-long".to_string(),
        severity: Severity::Warn,
        message: "Skill body is longer than the local review threshold.".to_string(),
        suggestion: Some("Split long reference material into references/.".to_string()),
    };
    let mut second = duplicate.clone();
    second.severity = Severity::Error;
    let report = RuleReport {
        findings: vec![duplicate, second],
        definitions: Vec::new(),
        conflicts: Vec::new(),
    };

    refresh_rule_outputs(&catalog, report).expect("refresh rule outputs");

    let findings = list_findings(&catalog).expect("findings");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "body.too-long");
    assert_eq!(
        findings[0].suggestion.as_deref(),
        Some("Split long reference material into references/.")
    );
}

#[test]
fn finding_triage_commands_set_clear_and_validate_status() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .refresh_rule_findings(&[RuleFindingDraft {
            id: "finding-1".to_string(),
            instance_id: Some("skill-1".to_string()),
            definition_id: Some("def.skill-1".to_string()),
            rule_id: "body.too-long".to_string(),
            severity: "warn".to_string(),
            message: "long body".to_string(),
            suggestion: None,
            created_at: 1,
        }])
        .expect("findings refresh");
    let finding = list_findings(&catalog)
        .expect("findings")
        .pop()
        .expect("finding exists");

    let triage = set_finding_triage(
        &catalog,
        &finding.triage_key,
        "needs-follow-up",
        Some("check with owner"),
    )
    .expect("set triage");
    assert_eq!(triage.status, "needs-follow-up");
    assert_eq!(triage.note.as_deref(), Some("check with owner"));
    assert_eq!(list_finding_triage(&catalog).expect("triage list").len(), 1);

    let updated = list_findings(&catalog)
        .expect("findings after triage")
        .pop()
        .expect("finding exists");
    assert_eq!(updated.triage_status, "needs-follow-up");
    assert!(matches!(
        set_finding_triage(&catalog, &finding.triage_key, "open", None),
        Err(CommandError::InvalidFindingTriageStatus(_))
    ));
    assert!(clear_finding_triage(&catalog, &finding.triage_key).expect("clear triage"));
    let cleared = list_findings(&catalog)
        .expect("findings after clear")
        .pop()
        .expect("finding exists");
    assert_eq!(cleared.triage_status, "open");
}

fn runtime_and_analysis_conflict_fixture() -> (Vec<SkillInstance>, Vec<ConflictGroupRecord>) {
    let claude_user = health_skill(
        "claude-user-review",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let claude_project = health_skill(
        "claude-project-review",
        AgentId::ClaudeCode,
        Scope::AgentProject,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let codex_review = health_skill(
        "codex-review",
        AgentId::Codex,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
    );
    let mut codex_overlap = health_skill(
        "codex-overlap",
        AgentId::Codex,
        Scope::AgentProject,
        "review-diff",
        false,
        SkillState::Disabled,
    );
    codex_overlap.path = codex_review.path.clone();
    codex_overlap.display_path = codex_review.display_path.clone();
    let opencode_review = health_skill(
        "opencode-review",
        AgentId::Opencode,
        Scope::AgentGlobal,
        "review-diff",
        false,
        SkillState::Disabled,
    );
    let conflicts = vec![
        ConflictGroupRecord {
            id: "same-agent-claude-runtime".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "content-drift".to_string(),
            winner_id: Some("claude-user-review".to_string()),
            instance_ids: vec![
                "claude-user-review".to_string(),
                "claude-project-review".to_string(),
            ],
        },
        ConflictGroupRecord {
            id: "analysis-cross-agent-duplicate".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "cross-agent-duplicate-name".to_string(),
            winner_id: None,
            instance_ids: vec![
                "claude-user-review".to_string(),
                "codex-review".to_string(),
                "opencode-review".to_string(),
            ],
        },
        ConflictGroupRecord {
            id: "analysis-source-overlap".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "source-overlap".to_string(),
            winner_id: None,
            instance_ids: vec!["codex-review".to_string(), "codex-overlap".to_string()],
        },
        ConflictGroupRecord {
            id: "analysis-enabled-state-mismatch".to_string(),
            definition_id: "def.review-diff".to_string(),
            reason: "enabled-state-mismatch".to_string(),
            winner_id: None,
            instance_ids: vec!["codex-review".to_string(), "opencode-review".to_string()],
        },
    ];

    (
        vec![
            claude_user,
            claude_project,
            codex_review,
            codex_overlap,
            opencode_review,
        ],
        conflicts,
    )
}

fn health_skill(
    id: &str,
    agent: AgentId,
    scope: Scope,
    name: &str,
    enabled: bool,
    state: SkillState,
) -> SkillInstance {
    SkillInstance {
        id: id.to_string(),
        agent,
        scope,
        project_root: if scope == Scope::AgentProject {
            Some(PathBuf::from("/tmp/project"))
        } else {
            None
        },
        path: PathBuf::from(format!("/tmp/{}/{}/SKILL.md", agent.as_str(), id)),
        display_path: PathBuf::from(format!("/tmp/{}/{}/SKILL.md", agent.as_str(), id)),
        definition_id: format!("def.{}", canonical_skill_name_suggestion(name)),
        name: name.to_string(),
        display_name: name.to_string(),
        description: "fixture skill".to_string(),
        version: None,
        state,
        enabled,
        frontmatter_raw: format!("name: {name}\ndescription: fixture"),
        body: "fixture body".to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: format!("{id}-fingerprint"),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn health_finding(
    id: &str,
    instance_id: Option<&str>,
    definition_id: Option<&str>,
    rule_id: &str,
    severity: &str,
) -> RuleFindingRecord {
    RuleFindingRecord {
        id: id.to_string(),
        triage_key: format!("triage-{id}"),
        triage_context: "fixture-context".to_string(),
        instance_id: instance_id.map(str::to_string),
        definition_id: definition_id.map(str::to_string),
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        effective_severity: severity.to_string(),
        severity_override: None,
        message: format!("{rule_id} fixture"),
        suggestion: None,
        created_at: 0,
        suppressed: false,
        suppression_reason: None,
        suppression_note: None,
        rule_tuning_updated_at: None,
        triage_status: "open".to_string(),
        triage_note: None,
        triage_updated_at: None,
    }
}
