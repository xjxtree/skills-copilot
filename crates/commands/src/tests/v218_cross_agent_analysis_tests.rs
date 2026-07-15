use super::*;

#[test]
fn cross_agent_analysis_groups_duplicates_overlap_mismatch_and_broken_rows() {
    let shared_path = PathBuf::from("/tmp/shared/SKILL.md");
    let claude = analysis_skill(
        "claude-alpha",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        "review-diff",
        true,
        SkillState::Loaded,
        shared_path.clone(),
    );
    let mut codex = analysis_skill(
        "codex-alpha",
        AgentId::Codex,
        Scope::AgentGlobal,
        "review-diff",
        false,
        SkillState::Disabled,
        shared_path.clone(),
    );
    codex.display_path = PathBuf::from("/tmp/codex/shared/SKILL.md");
    let canonical_variant = analysis_skill(
        "pi-alpha",
        AgentId::Pi,
        Scope::AgentGlobal,
        "Review Diff",
        true,
        SkillState::Loaded,
        PathBuf::from("/tmp/pi/review/SKILL.md"),
    );
    let broken = analysis_skill(
        "broken-alpha",
        AgentId::Hermes,
        Scope::AgentGlobal,
        "broken-skill",
        false,
        SkillState::Broken,
        PathBuf::from("/tmp/hermes/broken/SKILL.md"),
    );

    let analysis = analyze_skill_instances(&[claude, codex, canonical_variant, broken]);

    assert_eq!(analysis.summary.duplicate_name_groups, 1);
    assert_eq!(analysis.summary.canonical_name_groups, 1);
    assert_eq!(analysis.summary.path_overlap_groups, 1);
    assert_eq!(analysis.summary.enabled_mismatch_groups, 1);
    assert_eq!(analysis.summary.malformed_groups, 1);
    assert!(analysis.summary.affected_skill_count >= 4);
    assert!(analysis.groups.iter().any(|group| {
        group.kind == "source_path_overlap"
            && group.instance_ids == vec!["claude-alpha".to_string(), "codex-alpha".to_string()]
    }));
    assert!(analysis
        .groups
        .iter()
        .any(|group| { group.kind == "malformed_or_broken" && group.severity == "error" }));
}

#[test]
fn precedence_analysis_only_selects_same_agent_loaded_project_winner() {
    let global = analysis_skill(
        "codex-global",
        AgentId::Codex,
        Scope::AgentGlobal,
        "ship-helper",
        true,
        SkillState::Loaded,
        PathBuf::from("/tmp/home/.agents/skills/ship-helper/SKILL.md"),
    );
    let project = analysis_skill(
        "codex-project",
        AgentId::Codex,
        Scope::AgentProject,
        "ship-helper",
        true,
        SkillState::Loaded,
        PathBuf::from("/tmp/project/.agents/skills/ship-helper/SKILL.md"),
    );
    let other_agent = analysis_skill(
        "claude-project",
        AgentId::ClaudeCode,
        Scope::AgentProject,
        "ship-helper",
        true,
        SkillState::Loaded,
        PathBuf::from("/tmp/project/.claude/skills/ship-helper/SKILL.md"),
    );

    let analysis = analyze_skill_instances(&[global, project, other_agent]);
    let precedence = analysis
        .groups
        .iter()
        .find(|group| group.kind == "precedence_shadowing")
        .expect("same-agent precedence group");

    assert_eq!(analysis.summary.precedence_groups, 1);
    assert_eq!(precedence.winner_id.as_deref(), Some("codex-project"));
    assert_eq!(precedence.agents, vec!["codex".to_string()]);
    assert!(precedence
        .explanation
        .contains("Cross-agent duplicates do not share runtime precedence"));
}

fn analysis_skill(
    id: &str,
    agent: AgentId,
    scope: Scope,
    name: &str,
    enabled: bool,
    state: SkillState,
    path: PathBuf,
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
        path: path.clone(),
        display_path: path,
        definition_id: hash_string(&canonical_skill_name_suggestion(name)),
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
