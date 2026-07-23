use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, AgentId, AgentReadinessRecord,
    AttentionItem, AttentionKind, AttentionSeverity, EnvironmentHealthState, EvidenceKind,
    EvidenceRef, ListIncompleteReason, NoSafeActionReason, ProjectReadinessRecord,
    ReadinessBlocker, ResumeCapability, ResumeCapabilityState, ResumeUnsupportedReason, Scope,
    SessionContinuationRecord, SkillAggregateRecord, SkillEffectivenessCount,
    SkillEffectivenessState, SkillInstanceEffectivenessRecord, SourceCoverage,
};

fn evidence(id: &str, revision: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        kind: EvidenceKind::SkillInstance,
        source_revision: revision.to_string(),
        summary: "$HOME/.agents/skills/refactor is an authorized logical source".to_string(),
        agent: Some(AgentId::Codex),
        target_id: Some("instance:refactor".to_string()),
    }
}

fn read_only_action(revision: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: "action:resume-preview".to_string(),
        kind: ActionKind::ResumeSession,
        intent: ActionIntent::ResumeSession,
        target: ActionTargetRef {
            kind: ActionTargetKind::Session,
            id: "session:1".to_string(),
            agent: Some(AgentId::Codex),
            scope: Some(Scope::AgentProject),
        },
        project_id: Some("project:funnyaccount-system".to_string()),
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "session.previewResume".to_string(),
        apply_method: None,
        source_revision: revision.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::SessionContinuation],
        evidence_refs: vec!["evidence:skill".to_string()],
    }
}

fn session(revision: &str) -> SessionContinuationRecord {
    SessionContinuationRecord {
        id: "session:1".to_string(),
        agent: AgentId::Codex,
        project_id: Some("project:funnyaccount-system".to_string()),
        title: "Continue product work".to_string(),
        intent: Some("Implement the product projection".to_string()),
        started_at: Some(1_700_000_000_000),
        ended_at: Some(1_700_000_100_000),
        modified_at: 1_700_000_100_000,
        source_kind: "codex-state-index".to_string(),
        source_revision: revision.to_string(),
        snapshot_revision: revision.to_string(),
        coverage: SourceCoverage::enumerable(1, Some(1)),
        resume: ResumeCapability::supported(vec![
            "codex".to_string(),
            "resume".to_string(),
            "native-thread-id".to_string(),
        ]),
        evidence: vec![evidence("evidence:skill", revision)],
        actions: vec![read_only_action(revision)],
    }
}

fn aggregate_with_effectiveness(
    revision: &str,
    records: Vec<SkillInstanceEffectivenessRecord>,
) -> SkillAggregateRecord {
    let evidence = records
        .iter()
        .map(|record| EvidenceRef {
            id: record.evidence_refs[0].clone(),
            kind: EvidenceKind::SkillInstance,
            source_revision: revision.to_string(),
            summary: format!("Logical evidence for {}", record.instance_id),
            agent: record.agent,
            target_id: Some(record.instance_id.clone()),
        })
        .collect::<Vec<_>>();
    let installed_instance_count = records.iter().filter(|record| record.installed).count();
    let enabled_instance_count = records
        .iter()
        .filter(|record| record.installed && record.enabled)
        .count();
    let effective_instance_count = records
        .iter()
        .filter(|record| record.state == SkillEffectivenessState::Effective)
        .count();
    let effectiveness_counts = [
        SkillEffectivenessState::Effective,
        SkillEffectivenessState::Disabled,
        SkillEffectivenessState::Shadowed,
        SkillEffectivenessState::InstalledUnlinked,
        SkillEffectivenessState::Broken,
        SkillEffectivenessState::Unavailable,
    ]
    .into_iter()
    .filter_map(|state| {
        let count = records
            .iter()
            .filter(|record| record.state == state)
            .count();
        (count > 0).then_some(SkillEffectivenessCount { state, count })
    })
    .collect();
    let coverage = SourceCoverage::merge(
        &records
            .iter()
            .map(|record| record.coverage.clone())
            .collect::<Vec<_>>(),
    )
    .expect("merge fixture coverage");

    SkillAggregateRecord {
        id: "aggregate:mixed".to_string(),
        definition_id: "definition:mixed".to_string(),
        definition_fingerprint: Some("sha256:mixed".to_string()),
        canonical_name: "mixed".to_string(),
        display_name: "Mixed".to_string(),
        description: "Mixed effectiveness states.".to_string(),
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: "native".to_string(),
        source_identity: "source:mixed".to_string(),
        runtime_identity: "native:mixed".to_string(),
        read_only_reason: None,
        instance_ids: records
            .iter()
            .map(|record| record.instance_id.clone())
            .collect(),
        agents: vec![AgentId::Codex],
        scopes: vec![Scope::AgentGlobal],
        installed_instance_count,
        enabled_instance_count,
        effective_instance_count,
        primary_effectiveness: SkillEffectivenessState::Unavailable,
        effectiveness_counts,
        instance_effectiveness: records,
        finding_count: 0,
        conflict_count: 0,
        source_revision: revision.to_string(),
        coverage,
        evidence,
        actions: Vec::new(),
    }
}

#[test]
fn product_enum_wire_values_are_stable() {
    macro_rules! assert_wire_values {
        ($($value:expr => $wire:literal),+ $(,)?) => {
            $(
                assert_eq!(
                    serde_json::to_string(&$value).expect("serialize enum wire value"),
                    concat!("\"", $wire, "\""),
                );
            )+
        };
    }

    assert_wire_values!(
        EnvironmentHealthState::Healthy => "healthy",
        EnvironmentHealthState::Review => "review",
        EnvironmentHealthState::Blocked => "blocked",
    );
    assert_wire_values!(
        SkillEffectivenessState::Effective => "effective",
        SkillEffectivenessState::Disabled => "disabled",
        SkillEffectivenessState::Shadowed => "shadowed",
        SkillEffectivenessState::InstalledUnlinked => "installed_unlinked",
        SkillEffectivenessState::Broken => "broken",
        SkillEffectivenessState::Unavailable => "unavailable",
    );
    assert_wire_values!(
        EvidenceKind::ProjectContext => "project_context",
        EvidenceKind::AdapterCapability => "adapter_capability",
        EvidenceKind::ScanCoverage => "scan_coverage",
        EvidenceKind::SkillDefinition => "skill_definition",
        EvidenceKind::SkillInstance => "skill_instance",
        EvidenceKind::Finding => "finding",
        EvidenceKind::Conflict => "conflict",
        EvidenceKind::Session => "session",
        EvidenceKind::Config => "config",
        EvidenceKind::Package => "package",
        EvidenceKind::ActionReadback => "action_readback",
    );
    assert_wire_values!(
        ActionTargetKind::Project => "project",
        ActionTargetKind::Agent => "agent",
        ActionTargetKind::Skill => "skill",
        ActionTargetKind::Session => "session",
        ActionTargetKind::Config => "config",
        ActionTargetKind::Package => "package",
    );
    assert_wire_values!(
        ActionImpact::ReadOnly => "read_only",
        ActionImpact::AppLocalData => "app_local_data",
        ActionImpact::AgentConfig => "agent_config",
        ActionImpact::SkillFiles => "skill_files",
        ActionImpact::ExternalManager => "external_manager",
    );
    assert_wire_values!(
        ActionNetworkPosture::None => "none",
        ActionNetworkPosture::Conditional => "conditional",
        ActionNetworkPosture::Required => "required",
    );
    assert_wire_values!(
        AttentionKind::IncompleteEvidence => "incomplete_evidence",
        AttentionKind::StaleEvidence => "stale_evidence",
        AttentionKind::SourceUnavailable => "source_unavailable",
        AttentionKind::Finding => "finding",
        AttentionKind::Conflict => "conflict",
        AttentionKind::BrokenSkill => "broken_skill",
        AttentionKind::SkillUnavailable => "skill_unavailable",
    );
    assert_wire_values!(
        AttentionSeverity::Critical => "critical",
        AttentionSeverity::Error => "error",
        AttentionSeverity::Warning => "warning",
        AttentionSeverity::Information => "information",
    );
    assert_wire_values!(
        ResumeCapabilityState::Supported => "supported",
        ResumeCapabilityState::Unsupported => "unsupported",
    );
    assert_wire_values!(
        ResumeUnsupportedReason::AgentUnsupported => "agent_unsupported",
        ResumeUnsupportedReason::SessionUnsupported => "session_unsupported",
        ResumeUnsupportedReason::SourceIncomplete => "source_incomplete",
        ResumeUnsupportedReason::SourceChanged => "source_changed",
        ResumeUnsupportedReason::MissingNativeId => "missing_native_id",
        ResumeUnsupportedReason::InvalidProjectContext => "invalid_project_context",
    );
    assert_wire_values!(
        AgentId::ToolGlobal => "tool-global",
        AgentId::ClaudeCode => "claude-code",
        AgentId::Codex => "codex",
        AgentId::Pi => "pi",
        AgentId::Hermes => "hermes",
        AgentId::Openclaw => "openclaw",
        AgentId::Opencode => "opencode",
    );
    assert_wire_values!(
        Scope::ToolGlobal => "tool-global",
        Scope::AgentGlobal => "agent-global",
        Scope::AgentProject => "agent-project",
    );
}

#[test]
fn evidence_validation_accepts_logical_paths_and_rejects_private_absolute_paths() {
    let valid = evidence("evidence:skill", "sha256:revision");
    assert_eq!(valid.validate(), Ok(()));

    let mut unix_private = valid.clone();
    let separator = '/';
    unix_private.summary =
        format!("{separator}Users{separator}example{separator}skills was inspected");
    assert_eq!(
        unix_private.validate(),
        Err("evidence summary contains a raw absolute path")
    );

    let mut windows_private = valid;
    let separator = '\\';
    windows_private.summary =
        format!("C:{separator}Users{separator}example{separator}skills was inspected");
    assert_eq!(
        windows_private.validate(),
        Err("evidence summary contains a raw absolute path")
    );
}

#[test]
fn logical_product_identities_reject_embedded_physical_paths() {
    let revision = "sha256:logical-identity";
    let separator = '/';
    let physical_source =
        format!("codex-plugin:{separator}Users{separator}example{separator}cache{separator}skill");
    let record = SkillInstanceEffectivenessRecord {
        instance_id: "instance:logical".to_string(),
        agent: Some(AgentId::Codex),
        scope: Scope::AgentGlobal,
        source_identity: physical_source.clone(),
        runtime_identity: "native:logical".to_string(),
        installed: true,
        linked: true,
        enabled: true,
        precedence_proven: true,
        state: SkillEffectivenessState::Effective,
        coverage: SourceCoverage::enumerable(1, Some(1)),
        evidence_refs: vec!["evidence:logical".to_string()],
        action_ids: Vec::new(),
    };
    let mut aggregate = aggregate_with_effectiveness(revision, vec![record]);
    aggregate.source_identity = physical_source;

    assert_eq!(
        aggregate.validate(),
        Err("product model identity contains a path separator")
    );
}

#[test]
fn action_validation_separates_read_only_and_mutating_contracts() {
    let revision = "sha256:revision";
    assert_eq!(read_only_action(revision).validate(), Ok(()));

    let mut mutation = read_only_action(revision);
    mutation.id = "action:disable".to_string();
    mutation.target.kind = ActionTargetKind::Skill;
    mutation.impacts = vec![ActionImpact::AgentConfig];
    mutation.preview_method = "batch.previewSkillToggles".to_string();
    mutation.apply_method = Some("batch.applySkillToggles".to_string());
    mutation.confirmation_required = true;
    assert_eq!(mutation.validate(), Ok(()));

    mutation.confirmation_required = false;
    assert_eq!(
        mutation.validate(),
        Err("mutating action requires confirmation")
    );

    let mut duplicate_impact = read_only_action(revision);
    duplicate_impact.impacts.push(ActionImpact::ReadOnly);
    assert_eq!(
        duplicate_impact.validate(),
        Err("action impacts contain a duplicate")
    );

    let mut duplicate_evidence = read_only_action(revision);
    duplicate_evidence
        .evidence_refs
        .push(duplicate_evidence.evidence_refs[0].clone());
    assert_eq!(
        duplicate_evidence.validate(),
        Err("action evidence_refs contain a duplicate")
    );
}

#[test]
fn resume_capability_requires_copy_only_consistent_payloads() {
    let supported = ResumeCapability::supported(vec![
        "codex".to_string(),
        "resume".to_string(),
        "thread-id".to_string(),
    ]);
    assert_eq!(supported.state, ResumeCapabilityState::Supported);
    assert_eq!(supported.validate(), Ok(()));

    let unsupported = ResumeCapability::unsupported(
        skills_copilot_core::ResumeUnsupportedReason::AgentUnsupported,
    );
    assert_eq!(unsupported.validate(), Ok(()));

    let mut invalid = supported;
    invalid.copy_only = false;
    assert_eq!(
        invalid.validate(),
        Err("resume capability must be copy-only")
    );
}

#[test]
fn incomplete_session_coverage_cannot_expose_supported_resume() {
    let mut continuation = session("sha256:session-incomplete");
    continuation.coverage =
        SourceCoverage::incomplete(0, Some(1), ListIncompleteReason::SourceLimited);

    assert_eq!(
        continuation.validate(),
        Err("incomplete session coverage cannot expose supported resume")
    );
}

#[test]
fn product_read_models_round_trip_and_validate_revision_bound_links() {
    let revision = "sha256:product-revision";
    let skill_evidence = evidence("evidence:skill", revision);
    let action = read_only_action(revision);
    let session = session(revision);
    let skill = SkillAggregateRecord {
        id: "aggregate:refactor".to_string(),
        definition_id: "definition:refactor".to_string(),
        definition_fingerprint: Some("sha256:definition".to_string()),
        canonical_name: "refactor".to_string(),
        display_name: "Refactor".to_string(),
        description: "Safely refactor local code.".to_string(),
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: "native".to_string(),
        source_identity: "source:refactor".to_string(),
        runtime_identity: "native:refactor".to_string(),
        read_only_reason: None,
        instance_ids: vec!["instance:refactor".to_string()],
        agents: vec![AgentId::Codex],
        scopes: vec![Scope::AgentProject],
        installed_instance_count: 1,
        enabled_instance_count: 1,
        effective_instance_count: 1,
        primary_effectiveness: SkillEffectivenessState::Effective,
        effectiveness_counts: vec![SkillEffectivenessCount {
            state: SkillEffectivenessState::Effective,
            count: 1,
        }],
        instance_effectiveness: vec![SkillInstanceEffectivenessRecord {
            instance_id: "instance:refactor".to_string(),
            agent: Some(AgentId::Codex),
            scope: Scope::AgentProject,
            source_identity: "source:refactor".to_string(),
            runtime_identity: "native:refactor".to_string(),
            installed: true,
            linked: true,
            enabled: true,
            precedence_proven: true,
            state: SkillEffectivenessState::Effective,
            coverage: SourceCoverage::enumerable(1, Some(1)),
            evidence_refs: vec![skill_evidence.id.clone()],
            action_ids: vec![action.id.clone()],
        }],
        finding_count: 0,
        conflict_count: 0,
        source_revision: revision.to_string(),
        coverage: SourceCoverage::enumerable(1, Some(1)),
        evidence: vec![skill_evidence.clone()],
        actions: vec![action.clone()],
    };
    assert_eq!(skill.validate(), Ok(()));

    let mut incorrect_complete_coverage = skill.clone();
    incorrect_complete_coverage.coverage = SourceCoverage::enumerable(999, Some(999));
    assert_eq!(
        incorrect_complete_coverage.validate(),
        Err("skill aggregate coverage does not match instance rows")
    );

    let project = ProjectReadinessRecord {
        project_id: "project:funnyaccount-system".to_string(),
        project_display_name: "funnyaccount_system".to_string(),
        source_revision: revision.to_string(),
        health: EnvironmentHealthState::Healthy,
        coverage: SourceCoverage::enumerable(1, Some(1)),
        agents: vec![AgentReadinessRecord {
            agent: AgentId::Codex,
            health: EnvironmentHealthState::Healthy,
            coverage: SourceCoverage::enumerable(1, Some(1)),
            effective_skill_count: 1,
            issue_count: 0,
            conflict_count: 0,
            evidence_refs: vec![skill_evidence.id.clone()],
            action_ids: vec![action.id.clone()],
            blocking_reasons: Vec::new(),
            attention_item_ids: Vec::new(),
        }],
        blocking_reasons: Vec::new(),
        attention: Vec::new(),
        evidence: vec![skill_evidence],
        actions: vec![action],
        recent_sessions: vec![session],
    };
    assert_eq!(project.validate(), Ok(()));

    let mut incorrect_project_coverage = project.clone();
    incorrect_project_coverage.coverage = SourceCoverage::enumerable(999, Some(999));
    assert_eq!(
        incorrect_project_coverage.validate(),
        Err("project readiness coverage does not match agent rows")
    );

    let encoded = serde_json::to_string(&project).expect("serialize product record");
    let decoded: ProjectReadinessRecord =
        serde_json::from_str(&encoded).expect("deserialize product record");
    assert_eq!(decoded, project);
    assert_eq!(decoded.validate(), Ok(()));

    let mut duplicate_actions = project.clone();
    duplicate_actions
        .actions
        .push(duplicate_actions.actions[0].clone());
    assert_eq!(
        duplicate_actions.validate(),
        Err("projection actions contain a duplicate id")
    );

    let skill_encoded = serde_json::to_string(&skill).expect("serialize skill aggregate");
    let skill_decoded: SkillAggregateRecord =
        serde_json::from_str(&skill_encoded).expect("deserialize skill aggregate");
    assert_eq!(skill_decoded, skill);
}

#[test]
fn healthy_readiness_rejects_incomplete_coverage() {
    let revision = "sha256:incomplete";
    let mut project = ProjectReadinessRecord {
        project_id: "project:1".to_string(),
        project_display_name: "Project".to_string(),
        source_revision: revision.to_string(),
        health: EnvironmentHealthState::Healthy,
        coverage: SourceCoverage::incomplete(5, Some(6), ListIncompleteReason::UnreadableSource),
        agents: Vec::new(),
        blocking_reasons: Vec::new(),
        attention: Vec::new(),
        evidence: Vec::new(),
        actions: Vec::new(),
        recent_sessions: Vec::new(),
    };
    assert_eq!(
        project.validate(),
        Err("incomplete project readiness coverage requires blocked health")
    );

    project.health = EnvironmentHealthState::Blocked;
    project.evidence.push(EvidenceRef {
        id: "evidence:coverage".to_string(),
        kind: EvidenceKind::ScanCoverage,
        source_revision: revision.to_string(),
        summary: "Codex source coverage is incomplete".to_string(),
        agent: Some(AgentId::Codex),
        target_id: Some("codex".to_string()),
    });
    project.blocking_reasons.push(ReadinessBlocker {
        id: "blocker:coverage".to_string(),
        kind: AttentionKind::IncompleteEvidence,
        summary: "Codex source coverage is incomplete".to_string(),
        agent: Some(AgentId::Codex),
        evidence_refs: vec!["evidence:coverage".to_string()],
        action_ids: Vec::new(),
    });
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn blocker_attention_and_effectiveness_invariants_are_enforced() {
    let revision = "sha256:projection";
    let evidence = EvidenceRef {
        id: "evidence:coverage".to_string(),
        kind: EvidenceKind::ScanCoverage,
        source_revision: revision.to_string(),
        summary: "Adapter inspection is incomplete".to_string(),
        agent: Some(AgentId::Codex),
        target_id: Some("codex".to_string()),
    };
    let blocker = ReadinessBlocker {
        id: "blocker:coverage".to_string(),
        kind: AttentionKind::IncompleteEvidence,
        summary: "Adapter inspection is incomplete".to_string(),
        agent: Some(AgentId::Codex),
        evidence_refs: vec![evidence.id.clone()],
        action_ids: Vec::new(),
    };
    let attention = AttentionItem {
        id: "attention:coverage".to_string(),
        kind: AttentionKind::IncompleteEvidence,
        severity: AttentionSeverity::Error,
        title: "Codex evidence is incomplete".to_string(),
        summary: "Adapter inspection is incomplete".to_string(),
        target: ActionTargetRef {
            kind: ActionTargetKind::Agent,
            id: "codex".to_string(),
            agent: Some(AgentId::Codex),
            scope: None,
        },
        agent: Some(AgentId::Codex),
        evidence_refs: vec![evidence.id.clone()],
        action_ids: Vec::new(),
        no_safe_action_reason: Some(NoSafeActionReason::IncompleteEvidence),
    };
    let project = ProjectReadinessRecord {
        project_id: "project:1".to_string(),
        project_display_name: "Project".to_string(),
        source_revision: revision.to_string(),
        health: EnvironmentHealthState::Blocked,
        coverage: SourceCoverage::unknown(ListIncompleteReason::NotInspected),
        agents: vec![AgentReadinessRecord {
            agent: AgentId::Codex,
            health: EnvironmentHealthState::Blocked,
            coverage: SourceCoverage::unknown(ListIncompleteReason::NotInspected),
            effective_skill_count: 0,
            issue_count: 1,
            conflict_count: 0,
            evidence_refs: vec![evidence.id.clone()],
            action_ids: Vec::new(),
            blocking_reasons: vec![blocker.clone()],
            attention_item_ids: vec![attention.id.clone()],
        }],
        blocking_reasons: vec![blocker],
        attention: vec![attention],
        evidence: vec![evidence],
        actions: Vec::new(),
        recent_sessions: Vec::new(),
    };
    assert_eq!(project.validate(), Ok(()));

    let unavailable = SkillInstanceEffectivenessRecord {
        instance_id: "known:skill".to_string(),
        agent: Some(AgentId::Codex),
        scope: Scope::AgentGlobal,
        source_identity: "source:known".to_string(),
        runtime_identity: "native:known".to_string(),
        installed: false,
        linked: true,
        enabled: true,
        precedence_proven: false,
        state: SkillEffectivenessState::Unavailable,
        coverage: SourceCoverage::unknown(ListIncompleteReason::NotInspected),
        evidence_refs: vec!["evidence:known".to_string()],
        action_ids: Vec::new(),
    };
    let known = std::collections::HashSet::from(["evidence:known"]);
    assert_eq!(
        unavailable.validate(&known, &std::collections::HashSet::new()),
        Ok(())
    );

    let mut contradictory_unavailable = unavailable;
    contradictory_unavailable.installed = true;
    contradictory_unavailable.linked = false;
    contradictory_unavailable.coverage = SourceCoverage::enumerable(1, Some(1));
    assert_eq!(
        contradictory_unavailable.validate(&known, &std::collections::HashSet::new()),
        Err("unavailable installed skill requires linked enabled unproved precedence")
    );
}

#[test]
fn aggregate_effectiveness_counts_cover_every_projected_instance() {
    let revision = "sha256:mixed-effectiveness";
    let make_record =
        |id: &str, installed: bool, precedence_proven: bool, coverage: SourceCoverage| {
            SkillInstanceEffectivenessRecord {
                instance_id: id.to_string(),
                agent: Some(AgentId::Codex),
                scope: Scope::AgentGlobal,
                source_identity: "source:mixed".to_string(),
                runtime_identity: "native:mixed".to_string(),
                installed,
                linked: true,
                enabled: true,
                precedence_proven,
                state: if precedence_proven {
                    SkillEffectivenessState::Effective
                } else {
                    SkillEffectivenessState::Unavailable
                },
                coverage,
                evidence_refs: vec![format!("evidence:{id}")],
                action_ids: Vec::new(),
            }
        };
    let records = vec![
        make_record(
            "effective",
            true,
            true,
            SourceCoverage::enumerable(1, Some(1)),
        ),
        make_record(
            "installed-unproved",
            true,
            false,
            SourceCoverage::enumerable(1, Some(1)),
        ),
        make_record(
            "source-incomplete",
            false,
            false,
            SourceCoverage::unknown(ListIncompleteReason::NotInspected),
        ),
    ];

    let aggregate = aggregate_with_effectiveness(revision, records);
    assert_eq!(aggregate.validate(), Ok(()));

    let mut duplicate_state = aggregate.clone();
    duplicate_state
        .effectiveness_counts
        .push(SkillEffectivenessCount {
            state: SkillEffectivenessState::Effective,
            count: 0,
        });
    assert_eq!(
        duplicate_state.validate(),
        Err("skill aggregate effectiveness counts contain a duplicate state")
    );

    let mut mismatched_state_count = aggregate.clone();
    mismatched_state_count.effectiveness_counts = vec![
        SkillEffectivenessCount {
            state: SkillEffectivenessState::Effective,
            count: 2,
        },
        SkillEffectivenessCount {
            state: SkillEffectivenessState::Unavailable,
            count: 1,
        },
    ];
    assert_eq!(
        mismatched_state_count.validate(),
        Err("skill aggregate effectiveness count does not match instance rows")
    );

    let mut mismatched_primary = aggregate.clone();
    mismatched_primary.primary_effectiveness = SkillEffectivenessState::Effective;
    assert_eq!(
        mismatched_primary.validate(),
        Err("skill aggregate primary effectiveness does not match instance rows")
    );

    let mut mismatched_agents = aggregate.clone();
    mismatched_agents.agents = vec![AgentId::ClaudeCode];
    assert_eq!(
        mismatched_agents.validate(),
        Err("skill aggregate agents do not match instance rows")
    );

    let mut mismatched_scopes = aggregate.clone();
    mismatched_scopes.scopes = vec![Scope::AgentProject];
    assert_eq!(
        mismatched_scopes.validate(),
        Err("skill aggregate scopes do not match instance rows")
    );

    let mut mismatched_coverage = aggregate;
    mismatched_coverage.coverage = SourceCoverage::enumerable(3, Some(3));
    assert_eq!(
        mismatched_coverage.validate(),
        Err("skill aggregate coverage does not match instance rows")
    );
}
