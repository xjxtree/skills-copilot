use std::collections::{BTreeMap, BTreeSet, HashMap};

use skills_copilot_core::{Scope, SkillInstance};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Finding {
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DefinitionSummary {
    pub id: String,
    pub canonical_name: String,
    pub description: String,
    pub instances: Vec<String>,
    pub active_instance: Option<String>,
    pub has_multiple_instances: bool,
    pub has_conflict: bool,
    pub fingerprint_set: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictSummary {
    pub id: String,
    pub definition_id: String,
    pub reason: String,
    pub winner_id: Option<String>,
    pub instances: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RuleContext {
    pub previous_fingerprints: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct RuleReport {
    pub findings: Vec<Finding>,
    pub definitions: Vec<DefinitionSummary>,
    pub conflicts: Vec<ConflictSummary>,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn applies_to(&self, inst: &SkillInstance) -> bool;
    fn check(&self, inst: &SkillInstance, ctx: &RuleContext) -> Vec<Finding>;
}

pub fn evaluate_mvp_rules(instances: &[SkillInstance], ctx: &RuleContext) -> RuleReport {
    let rules: [&dyn Rule; 3] = [
        &FrontmatterRequiredFields,
        &PathOutsideWorkspace,
        &FingerprintChanged,
    ];
    let mut report = RuleReport::default();

    for inst in instances {
        for rule in rules {
            if rule.applies_to(inst) {
                report.findings.extend(rule.check(inst, ctx));
            }
        }
    }

    append_name_collision_results(instances, &mut report);
    report
}

struct FrontmatterRequiredFields;

impl Rule for FrontmatterRequiredFields {
    fn id(&self) -> &'static str {
        "frontmatter.required-fields"
    }

    fn applies_to(&self, _inst: &SkillInstance) -> bool {
        true
    }

    fn check(&self, inst: &SkillInstance, _ctx: &RuleContext) -> Vec<Finding> {
        let missing = missing_frontmatter_fields(inst);
        if missing.is_empty() {
            return Vec::new();
        }
        vec![Finding {
            instance_id: Some(inst.id.clone()),
            definition_id: Some(inst.definition_id.clone()),
            rule_id: self.id().to_string(),
            severity: Severity::Error,
            message: format!(
                "Missing required frontmatter fields: {}",
                missing.join(", ")
            ),
            suggestion: Some(
                "Add name and description to the SKILL.md YAML frontmatter.".to_string(),
            ),
        }]
    }
}

fn missing_frontmatter_fields(inst: &SkillInstance) -> Vec<&'static str> {
    if inst.frontmatter_raw.trim().is_empty() {
        return vec!["name", "description"];
    }
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&inst.frontmatter_raw) else {
        return vec!["name", "description"];
    };
    let mut missing = Vec::new();
    if yaml_string_field(&value, "name").is_none() {
        missing.push("name");
    }
    if yaml_string_field(&value, "description").is_none() {
        missing.push("description");
    }
    missing
}

fn yaml_string_field<'a>(value: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
    value
        .get(serde_yaml::Value::String(key.to_string()))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

struct PathOutsideWorkspace;

impl Rule for PathOutsideWorkspace {
    fn id(&self) -> &'static str {
        "path.outside-workspace"
    }

    fn applies_to(&self, inst: &SkillInstance) -> bool {
        inst.scope == Scope::AgentProject
    }

    fn check(&self, inst: &SkillInstance, _ctx: &RuleContext) -> Vec<Finding> {
        let Some(project_root) = &inst.project_root else {
            return vec![outside_workspace_finding(
                inst,
                "Project-scoped skill has no project root.",
            )];
        };
        if inst.path.starts_with(project_root) {
            return Vec::new();
        }
        vec![outside_workspace_finding(
            inst,
            "Project-scoped skill path is outside its project root.",
        )]
    }
}

fn outside_workspace_finding(inst: &SkillInstance, message: &str) -> Finding {
    Finding {
        instance_id: Some(inst.id.clone()),
        definition_id: Some(inst.definition_id.clone()),
        rule_id: "path.outside-workspace".to_string(),
        severity: Severity::Error,
        message: message.to_string(),
        suggestion: Some(
            "Move the skill under <project>/.claude/skills or rescan it as a global skill."
                .to_string(),
        ),
    }
}

struct FingerprintChanged;

impl Rule for FingerprintChanged {
    fn id(&self) -> &'static str {
        "fingerprint.changed"
    }

    fn applies_to(&self, inst: &SkillInstance) -> bool {
        !inst.fingerprint.is_empty()
    }

    fn check(&self, inst: &SkillInstance, ctx: &RuleContext) -> Vec<Finding> {
        let Some(previous) = ctx.previous_fingerprints.get(&inst.id) else {
            return Vec::new();
        };
        if previous == &inst.fingerprint {
            return Vec::new();
        }
        vec![Finding {
            instance_id: Some(inst.id.clone()),
            definition_id: Some(inst.definition_id.clone()),
            rule_id: self.id().to_string(),
            severity: Severity::Info,
            message: "Skill content fingerprint changed since the previous scan.".to_string(),
            suggestion: Some(
                "Review the skill details before relying on this version.".to_string(),
            ),
        }]
    }
}

fn append_name_collision_results(instances: &[SkillInstance], report: &mut RuleReport) {
    let mut groups: BTreeMap<&str, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        groups
            .entry(inst.definition_id.as_str())
            .or_default()
            .push(inst);
    }

    for (definition_id, group) in groups {
        let canonical_name = group[0].name.clone();
        let description = group
            .iter()
            .find(|inst| !inst.description.trim().is_empty())
            .map(|inst| inst.description.clone())
            .unwrap_or_default();
        let instances: Vec<String> = group.iter().map(|inst| inst.id.clone()).collect();
        let fingerprint_set: Vec<String> = group
            .iter()
            .map(|inst| inst.fingerprint.clone())
            .filter(|fp| !fp.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let runtime_conflicts = runtime_name_collision_groups(&group);
        let has_multiple_instances = instances.len() > 1;
        let has_conflict = !runtime_conflicts.is_empty();

        report.definitions.push(DefinitionSummary {
            id: definition_id.to_string(),
            canonical_name: canonical_name.clone(),
            description,
            active_instance: group
                .iter()
                .find(|inst| inst.enabled)
                .map(|inst| inst.id.clone())
                .or_else(|| group.first().map(|inst| inst.id.clone())),
            instances: instances.clone(),
            has_multiple_instances,
            has_conflict,
            fingerprint_set: fingerprint_set.clone(),
        });

        for (agent, collision_group) in runtime_conflicts {
            let collision_instances: Vec<String> =
                collision_group.iter().map(|inst| inst.id.clone()).collect();
            let collision_fingerprints: Vec<String> = collision_group
                .iter()
                .map(|inst| inst.fingerprint.clone())
                .filter(|fp| !fp.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let has_content_drift = collision_fingerprints.len() > 1;
            let reason = if has_content_drift {
                "content-drift"
            } else {
                "name-collision"
            };
            report.conflicts.push(ConflictSummary {
                id: format!("{definition_id}:{agent}:{reason}"),
                definition_id: definition_id.to_string(),
                reason: reason.to_string(),
                winner_id: None,
                instances: collision_instances.clone(),
            });

            let severity = if has_content_drift {
                Severity::Warn
            } else {
                Severity::Info
            };
            for inst in collision_group {
                report.findings.push(Finding {
                    instance_id: Some(inst.id.clone()),
                    definition_id: Some(definition_id.to_string()),
                    rule_id: "name.collision".to_string(),
                    severity: severity.clone(),
                    message: format!(
                        "{} runtime sees skill name '{}' in {} locations.",
                        inst.agent.as_str(),
                        canonical_name,
                        collision_instances.len()
                    ),
                    suggestion: Some(
                        if has_content_drift {
                            "Compare the conflicting skill bodies and choose the intended version."
                        } else {
                            "Confirm that duplicate skill locations are intentional."
                        }
                        .to_string(),
                    ),
                });
            }
        }
    }
}

fn runtime_name_collision_groups<'a>(
    group: &[&'a SkillInstance],
) -> Vec<(String, Vec<&'a SkillInstance>)> {
    let mut by_agent: BTreeMap<String, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in group {
        by_agent
            .entry(inst.agent.as_str().to_string())
            .or_default()
            .push(*inst);
    }

    by_agent
        .into_iter()
        .filter_map(|(agent, members)| {
            let distinct_paths = members
                .iter()
                .map(|inst| inst.path.to_string_lossy().to_string())
                .collect::<BTreeSet<_>>();
            if members.len() > 1 && distinct_paths.len() > 1 {
                Some((agent, members))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use skills_copilot_core::{
        AgentId, NetworkAccess, PermissionRequest, Scope, SkillInstance, SkillState,
    };

    use super::*;

    #[test]
    fn yaml_contract_frontmatter_required_fields_handles_nested_values_and_malformed_input() {
        let valid = skill(
            "yaml-valid",
            "yaml-valid",
            "yaml-valid",
            "name: yaml-valid\ndescription: Valid\nenabled: true\nallowed-tools:\n  - Read\nmetadata:\n  openclaw:\n    skillKey: routed-key\n",
            "body",
        );
        let valid_report = evaluate_mvp_rules(&[valid], &RuleContext::default());
        assert!(valid_report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "frontmatter.required-fields"));

        let malformed = skill(
            "yaml-malformed",
            "yaml-malformed",
            "yaml-malformed",
            "name: [unterminated\n",
            "body",
        );
        let malformed_report = evaluate_mvp_rules(&[malformed], &RuleContext::default());
        assert!(malformed_report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "frontmatter.required-fields"));
    }

    #[test]
    fn required_fields_reports_missing_description() {
        let inst = skill("a", "same", "same", "---\nname: same\n---\n", "body");
        let report = evaluate_mvp_rules(&[inst], &RuleContext::default());

        assert!(report.findings.iter().any(|finding| {
            finding.rule_id == "frontmatter.required-fields"
                && finding.message.contains("description")
        }));
    }

    #[test]
    fn same_agent_name_collision_creates_conflict_and_findings() {
        let first = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let second = skill(
            "b",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].reason, "content-drift");
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.rule_id == "name.collision")
                .count(),
            2
        );
    }

    #[test]
    fn cross_agent_duplicate_does_not_create_runtime_conflict() {
        let first = skill(
            "claude",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let mut second = skill(
            "codex",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        second.agent = AgentId::Codex;

        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());
        let definition = report
            .definitions
            .iter()
            .find(|definition| definition.id == "same")
            .expect("definition");

        assert!(definition.has_multiple_instances);
        assert!(!definition.has_conflict);
        assert!(report.conflicts.is_empty());
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "name.collision"));
    }

    #[test]
    fn same_agent_same_path_duplicate_does_not_create_runtime_conflict() {
        let first = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let mut second = skill(
            "b",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        second.path = first.path.clone();
        second.display_path = first.display_path.clone();

        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn path_outside_workspace_flags_project_skill() {
        let mut inst = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body",
        );
        inst.scope = Scope::AgentProject;
        inst.project_root = Some(PathBuf::from("/tmp/project"));
        inst.path = PathBuf::from("/tmp/other/.claude/skills/same/SKILL.md");

        let report = evaluate_mvp_rules(&[inst], &RuleContext::default());

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "path.outside-workspace"));
    }

    #[test]
    fn fingerprint_changed_compares_previous_scan() {
        let inst = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body",
        );
        let mut ctx = RuleContext::default();
        ctx.previous_fingerprints
            .insert(inst.id.clone(), "old-fingerprint".to_string());

        let report = evaluate_mvp_rules(&[inst], &ctx);

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "fingerprint.changed"));
    }

    fn skill(
        id: &str,
        definition_id: &str,
        name: &str,
        frontmatter_raw: &str,
        body: &str,
    ) -> SkillInstance {
        SkillInstance {
            id: id.to_string(),
            agent: AgentId::ClaudeCode,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from(format!("/tmp/{id}/{name}/SKILL.md")),
            display_path: PathBuf::from(format!("/tmp/{id}/{name}/SKILL.md")),
            definition_id: definition_id.to_string(),
            name: name.to_string(),
            display_name: name.to_string(),
            description: "description".to_string(),
            version: None,
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: frontmatter_raw.to_string(),
            body: body.to_string(),
            scripts: Vec::new(),
            permissions: PermissionRequest {
                network: NetworkAccess::None,
                ..PermissionRequest::default()
            },
            fingerprint: format!("{body}-fingerprint"),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        }
    }
}
