use super::*;
use skills_copilot_catalog::{ConflictGroupRecord, RuleFindingRecord};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, AttentionKind, EnvironmentHealthState,
    ListIncompleteReason, ResumeCapability, ResumeUnsupportedReason, SkillEffectivenessState,
    SourceCoverage,
};

const REVISION: &str = "revision:v1";
const PROJECT_ID: &str = "project:funnyaccount-system";

fn registry_action(mut action: ActionDescriptor) -> ActionDescriptor {
    action.id = deterministic_action_id(
        action.kind,
        action.intent,
        &action.target,
        action.project_id.as_deref(),
    )
    .expect("registry action id");
    action
}

#[test]
fn six_agent_projection_preserves_plugin_and_compatibility_provenance() {
    let facts = vec![
        skill_fact(
            "claude-review",
            AgentId::ClaudeCode,
            "review",
            "source:claude-review",
            "native",
            "native:review",
        ),
        plugin_skill_fact(),
        opencode_compatibility_fact(),
        skill_fact(
            "pi-plan",
            AgentId::Pi,
            "plan",
            "source:pi-plan",
            "native",
            "native:plan",
        ),
        skill_fact(
            "hermes-audit",
            AgentId::Hermes,
            "audit",
            "source:hermes-audit",
            "native",
            "native:audit",
        ),
        skill_fact(
            "openclaw-browser",
            AgentId::Openclaw,
            "browser",
            "source:openclaw-browser",
            "native",
            "native:browser",
        ),
    ];

    let projection =
        project(base_input(), facts, Vec::new(), Vec::new()).expect("six-agent product projection");

    assert_eq!(projection.readiness.health, EnvironmentHealthState::Healthy);
    assert_eq!(projection.readiness.agents.len(), 6);
    assert!(projection
        .readiness
        .agents
        .iter()
        .all(|agent| agent.health == EnvironmentHealthState::Healthy));
    assert_eq!(
        projection
            .readiness
            .agents
            .iter()
            .map(|agent| agent.effective_skill_count)
            .sum::<usize>(),
        6
    );

    let plugin = projection
        .skill_aggregates
        .iter()
        .find(|record| record.source_kind == "chatgpt-plugin-cache")
        .expect("Codex plugin aggregate");
    assert_eq!(plugin.agents, vec![AgentId::Codex]);
    assert_eq!(plugin.package_name.as_deref(), Some("product-design"));
    assert_eq!(plugin.package_version.as_deref(), Some("0.1.52"));
    assert_eq!(
        plugin.source_identity,
        "codex-plugin:openai-curated-remote:product-design:0.1.52"
    );
    assert_eq!(
        plugin.runtime_identity,
        "plugin:product-design@openai-curated-remote:audit"
    );

    let compatibility = projection
        .skill_aggregates
        .iter()
        .find(|record| record.source_kind == "opencode-compatibility")
        .expect("opencode compatibility aggregate");
    assert_eq!(compatibility.agents, vec![AgentId::Opencode]);
    assert_eq!(
        compatibility.source_identity,
        "compatibility:opencode:claude:user:seedream"
    );
    assert_eq!(
        compatibility.primary_effectiveness,
        SkillEffectivenessState::Effective
    );

    let audit_rows = projection
        .skill_aggregates
        .iter()
        .filter(|record| record.canonical_name == "audit")
        .collect::<Vec<_>>();
    assert_eq!(
        audit_rows.len(),
        2,
        "same-name native and plugin sources must remain distinct"
    );
}

#[test]
fn effectiveness_states_are_orthogonal_and_unproved_precedence_is_unavailable() {
    let mut effective = skill_fact(
        "effective",
        AgentId::Codex,
        "effective",
        "source:effective",
        "native",
        "native:effective",
    );
    effective.definition_id = "definition:effective".to_string();

    let mut disabled = skill_fact(
        "disabled",
        AgentId::Codex,
        "disabled",
        "source:disabled",
        "native",
        "native:disabled",
    );
    disabled.enabled = false;
    disabled.adapter_state = SkillState::Disabled;

    let mut shadowed = skill_fact(
        "shadowed",
        AgentId::Codex,
        "shadowed",
        "source:shadowed",
        "native",
        "native:shadowed",
    );
    shadowed.adapter_state = SkillState::Shadowed;

    let mut unlinked = skill_fact(
        "unlinked",
        AgentId::Codex,
        "unlinked",
        "source:unlinked",
        "manager",
        "native:unlinked",
    );
    unlinked.linked = false;

    let mut broken = skill_fact(
        "broken",
        AgentId::Codex,
        "broken",
        "source:broken",
        "native",
        "native:broken",
    );
    broken.adapter_state = SkillState::Broken;

    let mut unproved = skill_fact(
        "unproved",
        AgentId::Codex,
        "unproved",
        "source:unproved",
        "native",
        "native:unproved",
    );
    unproved.precedence_proven = false;

    let mut incomplete = skill_fact(
        "incomplete",
        AgentId::Codex,
        "incomplete",
        "source:incomplete",
        "native",
        "native:incomplete",
    );
    incomplete.coverage = SourceCoverage::unknown(ListIncompleteReason::NotInspected);

    let projection = project(
        base_input(),
        vec![
            effective, disabled, shadowed, unlinked, broken, unproved, incomplete,
        ],
        Vec::new(),
        Vec::new(),
    )
    .expect("effectiveness projection");
    let states = projection
        .skill_aggregates
        .iter()
        .map(|record| (record.canonical_name.as_str(), record.primary_effectiveness))
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(states["effective"], SkillEffectivenessState::Effective);
    assert_eq!(states["disabled"], SkillEffectivenessState::Disabled);
    assert_eq!(states["shadowed"], SkillEffectivenessState::Shadowed);
    assert_eq!(
        states["unlinked"],
        SkillEffectivenessState::InstalledUnlinked
    );
    assert_eq!(states["broken"], SkillEffectivenessState::Broken);
    assert_eq!(states["unproved"], SkillEffectivenessState::Unavailable);
    assert_eq!(states["incomplete"], SkillEffectivenessState::Unavailable);

    let incomplete = projection
        .skill_aggregates
        .iter()
        .find(|record| record.canonical_name == "incomplete")
        .expect("incomplete aggregate");
    assert_eq!(incomplete.installed_instance_count, 0);
    assert_eq!(incomplete.enabled_instance_count, 0);
    assert_eq!(incomplete.effective_instance_count, 0);
}

#[test]
fn material_identity_dimensions_prevent_same_name_merges() {
    let base = skill_fact(
        "base",
        AgentId::ClaudeCode,
        "same-name",
        "source:one",
        "native",
        "native:same-name",
    );
    let mut changed_content = base.clone();
    changed_content.instance_id = "changed-content".to_string();
    changed_content.definition_fingerprint = Some("fingerprint:two".to_string());

    let mut changed_source = base.clone();
    changed_source.instance_id = "changed-source".to_string();
    changed_source.source_identity = "source:two".to_string();

    let mut changed_runtime = base.clone();
    changed_runtime.instance_id = "changed-runtime".to_string();
    changed_runtime.runtime_identity = "plugin:one:same-name".to_string();

    let mut changed_scope = base.clone();
    changed_scope.instance_id = "changed-scope".to_string();
    changed_scope.scope = Scope::AgentProject;

    let projection = project(
        base_input(),
        vec![
            base,
            changed_content,
            changed_source,
            changed_runtime,
            changed_scope,
        ],
        Vec::new(),
        Vec::new(),
    )
    .expect("identity projection");

    assert_eq!(
        projection.skill_aggregates.len(),
        5,
        "content, source, runtime, and scope identity must all be grouping dimensions"
    );
}

#[test]
fn retained_precedence_loser_projects_as_shadowed_only_with_a_proven_winner() {
    let winner = skill_fact(
        "precedence-winner",
        AgentId::Pi,
        "ordered-skill",
        "source:project-winner",
        "native",
        "native:ordered-skill",
    );
    let loser = skill_fact(
        "precedence-loser",
        AgentId::Pi,
        "ordered-skill",
        "source:global-loser",
        "native",
        "native:ordered-skill",
    );
    let conflict = ConflictGroupRecord {
        id: "conflict:precedence".to_string(),
        definition_id: winner.definition_id.clone(),
        reason: "name-collision".to_string(),
        winner_id: Some(winner.instance_id.clone()),
        instance_ids: vec![winner.instance_id.clone(), loser.instance_id.clone()],
    };

    let projection = project(
        base_input(),
        vec![winner, loser],
        Vec::new(),
        vec![conflict],
    )
    .expect("proven precedence projection");
    let states = projection
        .skill_aggregates
        .iter()
        .flat_map(|aggregate| aggregate.instance_effectiveness.iter())
        .map(|record| (record.instance_id.as_str(), record.state))
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(
        states["precedence-winner"],
        SkillEffectivenessState::Effective
    );
    assert_eq!(
        states["precedence-loser"],
        SkillEffectivenessState::Shadowed
    );
    assert_eq!(projection.readiness.health, EnvironmentHealthState::Review);
    assert!(projection.readiness.blocking_reasons.is_empty());
}

#[test]
fn optional_missing_roots_do_not_degrade_scan_coverage_but_missing_scan_evidence_blocks() {
    let report = AgentCatalogScanReport {
        agent: AgentId::ClaudeCode,
        display_name: "Claude Code",
        scanned_count: 1,
        roots_considered: vec![
            PathBuf::from("/optional/not-created"),
            PathBuf::from("/observed"),
        ],
        scanned_roots: vec![PathBuf::from("/observed")],
        partial_roots: Vec::new(),
        skipped_roots: Vec::new(),
        issues: Vec::new(),
        root_aliases: Vec::new(),
        budget_exhausted: false,
    };
    let coverage = source_coverage_from_scan_report(&report);
    assert!(coverage.is_complete());
    assert_eq!(coverage.inspected_sources, 1);
    assert_eq!(coverage.expected_sources, Some(1));

    let mut input = base_input();
    input
        .agent_sources
        .retain(|source| source.agent != AgentId::Openclaw);
    let projection =
        project(input, Vec::new(), Vec::new(), Vec::new()).expect("missing scan projection");
    let openclaw = projection
        .readiness
        .agents
        .iter()
        .find(|record| record.agent == AgentId::Openclaw)
        .expect("synthesized OpenClaw readiness");

    assert_eq!(projection.readiness.health, EnvironmentHealthState::Blocked);
    assert_eq!(openclaw.health, EnvironmentHealthState::Blocked);
    assert_eq!(
        openclaw.coverage.incomplete_reason,
        Some(ListIncompleteReason::NotInspected)
    );
    assert!(!openclaw.blocking_reasons.is_empty());
}

#[test]
fn active_findings_conflicts_and_existing_actions_form_one_attention_queue() {
    let mut first = skill_fact(
        "conflict-one",
        AgentId::Codex,
        "conflict",
        "source:one",
        "native",
        "native:conflict",
    );
    let second = skill_fact(
        "conflict-two",
        AgentId::Codex,
        "conflict",
        "source:two",
        "native",
        "native:conflict",
    );
    let action = registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::ToggleSkill,
        intent: ActionIntent::DisableSkill,
        target: ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: first.instance_id.clone(),
            agent: first.agent,
            scope: Some(first.scope),
        },
        project_id: None,
        impacts: vec![ActionImpact::AgentConfig],
        preview_method: "batch.previewSkillToggles".to_string(),
        apply_method: Some("batch.applySkillToggles".to_string()),
        source_revision: REVISION.to_string(),
        confirmation_required: true,
        network: ActionNetworkPosture::None,
        readback: vec![
            ActionReadbackDomain::AgentConfig,
            ActionReadbackDomain::SkillAggregates,
        ],
        evidence_refs: vec![skill_projection_evidence_id(&first.instance_id)],
    });
    first.action_ids = vec![action.id.clone()];
    let mut input = base_input();
    input.actions.push(action);
    let active = finding("active", &first.instance_id, "warning", false, "open");
    let suppressed = finding("suppressed", &first.instance_id, "error", true, "open");
    let reviewed = finding("reviewed", &first.instance_id, "error", false, "reviewed");
    let conflict = ConflictGroupRecord {
        id: "conflict:runtime".to_string(),
        definition_id: first.definition_id.clone(),
        reason: "content-drift".to_string(),
        winner_id: None,
        instance_ids: vec![first.instance_id.clone(), second.instance_id.clone()],
    };

    let projection = project(
        input,
        vec![first, second],
        vec![active, suppressed, reviewed],
        vec![conflict],
    )
    .expect("attention projection");

    assert_eq!(projection.readiness.health, EnvironmentHealthState::Blocked);
    assert_eq!(projection.readiness.actions.len(), 1);
    assert!(projection
        .readiness
        .attention
        .iter()
        .any(|item| item.kind == AttentionKind::Finding));
    assert!(projection
        .readiness
        .attention
        .iter()
        .any(|item| item.kind == AttentionKind::Conflict));
    assert_eq!(
        projection
            .readiness
            .attention
            .iter()
            .filter(|item| item.kind == AttentionKind::Finding)
            .count(),
        1,
        "suppressed and reviewed findings must reuse the current visibility policy"
    );
    assert!(projection.readiness.blocking_reasons.iter().any(|blocker| {
        blocker.kind == AttentionKind::Conflict
            && blocker
                .evidence_refs
                .contains(&conflict_projection_evidence_id("conflict:runtime"))
    }));
}

#[test]
fn session_continuations_require_exact_project_match_and_preserve_adapter_capability() {
    let agents = [
        AgentId::ClaudeCode,
        AgentId::Codex,
        AgentId::Opencode,
        AgentId::Pi,
        AgentId::Hermes,
        AgentId::Openclaw,
    ];
    let mut sessions = agents
        .into_iter()
        .enumerate()
        .map(|(index, agent)| SessionProjectionInput {
            id: format!("session:{}", agent.as_str()),
            agent,
            project_id: Some(PROJECT_ID.to_string()),
            title: format!("{} session", agent.as_str()),
            intent: Some("Continue verified local work".to_string()),
            started_at: Some(100 + index as i64),
            ended_at: None,
            modified_at: 200 + index as i64,
            source_kind: format!("{}-native-session", agent.as_str()),
            source_revision: format!("session-revision:{}", agent.as_str()),
            snapshot_revision: REVISION.to_string(),
            coverage: SourceCoverage::enumerable(1, Some(1)),
            resume: ResumeCapability::supported(vec![
                format!("{}-native", agent.as_str()),
                "resume".to_string(),
                format!("native-id-{index}"),
            ]),
            evidence_summary: format!("{} native session source was inspected", agent.as_str()),
            action_ids: Vec::new(),
        })
        .collect::<Vec<_>>();
    sessions.push(SessionProjectionInput {
        id: "session:other-project".to_string(),
        agent: AgentId::Codex,
        project_id: Some("project:other".to_string()),
        title: "Other project".to_string(),
        intent: None,
        started_at: None,
        ended_at: None,
        modified_at: 500,
        source_kind: "codex-native-session".to_string(),
        source_revision: "session-revision:other".to_string(),
        snapshot_revision: REVISION.to_string(),
        coverage: SourceCoverage::enumerable(1, Some(1)),
        resume: ResumeCapability::unsupported(ResumeUnsupportedReason::SessionUnsupported),
        evidence_summary: "Codex native session source was inspected".to_string(),
        action_ids: Vec::new(),
    });
    sessions.push(SessionProjectionInput {
        id: "session:incomplete".to_string(),
        agent: AgentId::Codex,
        project_id: Some(PROJECT_ID.to_string()),
        title: "Incomplete source".to_string(),
        intent: None,
        started_at: None,
        ended_at: None,
        modified_at: 1,
        source_kind: "codex-native-session".to_string(),
        source_revision: "session-revision:incomplete".to_string(),
        snapshot_revision: REVISION.to_string(),
        coverage: SourceCoverage::incomplete(0, Some(1), ListIncompleteReason::SourceLimited),
        resume: ResumeCapability::supported(vec![
            "must-not-be-returned".to_string(),
            "resume".to_string(),
        ]),
        evidence_summary: "Codex session source is source-limited".to_string(),
        action_ids: Vec::new(),
    });
    let mut input = base_input();
    input.sessions = sessions;

    let projection = project(input, Vec::new(), Vec::new(), Vec::new())
        .expect("session continuation projection");

    assert_eq!(projection.session_continuations.len(), 7);
    assert!(projection
        .session_continuations
        .iter()
        .all(|record| record.project_id.as_deref() == Some(PROJECT_ID)));
    for agent in agents {
        let record = projection
            .session_continuations
            .iter()
            .find(|record| record.agent == agent && record.id != "session:incomplete")
            .expect("agent continuation record");
        assert_eq!(record.resume.argv[0], format!("{}-native", agent.as_str()));
        assert!(record.resume.copy_only);
    }
    let incomplete = projection
        .session_continuations
        .iter()
        .find(|record| record.id == "session:incomplete")
        .expect("incomplete continuation");
    assert!(incomplete.resume.argv.is_empty());
    assert_eq!(
        incomplete.resume.unsupported_reason,
        Some(ResumeUnsupportedReason::SourceIncomplete)
    );
}

#[test]
fn session_actions_bind_the_product_snapshot_and_native_session_evidence() {
    let mut session = session_input(AgentId::Codex, "action-session", 10);
    let action = registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::ResumeSession,
        intent: ActionIntent::ResumeSession,
        target: ActionTargetRef {
            kind: ActionTargetKind::Session,
            id: session.id.clone(),
            agent: Some(session.agent),
            scope: Some(Scope::AgentProject),
        },
        project_id: Some(PROJECT_ID.to_string()),
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "session.previewResume".to_string(),
        apply_method: None,
        source_revision: REVISION.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::SessionContinuation],
        evidence_refs: vec![session_projection_evidence_id(
            &session.id,
            &session.source_revision,
        )],
    });
    session.action_ids = vec![action.id.clone()];
    let native_revision = session.source_revision.clone();
    let mut input = base_input();
    input.sessions = vec![session];
    input.actions = vec![action];

    let projection =
        project(input, Vec::new(), Vec::new(), Vec::new()).expect("session action projection");
    let continuation = &projection.session_continuations[0];

    assert_eq!(continuation.source_revision, native_revision);
    assert_eq!(continuation.snapshot_revision, REVISION);
    assert_eq!(continuation.actions.len(), 1);
    assert_eq!(continuation.actions[0].source_revision, REVISION);
    assert_eq!(
        continuation.actions[0].evidence_refs,
        vec![continuation.evidence[0].id.clone()]
    );
}

#[test]
fn action_ownership_must_match_the_declaring_skill_agent_or_session() {
    let first = skill_fact(
        "owner-first",
        AgentId::Codex,
        "owner",
        "source:owner",
        "native",
        "native:owner",
    );
    let mut second = first.clone();
    second.instance_id = "owner-second".to_string();
    let unrelated_skill_action = registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::RefreshEvidence,
        intent: ActionIntent::InspectEvidence,
        target: ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: second.instance_id.clone(),
            agent: second.agent,
            scope: Some(second.scope),
        },
        project_id: None,
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "catalog.getSkill".to_string(),
        apply_method: None,
        source_revision: REVISION.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::CatalogSkills],
        evidence_refs: vec![skill_projection_evidence_id(&second.instance_id)],
    });
    let mut wrong_skill_owner = first.clone();
    wrong_skill_owner.action_ids = vec![unrelated_skill_action.id.clone()];
    let mut input = base_input();
    input.actions = vec![unrelated_skill_action];
    let error = project(
        input,
        vec![wrong_skill_owner, second],
        Vec::new(),
        Vec::new(),
    )
    .expect_err("cross-instance action ownership must fail closed");
    assert!(error.to_string().contains("owning skill"));

    let mut input = base_input();
    let claude = input
        .agent_sources
        .iter_mut()
        .find(|source| source.agent == AgentId::ClaudeCode)
        .expect("Claude source");
    let wrong_agent_action = registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::RefreshEvidence,
        intent: ActionIntent::InspectEvidence,
        target: ActionTargetRef {
            kind: ActionTargetKind::Agent,
            id: AgentId::Codex.as_str().to_string(),
            agent: Some(AgentId::Codex),
            scope: None,
        },
        project_id: None,
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "catalog.scanAgent".to_string(),
        apply_method: None,
        source_revision: REVISION.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::CatalogSkills],
        evidence_refs: vec![coverage_projection_evidence_id(AgentId::ClaudeCode)],
    });
    claude.action_ids = vec![wrong_agent_action.id.clone()];
    input.actions = vec![wrong_agent_action];
    let error = project(input, Vec::new(), Vec::new(), Vec::new())
        .expect_err("cross-agent action ownership must fail closed");
    assert!(error.to_string().contains("owning agent"));

    let mut session = session_input(AgentId::Codex, "owner-session", 10);
    let mut input = base_input();
    let wrong_session_action = registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::ResumeSession,
        intent: ActionIntent::ResumeSession,
        target: ActionTargetRef {
            kind: ActionTargetKind::Session,
            id: "another-session".to_string(),
            agent: Some(AgentId::Codex),
            scope: Some(Scope::AgentProject),
        },
        project_id: Some(PROJECT_ID.to_string()),
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "session.previewResume".to_string(),
        apply_method: None,
        source_revision: REVISION.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::SessionContinuation],
        evidence_refs: vec![session_projection_evidence_id(
            &session.id,
            &session.source_revision,
        )],
    });
    session.action_ids = vec![wrong_session_action.id.clone()];
    input.actions = vec![wrong_session_action];
    input.sessions = vec![session];
    let error = project(input, Vec::new(), Vec::new(), Vec::new())
        .expect_err("cross-session action ownership must fail closed");
    assert!(error.to_string().contains("owning session"));
}

#[test]
fn conflict_winner_must_be_current_and_proven_before_losers_are_shadowed() {
    let first = skill_fact(
        "conflict-current-a",
        AgentId::Pi,
        "conflict-current",
        "source:conflict-current-a",
        "native",
        "native:conflict-current",
    );
    let second = skill_fact(
        "conflict-current-b",
        AgentId::Pi,
        "conflict-current",
        "source:conflict-current-b",
        "native",
        "native:conflict-current",
    );
    let absent_winner = ConflictGroupRecord {
        id: "conflict:absent-winner".to_string(),
        definition_id: first.definition_id.clone(),
        reason: "name-collision".to_string(),
        winner_id: Some("conflict-not-current".to_string()),
        instance_ids: vec![first.instance_id.clone(), second.instance_id.clone()],
    };
    let error = project(
        base_input(),
        vec![first.clone(), second.clone()],
        Vec::new(),
        vec![absent_winner],
    )
    .expect_err("absent conflict winner must fail closed");
    assert!(error.to_string().contains("winner"));

    let mut mismatched_runtime = second.clone();
    mismatched_runtime.runtime_identity = "native:another-runtime".to_string();
    let cross_runtime = ConflictGroupRecord {
        id: "conflict:cross-runtime".to_string(),
        definition_id: first.definition_id.clone(),
        reason: "name-collision".to_string(),
        winner_id: Some(first.instance_id.clone()),
        instance_ids: vec![
            first.instance_id.clone(),
            mismatched_runtime.instance_id.clone(),
        ],
    };
    let error = project(
        base_input(),
        vec![first.clone(), mismatched_runtime],
        Vec::new(),
        vec![cross_runtime],
    )
    .expect_err("cross-runtime conflict must fail closed");
    assert!(error.to_string().contains("runtime identity"));

    let mut historical_winner = first.clone();
    historical_winner.adapter_state = SkillState::Missing;
    let historical_conflict = ConflictGroupRecord {
        id: "conflict:historical-winner".to_string(),
        definition_id: historical_winner.definition_id.clone(),
        reason: "name-collision".to_string(),
        winner_id: Some(historical_winner.instance_id.clone()),
        instance_ids: vec![
            historical_winner.instance_id.clone(),
            second.instance_id.clone(),
        ],
    };
    let error = project(
        base_input(),
        vec![historical_winner, second.clone()],
        Vec::new(),
        vec![historical_conflict],
    )
    .expect_err("historical conflict winner must fail closed");
    assert!(error.to_string().contains("non-current"));

    let mut unproved_winner = first;
    unproved_winner.precedence_proven = false;
    let unresolved = ConflictGroupRecord {
        id: "conflict:unproved-winner".to_string(),
        definition_id: unproved_winner.definition_id.clone(),
        reason: "name-collision".to_string(),
        winner_id: Some(unproved_winner.instance_id.clone()),
        instance_ids: vec![
            unproved_winner.instance_id.clone(),
            second.instance_id.clone(),
        ],
    };
    let projection = project(
        base_input(),
        vec![unproved_winner, second],
        Vec::new(),
        vec![unresolved],
    )
    .expect("unproved current winner becomes unresolved");

    assert_eq!(projection.readiness.health, EnvironmentHealthState::Blocked);
    assert!(projection
        .skill_aggregates
        .iter()
        .flat_map(|aggregate| &aggregate.instance_effectiveness)
        .all(|record| record.state == SkillEffectivenessState::Unavailable));
    assert!(projection
        .readiness
        .attention
        .iter()
        .any(|item| item.kind == AttentionKind::Conflict
            && item.severity == skills_copilot_core::AttentionSeverity::Error));
}

#[test]
fn accepted_snapshot_rejects_mixed_component_revisions() {
    let mut skill = skill_fact(
        "mixed-revision",
        AgentId::Codex,
        "mixed-revision",
        "source:mixed-revision",
        "native",
        "native:mixed-revision",
    );
    skill.source_revision = "revision:other".to_string();

    let error = project(base_input(), vec![skill], Vec::new(), Vec::new())
        .expect_err("mixed source revisions must fail closed");

    assert!(error.to_string().contains("source revision"));
}

#[test]
fn projection_serialization_is_stable_when_snapshot_inputs_are_reordered() {
    let mut stable_skill = skill_fact(
        "stable-claude",
        AgentId::ClaudeCode,
        "stable",
        "source:stable-claude",
        "native",
        "native:stable",
    );
    let mut skills = vec![
        plugin_skill_fact(),
        opencode_compatibility_fact(),
        stable_skill.clone(),
    ];
    let findings = vec![
        finding("stable-a", "stable-claude", "warning", false, "open"),
        finding(
            "stable-b",
            "codex-plugin-audit",
            "information",
            false,
            "open",
        ),
    ];
    let skill_evidence = skill_projection_evidence_id(&stable_skill.instance_id);
    let finding_evidence = finding_projection_evidence_id("stable-a");
    let mut actions = ["a", "b"]
        .into_iter()
        .map(|suffix| {
            registry_action(ActionDescriptor {
                id: String::new(),
                kind: ActionKind::ToggleSkill,
                intent: if suffix == "a" {
                    ActionIntent::DisableSkill
                } else {
                    ActionIntent::EnableSkill
                },
                target: ActionTargetRef {
                    kind: ActionTargetKind::Skill,
                    id: stable_skill.instance_id.clone(),
                    agent: stable_skill.agent,
                    scope: Some(stable_skill.scope),
                },
                project_id: None,
                impacts: vec![ActionImpact::SkillFiles, ActionImpact::AgentConfig],
                preview_method: "batch.previewSkillToggles".to_string(),
                apply_method: Some("batch.applySkillToggles".to_string()),
                source_revision: REVISION.to_string(),
                confirmation_required: true,
                network: ActionNetworkPosture::None,
                readback: vec![
                    ActionReadbackDomain::AgentConfig,
                    ActionReadbackDomain::SkillAggregates,
                ],
                evidence_refs: vec![finding_evidence.clone(), skill_evidence.clone()],
            })
        })
        .collect::<Vec<_>>();
    actions.push(registry_action(ActionDescriptor {
        id: String::new(),
        kind: ActionKind::RefreshEvidence,
        intent: ActionIntent::InspectEvidence,
        target: ActionTargetRef {
            kind: ActionTargetKind::Agent,
            id: AgentId::ClaudeCode.as_str().to_string(),
            agent: Some(AgentId::ClaudeCode),
            scope: None,
        },
        project_id: None,
        impacts: vec![ActionImpact::ReadOnly],
        preview_method: "catalog.scanAgent".to_string(),
        apply_method: None,
        source_revision: REVISION.to_string(),
        confirmation_required: false,
        network: ActionNetworkPosture::None,
        readback: vec![ActionReadbackDomain::CatalogSkills],
        evidence_refs: vec![coverage_projection_evidence_id(AgentId::ClaudeCode)],
    }));
    let stable_skill_action_ids = actions
        .iter()
        .filter(|action| action.target.kind == ActionTargetKind::Skill)
        .map(|action| action.id.clone())
        .collect::<Vec<_>>();
    stable_skill.action_ids = stable_skill_action_ids.clone();
    skills
        .iter_mut()
        .find(|skill| skill.instance_id == stable_skill.instance_id)
        .expect("stable skill")
        .action_ids = stable_skill_action_ids;
    let stable_agent_action_id = actions
        .iter()
        .find(|action| action.target.kind == ActionTargetKind::Agent)
        .expect("agent action")
        .id
        .clone();
    let sessions = vec![
        session_input(AgentId::Codex, "stable-session-a", 20),
        session_input(AgentId::ClaudeCode, "stable-session-b", 10),
    ];

    let mut first_input = base_input();
    first_input.sessions = sessions.clone();
    let first_claude = first_input
        .agent_sources
        .iter_mut()
        .find(|source| source.agent == AgentId::ClaudeCode)
        .expect("Claude source");
    first_claude.coverage =
        SourceCoverage::incomplete(1, Some(2), ListIncompleteReason::SourceLimited);
    first_claude.action_ids = vec![stable_agent_action_id.clone()];
    first_input.actions = actions.clone();
    let first = project(first_input, skills.clone(), findings.clone(), Vec::new())
        .expect("ordered projection");

    skills.reverse();
    let stable_skill = skills
        .iter_mut()
        .find(|skill| skill.instance_id == "stable-claude")
        .expect("stable skill");
    stable_skill.action_ids.reverse();
    let mut reversed_findings = findings;
    reversed_findings.reverse();
    let mut second_input = base_input();
    second_input.agent_sources.reverse();
    let second_claude = second_input
        .agent_sources
        .iter_mut()
        .find(|source| source.agent == AgentId::ClaudeCode)
        .expect("Claude source");
    second_claude.coverage =
        SourceCoverage::incomplete(1, Some(2), ListIncompleteReason::SourceLimited);
    second_claude.action_ids = vec![stable_agent_action_id];
    actions.reverse();
    for action in &mut actions {
        action.impacts.reverse();
        action.evidence_refs.reverse();
    }
    second_input.actions = actions;
    second_input.sessions = sessions.into_iter().rev().collect();
    let second =
        project(second_input, skills, reversed_findings, Vec::new()).expect("reordered projection");

    assert_eq!(first.readiness.source_revision, REVISION);
    assert_eq!(second.readiness.source_revision, REVISION);
    assert_eq!(
        serde_json::to_vec(&first).expect("serialize first projection"),
        serde_json::to_vec(&second).expect("serialize reordered projection"),
        "provider-free projection output must depend only on accepted facts, not input order"
    );
}

#[test]
fn duplicate_logical_findings_are_canonicalized_before_projection() {
    let skill = skill_fact(
        "duplicate-finding-skill",
        AgentId::Codex,
        "duplicate-finding",
        "source:duplicate-finding",
        "native",
        "native:duplicate-finding",
    );
    let first = finding("duplicate-a", &skill.instance_id, "warning", false, "open");
    let mut second = first.clone();
    second.id = "duplicate-b".to_string();
    second.effective_severity = "critical".to_string();

    let ordered = project(
        base_input(),
        vec![skill.clone()],
        vec![first.clone(), second.clone()],
        Vec::new(),
    )
    .expect("ordered duplicate findings");
    let reversed = project(base_input(), vec![skill], vec![second, first], Vec::new())
        .expect("reversed duplicate findings");

    assert_eq!(
        serde_json::to_vec(&ordered).expect("serialize ordered findings"),
        serde_json::to_vec(&reversed).expect("serialize reversed findings")
    );
    assert_eq!(
        ordered
            .readiness
            .attention
            .iter()
            .filter(|item| item.kind == AttentionKind::Finding)
            .count(),
        1
    );
    assert!(ordered
        .readiness
        .attention
        .iter()
        .any(|item| item.kind == AttentionKind::Finding
            && item.severity == skills_copilot_core::AttentionSeverity::Critical));
}

#[test]
fn incomplete_coverage_table_never_produces_healthy_or_effective_assertions() {
    let cases = [
        ("partial", ListIncompleteReason::SafetyBudget),
        ("stale", ListIncompleteReason::StaleSource),
        ("unavailable", ListIncompleteReason::UnreadableSource),
        ("source-limited", ListIncompleteReason::SourceLimited),
    ];

    for (label, reason) in cases {
        let coverage = SourceCoverage::incomplete(0, Some(1), reason);
        let mut input = base_input();
        input
            .agent_sources
            .iter_mut()
            .find(|source| source.agent == AgentId::Codex)
            .expect("Codex source")
            .coverage = coverage.clone();
        let mut skill = skill_fact(
            &format!("{label}-skill"),
            AgentId::Codex,
            &format!("{label}-skill"),
            &format!("source:{label}"),
            "native",
            &format!("native:{label}"),
        );
        skill.coverage = coverage;

        let projection = project(input, vec![skill], Vec::new(), Vec::new())
            .unwrap_or_else(|error| panic!("{label} projection failed: {error}"));
        let codex = projection
            .readiness
            .agents
            .iter()
            .find(|agent| agent.agent == AgentId::Codex)
            .expect("Codex readiness");
        let aggregate = projection.skill_aggregates.first().expect("aggregate");

        assert_eq!(
            projection.readiness.health,
            EnvironmentHealthState::Blocked,
            "{label}"
        );
        assert_eq!(codex.health, EnvironmentHealthState::Blocked, "{label}");
        assert_eq!(
            aggregate.primary_effectiveness,
            SkillEffectivenessState::Unavailable,
            "{label}"
        );
        assert_eq!(aggregate.effective_instance_count, 0, "{label}");
    }
}

#[test]
fn publisher_package_and_version_are_material_grouping_dimensions() {
    let mut base = plugin_skill_fact();
    base.instance_id = "package-base".to_string();
    let mut publisher = base.clone();
    publisher.instance_id = "package-publisher".to_string();
    publisher.publisher = Some("another-publisher".to_string());
    let mut package = base.clone();
    package.instance_id = "package-name".to_string();
    package.package_name = Some("another-package".to_string());
    let mut version = base.clone();
    version.instance_id = "package-version".to_string();
    version.package_version = Some("9.9.9".to_string());

    let projection = project(
        base_input(),
        vec![base, publisher, package, version],
        Vec::new(),
        Vec::new(),
    )
    .expect("package identity projection");

    assert_eq!(
        projection.skill_aggregates.len(),
        4,
        "publisher, package name, and version must each prevent a merge"
    );
}

#[test]
fn installed_unlinked_requires_complete_manager_inventory() {
    let mut complete = skill_fact(
        "manager-complete",
        AgentId::Codex,
        "manager-skill",
        "manager:package:skill",
        "manager",
        "native:manager-skill",
    );
    complete.linked = false;
    let complete_projection = project(base_input(), vec![complete], Vec::new(), Vec::new())
        .expect("complete manager inventory");
    assert_eq!(
        complete_projection.skill_aggregates[0].primary_effectiveness,
        SkillEffectivenessState::InstalledUnlinked
    );

    let mut unknown = skill_fact(
        "manager-unknown",
        AgentId::Codex,
        "manager-skill",
        "manager:package:skill",
        "manager",
        "native:manager-skill",
    );
    unknown.linked = false;
    unknown.coverage = SourceCoverage::unknown(ListIncompleteReason::NotInspected);
    let unknown_projection = project(base_input(), vec![unknown], Vec::new(), Vec::new())
        .expect("unknown manager inventory");
    assert_eq!(
        unknown_projection.skill_aggregates[0].primary_effectiveness,
        SkillEffectivenessState::Unavailable
    );
    assert_eq!(
        unknown_projection.skill_aggregates[0].installed_instance_count,
        0
    );
}

#[test]
fn historical_missing_rows_are_excluded_from_current_product_inventory() {
    let mut missing = skill_fact(
        "historical-missing",
        AgentId::Codex,
        "historical",
        "source:historical",
        "native",
        "native:historical",
    );
    missing.adapter_state = SkillState::Missing;

    let projection = project(base_input(), vec![missing], Vec::new(), Vec::new())
        .expect("current inventory projection");

    assert!(projection.skill_aggregates.is_empty());
    assert!(projection.readiness.attention.is_empty());
    assert_eq!(projection.readiness.health, EnvironmentHealthState::Healthy);
}

#[test]
fn logical_source_identity_rejects_physical_path_data() {
    let mut skill = skill_fact(
        "private-source",
        AgentId::Codex,
        "private-source",
        "source:private",
        "native",
        "native:private-source",
    );
    skill.source_identity = "/private/source/SKILL.md".to_string();

    let error = project(base_input(), vec![skill], Vec::new(), Vec::new())
        .expect_err("physical source identity must fail closed");

    assert!(error.to_string().contains("private path"));

    let mut prefixed = skill_fact(
        "prefixed-private-source",
        AgentId::Codex,
        "prefixed-private-source",
        "source:private",
        "native",
        "native:prefixed-private-source",
    );
    let separator = '/';
    prefixed.source_identity = format!(
        "codex-plugin:{separator}Users{separator}example{separator}cache{separator}SKILL.md"
    );
    let error = project(base_input(), vec![prefixed], Vec::new(), Vec::new())
        .expect_err("prefixed physical source identity must fail closed");
    assert!(error.to_string().contains("path"));
}

fn base_input() -> ProductProjectionInput {
    ProductProjectionInput {
        project_id: PROJECT_ID.to_string(),
        project_display_name: "funnyaccount_system".to_string(),
        source_revision: REVISION.to_string(),
        agent_sources: [
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Opencode,
            AgentId::Pi,
            AgentId::Hermes,
            AgentId::Openclaw,
        ]
        .into_iter()
        .map(|agent| AgentProjectionInput {
            agent,
            source_revision: REVISION.to_string(),
            coverage: SourceCoverage::enumerable(1, Some(1)),
            evidence_summary: format!("{} adapter scan is complete", agent.as_str()),
            action_ids: Vec::new(),
        })
        .collect(),
        skills: Vec::new(),
        findings: Vec::new(),
        conflicts: Vec::new(),
        sessions: Vec::new(),
        actions: Vec::new(),
    }
}

fn session_input(agent: AgentId, id: &str, modified_at: i64) -> SessionProjectionInput {
    SessionProjectionInput {
        id: id.to_string(),
        agent,
        project_id: Some(PROJECT_ID.to_string()),
        title: format!("{id} title"),
        intent: Some("Continue deterministic work".to_string()),
        started_at: Some(modified_at - 1),
        ended_at: None,
        modified_at,
        source_kind: format!("{}-native-session", agent.as_str()),
        source_revision: format!("session-revision:{id}"),
        snapshot_revision: REVISION.to_string(),
        coverage: SourceCoverage::enumerable(1, Some(1)),
        resume: ResumeCapability::unsupported(ResumeUnsupportedReason::SessionUnsupported),
        evidence_summary: format!("{} native session source was inspected", agent.as_str()),
        action_ids: Vec::new(),
    }
}

fn project(
    mut input: ProductProjectionInput,
    skills: Vec<SkillProjectionInput>,
    findings: Vec<RuleFindingRecord>,
    conflicts: Vec<ConflictGroupRecord>,
) -> Result<ProductProjection, ProductProjectionError> {
    input.skills = skills;
    input.findings = findings
        .into_iter()
        .map(|record| FindingProjectionInput {
            source_revision: REVISION.to_string(),
            record,
        })
        .collect();
    input.conflicts = conflicts
        .into_iter()
        .map(|record| ConflictProjectionInput {
            source_revision: REVISION.to_string(),
            record,
        })
        .collect();
    derive_product_projection(input)
}

fn skill_fact(
    id: &str,
    agent: AgentId,
    name: &str,
    source_identity: &str,
    source_kind: &str,
    runtime_identity: &str,
) -> SkillProjectionInput {
    SkillProjectionInput {
        instance_id: id.to_string(),
        agent: Some(agent),
        scope: Scope::AgentGlobal,
        definition_id: format!("definition:{name}"),
        definition_fingerprint: Some("fingerprint:one".to_string()),
        canonical_name: name.to_string(),
        display_name: name.to_string(),
        description: format!("{name} fixture"),
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: source_kind.to_string(),
        source_identity: source_identity.to_string(),
        runtime_identity: runtime_identity.to_string(),
        source_revision: REVISION.to_string(),
        read_only_reason: None,
        installed: true,
        linked: true,
        enabled: true,
        precedence_proven: true,
        adapter_state: SkillState::Loaded,
        coverage: SourceCoverage::enumerable(1, Some(1)),
        evidence_summary: format!("{} authorized {source_kind} skill source", agent.as_str()),
        action_ids: Vec::new(),
    }
}

fn plugin_skill_fact() -> SkillProjectionInput {
    let mut fact = skill_fact(
        "codex-plugin-audit",
        AgentId::Codex,
        "audit",
        "codex-plugin:openai-curated-remote:product-design:0.1.52",
        "chatgpt-plugin-cache",
        "plugin:product-design@openai-curated-remote:audit",
    );
    fact.publisher = Some("openai-curated-remote".to_string());
    fact.package_name = Some("product-design".to_string());
    fact.package_version = Some("0.1.52".to_string());
    fact.read_only_reason = Some("Installed Codex plugin files are read-only".to_string());
    fact
}

fn opencode_compatibility_fact() -> SkillProjectionInput {
    skill_fact(
        "opencode-claude-compatibility",
        AgentId::Opencode,
        "seedream",
        "compatibility:opencode:claude:user:seedream",
        "opencode-compatibility",
        "native:seedream",
    )
}

fn finding(
    id: &str,
    instance_id: &str,
    severity: &str,
    suppressed: bool,
    triage_status: &str,
) -> RuleFindingRecord {
    RuleFindingRecord {
        id: id.to_string(),
        triage_key: format!("triage:{id}"),
        triage_context: "fixture".to_string(),
        instance_id: Some(instance_id.to_string()),
        definition_id: Some("definition:conflict".to_string()),
        rule_id: format!("rule.{id}"),
        severity: severity.to_string(),
        effective_severity: severity.to_string(),
        severity_override: None,
        message: format!("{id} finding requires review"),
        suggestion: Some("Review deterministic evidence".to_string()),
        created_at: 1,
        suppressed,
        suppression_reason: suppressed.then(|| "fixture".to_string()),
        suppression_note: None,
        rule_tuning_updated_at: None,
        triage_status: triage_status.to_string(),
        triage_note: None,
        triage_updated_at: None,
    }
}
