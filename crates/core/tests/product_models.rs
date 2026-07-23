use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionNetworkPosture, ActionTargetKind, ActionTargetRef,
    AgentId, AgentReadinessRecord, EnvironmentHealthState, EvidenceKind, EvidenceRef,
    ListIncompleteReason, ProjectReadinessRecord, ResumeCapability, ResumeCapabilityState, Scope,
    SessionContinuationRecord, SkillAggregateRecord, SkillEffectivenessCount,
    SkillEffectivenessState, SourceCoverage,
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
        target: ActionTargetRef {
            kind: ActionTargetKind::Session,
            id: "session:1".to_string(),
            agent: Some(AgentId::Codex),
            scope: Some(Scope::AgentProject),
        },
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "session.previewResume".to_string(),
        apply_method: None,
        source_revision: revision.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
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

#[test]
fn product_enum_wire_values_are_stable() {
    let cases = [
        (
            serde_json::to_string(&EnvironmentHealthState::Healthy).unwrap(),
            "\"healthy\"",
        ),
        (
            serde_json::to_string(&SkillEffectivenessState::InstalledUnlinked).unwrap(),
            "\"installed_unlinked\"",
        ),
        (
            serde_json::to_string(&EvidenceKind::ScanCoverage).unwrap(),
            "\"scan_coverage\"",
        ),
        (
            serde_json::to_string(&AgentId::ClaudeCode).unwrap(),
            "\"claude-code\"",
        ),
        (
            serde_json::to_string(&Scope::AgentProject).unwrap(),
            "\"agent-project\"",
        ),
    ];

    for (actual, expected) in cases {
        assert_eq!(actual, expected);
    }
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
fn product_read_models_round_trip_and_validate_revision_bound_links() {
    let revision = "sha256:product-revision";
    let skill_evidence = evidence("evidence:skill", revision);
    let action = read_only_action(revision);
    let session = session(revision);
    let skill = SkillAggregateRecord {
        id: "aggregate:refactor".to_string(),
        definition_id: "definition:refactor".to_string(),
        canonical_name: "refactor".to_string(),
        display_name: "Refactor".to_string(),
        description: "Safely refactor local code.".to_string(),
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: "native".to_string(),
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
        finding_count: 0,
        conflict_count: 0,
        source_revision: revision.to_string(),
        coverage: SourceCoverage::enumerable(1, Some(1)),
        evidence: vec![skill_evidence.clone()],
        actions: vec![action.clone()],
    };
    assert_eq!(skill.validate(), Ok(()));

    let project = ProjectReadinessRecord {
        project_id: "project:funnyaccount-system".to_string(),
        project_display_name: "funnyaccount_system".to_string(),
        source_revision: revision.to_string(),
        health: EnvironmentHealthState::Healthy,
        coverage: SourceCoverage::enumerable(6, Some(6)),
        agents: vec![AgentReadinessRecord {
            agent: AgentId::Codex,
            health: EnvironmentHealthState::Healthy,
            coverage: SourceCoverage::enumerable(1, Some(1)),
            effective_skill_count: 1,
            issue_count: 0,
            conflict_count: 0,
            evidence_refs: vec![skill_evidence.id.clone()],
            action_ids: vec![action.id.clone()],
        }],
        evidence: vec![skill_evidence],
        actions: vec![action],
        recent_sessions: vec![session],
    };
    assert_eq!(project.validate(), Ok(()));

    let encoded = serde_json::to_string(&project).expect("serialize product record");
    let decoded: ProjectReadinessRecord =
        serde_json::from_str(&encoded).expect("deserialize product record");
    assert_eq!(decoded, project);
    assert_eq!(decoded.validate(), Ok(()));

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
        evidence: Vec::new(),
        actions: Vec::new(),
        recent_sessions: Vec::new(),
    };
    assert_eq!(
        project.validate(),
        Err("healthy project readiness requires complete coverage")
    );

    project.health = EnvironmentHealthState::Blocked;
    assert_eq!(project.validate(), Ok(()));
}
