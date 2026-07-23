use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    #[serde(rename = "tool-global")]
    ToolGlobal,
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "pi")]
    Pi,
    #[serde(rename = "hermes")]
    Hermes,
    #[serde(rename = "openclaw")]
    Openclaw,
    #[serde(rename = "opencode")]
    Opencode,
}

impl AgentId {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentId::ToolGlobal => "tool-global",
            AgentId::ClaudeCode => "claude-code",
            AgentId::Codex => "codex",
            AgentId::Pi => "pi",
            AgentId::Hermes => "hermes",
            AgentId::Openclaw => "openclaw",
            AgentId::Opencode => "opencode",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Scope {
    #[serde(rename = "tool-global")]
    ToolGlobal,
    #[serde(rename = "agent-global")]
    AgentGlobal,
    #[serde(rename = "agent-project")]
    AgentProject,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ToolGlobal => "tool-global",
            Scope::AgentGlobal => "agent-global",
            Scope::AgentProject => "agent-project",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SkillState {
    Loaded,
    Disabled,
    Shadowed,
    Broken,
    Missing,
}

impl SkillState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillState::Loaded => "loaded",
            SkillState::Disabled => "disabled",
            SkillState::Shadowed => "shadowed",
            SkillState::Broken => "broken",
            SkillState::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NetworkAccess {
    None,
    ReadOnly,
    Full,
    Unknown(String),
}

impl NetworkAccess {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkAccess::None => "none",
            NetworkAccess::ReadOnly => "read-only",
            NetworkAccess::Full => "full",
            NetworkAccess::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PermissionRequest {
    pub tools: Vec<String>,
    pub files: Vec<String>,
    pub network: NetworkAccess,
    pub network_declared: bool,
    pub exec: bool,
    pub exec_declared: bool,
    pub requires_human: bool,
    pub requires_human_declared: bool,
}

impl Default for PermissionRequest {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            files: Vec::new(),
            network: NetworkAccess::None,
            network_declared: false,
            exec: false,
            exec_declared: false,
            requires_human: true,
            requires_human_declared: false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillScript {
    pub name: String,
    pub path: PathBuf,
    pub interpreter: Option<String>,
    pub description: Option<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillInstance {
    pub id: String,
    pub agent: AgentId,
    pub scope: Scope,
    pub project_root: Option<PathBuf>,
    pub path: PathBuf,
    pub display_path: PathBuf,
    pub definition_id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: Option<String>,
    pub state: SkillState,
    pub enabled: bool,
    pub frontmatter_raw: String,
    pub body: String,
    pub scripts: Vec<SkillScript>,
    pub permissions: PermissionRequest,
    pub fingerprint: String,
    pub mtime: i64,
    pub first_seen: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillDefinition {
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
pub enum ConflictReason {
    NameCollision,
    ContentDrift,
    PermissionMismatch,
    Shadowed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictGroup {
    pub definition_id: String,
    pub reason: ConflictReason,
    pub instances: Vec<String>,
    pub winner_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_skill_instance() -> SkillInstance {
        SkillInstance {
            id: "codex:agent-project:skills/refactor/SKILL.md".to_string(),
            agent: AgentId::Codex,
            scope: Scope::AgentProject,
            project_root: Some(PathBuf::from("workspace")),
            path: PathBuf::from("skills/refactor/SKILL.md"),
            display_path: PathBuf::from(".codex/skills/refactor/SKILL.md"),
            definition_id: "skill:refactor".to_string(),
            name: "refactor".to_string(),
            display_name: "Refactor".to_string(),
            description: "Safely refactor local code.".to_string(),
            version: Some("1.2.3".to_string()),
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: "name: refactor".to_string(),
            body: "Use focused edits and tests.".to_string(),
            scripts: vec![SkillScript {
                name: "check".to_string(),
                path: PathBuf::from("skills/refactor/scripts/check.sh"),
                interpreter: Some("bash".to_string()),
                description: Some("Run focused checks.".to_string()),
                fingerprint: "sha256:script".to_string(),
            }],
            permissions: PermissionRequest::default(),
            fingerprint: "sha256:instance".to_string(),
            mtime: 1_700_000_000,
            first_seen: 1_700_000_001,
            last_seen: 1_700_000_002,
        }
    }

    #[test]
    fn agent_id_wire_values_are_stable() {
        let cases = [
            (AgentId::ToolGlobal, "tool-global"),
            (AgentId::ClaudeCode, "claude-code"),
            (AgentId::Codex, "codex"),
            (AgentId::Pi, "pi"),
            (AgentId::Hermes, "hermes"),
            (AgentId::Openclaw, "openclaw"),
            (AgentId::Opencode, "opencode"),
        ];

        for (agent, expected) in cases {
            assert_eq!(agent.as_str(), expected);
        }
    }

    #[test]
    fn scope_wire_values_are_stable() {
        let cases = [
            (Scope::ToolGlobal, "tool-global"),
            (Scope::AgentGlobal, "agent-global"),
            (Scope::AgentProject, "agent-project"),
        ];

        for (scope, expected) in cases {
            assert_eq!(scope.as_str(), expected);
        }
    }

    #[test]
    fn permission_request_default_is_safe_and_human_gated() {
        let permissions = PermissionRequest::default();

        assert!(permissions.tools.is_empty());
        assert!(permissions.files.is_empty());
        assert_eq!(permissions.network, NetworkAccess::None);
        assert_eq!(permissions.network.as_str(), "none");
        assert!(!permissions.network_declared);
        assert!(!permissions.exec);
        assert!(!permissions.exec_declared);
        assert!(permissions.requires_human);
        assert!(!permissions.requires_human_declared);
    }

    #[test]
    fn skill_instance_preserves_identity_and_state_fields() {
        let instance = sample_skill_instance();

        assert_eq!(instance.id, "codex:agent-project:skills/refactor/SKILL.md");
        assert_eq!(instance.definition_id, "skill:refactor");
        assert_eq!(instance.agent, AgentId::Codex);
        assert_eq!(instance.agent.as_str(), "codex");
        assert_eq!(instance.scope, Scope::AgentProject);
        assert_eq!(instance.scope.as_str(), "agent-project");
        assert_eq!(instance.project_root, Some(PathBuf::from("workspace")));
        assert_eq!(instance.path, PathBuf::from("skills/refactor/SKILL.md"));
        assert_eq!(
            instance.display_path,
            PathBuf::from(".codex/skills/refactor/SKILL.md")
        );
        assert!(instance.enabled);
        assert_eq!(instance.state, SkillState::Loaded);
        assert_eq!(instance.state.as_str(), "loaded");
        assert_eq!(instance.fingerprint, "sha256:instance");
        assert_eq!(instance.permissions, PermissionRequest::default());
        assert_eq!(instance.scripts.len(), 1);
        assert_eq!(instance.scripts[0].fingerprint, "sha256:script");
    }

    #[test]
    fn skill_definition_preserves_aggregate_identity_fields() {
        let instance = sample_skill_instance();
        let definition = SkillDefinition {
            id: instance.definition_id.clone(),
            canonical_name: instance.name.clone(),
            description: instance.description.clone(),
            instances: vec![
                instance.id.clone(),
                "codex:agent-global:refactor".to_string(),
            ],
            active_instance: Some(instance.id.clone()),
            has_multiple_instances: true,
            has_conflict: true,
            fingerprint_set: vec![instance.fingerprint.clone(), "sha256:global".to_string()],
        };

        assert_eq!(definition.id, "skill:refactor");
        assert_eq!(definition.canonical_name, "refactor");
        assert_eq!(definition.description, "Safely refactor local code.");
        assert_eq!(
            definition.instances,
            vec![
                "codex:agent-project:skills/refactor/SKILL.md".to_string(),
                "codex:agent-global:refactor".to_string()
            ]
        );
        assert_eq!(
            definition.active_instance,
            Some("codex:agent-project:skills/refactor/SKILL.md".to_string())
        );
        assert!(definition.has_multiple_instances);
        assert!(definition.has_conflict);
        assert_eq!(
            definition.fingerprint_set,
            vec!["sha256:instance".to_string(), "sha256:global".to_string()]
        );
    }
}
