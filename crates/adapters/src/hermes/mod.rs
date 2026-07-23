use std::path::{Path, PathBuf};

use crate::shared::{
    optional_frontmatter_string, required_frontmatter_string, split_yaml_frontmatter,
    stable_path_id,
};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

use crate::environment::{absolute_env_path, expand_local_path};

#[derive(Debug, Default)]
pub struct HermesAdapter;

impl AgentAdapter for HermesAdapter {
    fn id(&self) -> AgentId {
        AgentId::Hermes
    }

    fn display_name(&self) -> &'static str {
        "Hermes"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let hermes_home = hermes_home_dir(ctx);
        let mut roots = vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: hermes_home.join("skills"),
            source: RootSource::UserHome,
            logical_source_id: Some("hermes-user-skills".to_string()),
        }];
        roots.extend(hermes_external_skill_roots(&hermes_home, &ctx.user_home));
        roots
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let fallback_name = containing_dir_name(path);
        let parsed = parse_skill_content(&content);
        let (frontmatter_raw, body, name, description, version, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name.clone(),
                parsed.description,
                parsed.version,
                SkillState::Loaded,
                true,
            ),
            Err(message) => (
                String::new(),
                content,
                fallback_name,
                message,
                None,
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id("hermes", path),
            agent: AgentId::Hermes,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name: name,
            description,
            version,
            state,
            enabled,
            frontmatter_raw,
            body,
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: String::new(),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        })
    }

    fn is_enabled(&self, instance: &SkillInstance) -> bool {
        instance.enabled
    }

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        vec![hermes_home_dir(ctx).join("config.yaml")]
    }
}

impl AgentConfigAdapter for HermesAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        doc.text = patch_hermes_config(&doc.text, &instance.name, on)?;
        Ok(())
    }
}

pub fn hermes_home_dir(ctx: &AdapterContext) -> PathBuf {
    absolute_env_path("HERMES_HOME").unwrap_or_else(|| ctx.user_home.join(".hermes"))
}

fn hermes_external_skill_roots(hermes_home: &Path, user_home: &Path) -> Vec<AdapterRoot> {
    let config_path = hermes_home.join("config.yaml");
    let Ok(config_text) = std::fs::read_to_string(&config_path) else {
        return Vec::new();
    };
    let config_dir = config_path.parent().unwrap_or(hermes_home);
    parse_external_dirs(&config_text, config_dir, user_home)
        .into_iter()
        .enumerate()
        .map(|(index, path)| AdapterRoot {
            scope: Scope::AgentGlobal,
            path,
            source: RootSource::Extra,
            logical_source_id: Some(format!("configured-external:{index}")),
        })
        .collect()
}

fn parse_external_dirs(config_text: &str, config_dir: &Path, user_home: &Path) -> Vec<PathBuf> {
    let Ok(config) = serde_norway::from_str::<serde_norway::Value>(config_text) else {
        return Vec::new();
    };
    let Some(external_dirs) = config
        .get("skills")
        .and_then(|skills| skills.get("external_dirs"))
        .and_then(serde_norway::Value::as_sequence)
    else {
        return Vec::new();
    };

    let mut dirs = Vec::new();
    for entry in external_dirs {
        let Some(raw_dir) = entry.as_str() else {
            continue;
        };
        let Some(path) = external_dir_path(raw_dir, config_dir, user_home) else {
            continue;
        };
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }
    dirs
}

pub fn hermes_disabled_skill_names(config_text: &str) -> Vec<String> {
    serde_norway::from_str::<serde_norway::Value>(config_text)
        .ok()
        .and_then(|value| {
            value
                .get("skills")
                .and_then(|skills| skills.get("disabled"))
                .and_then(serde_norway::Value::as_sequence)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_norway::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_default()
}

fn patch_hermes_config(
    config_text: &str,
    skill_name: &str,
    enabled: bool,
) -> Result<String, AdapterError> {
    let mut value = if config_text.trim().is_empty() {
        serde_norway::Value::Mapping(serde_norway::Mapping::new())
    } else {
        serde_norway::from_str(config_text)
            .map_err(|err| AdapterError::new(format!("invalid Hermes config YAML: {err}")))?
    };

    let disabled = hermes_disabled_array_mut(&mut value)?;
    if enabled {
        disabled.retain(|value| value.as_str() != Some(skill_name));
    } else if !disabled
        .iter()
        .any(|value| value.as_str() == Some(skill_name))
    {
        disabled.push(serde_norway::Value::String(skill_name.to_string()));
    }

    let mut text = serde_norway::to_string(&value)
        .map_err(|err| AdapterError::new(format!("failed to serialize Hermes config: {err}")))?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

fn hermes_disabled_array_mut(
    value: &mut serde_norway::Value,
) -> Result<&mut Vec<serde_norway::Value>, AdapterError> {
    use serde_norway::{Mapping, Value};

    let Value::Mapping(root) = value else {
        return Err(AdapterError::new(
            "Hermes config must be a YAML mapping before it can be patched",
        ));
    };

    let skills_key = Value::String("skills".to_string());
    if !root.contains_key(&skills_key) {
        root.insert(skills_key.clone(), Value::Mapping(Mapping::new()));
    }
    let skills = root
        .get_mut(&skills_key)
        .ok_or_else(|| AdapterError::new("failed to create Hermes skills config"))?;
    let Value::Mapping(skills) = skills else {
        return Err(AdapterError::new(
            "Hermes config `skills` must be a mapping before it can be patched",
        ));
    };

    let disabled_key = Value::String("disabled".to_string());
    if !skills.contains_key(&disabled_key) {
        skills.insert(disabled_key.clone(), Value::Sequence(Vec::new()));
    }
    let disabled = skills
        .get_mut(&disabled_key)
        .ok_or_else(|| AdapterError::new("failed to create Hermes disabled skills list"))?;
    let Value::Sequence(disabled) = disabled else {
        return Err(AdapterError::new(
            "Hermes config `skills.disabled` must be a sequence before it can be patched",
        ));
    };
    Ok(disabled)
}

fn external_dir_path(raw_dir: &str, config_dir: &Path, user_home: &Path) -> Option<PathBuf> {
    expand_local_path(raw_dir, user_home, config_dir)
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
    version: Option<String>,
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_yaml_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = required_frontmatter_string(&frontmatter, "name", "Hermes")?;
    let description = required_frontmatter_string(&frontmatter, "description", "Hermes")?;
    let version = optional_frontmatter_string(&frontmatter, "version");

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        version,
    })
}

fn containing_dir_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_contract_preserves_scalar_sequence_bool_and_nested_mapping() {
        let raw = "name: sample-skill\ndescription: Sample\nenabled: true\nallowed-tools:\n  - Read\n  - Search\nmetadata:\n  openclaw:\n    skillKey: routed-key\n";
        let value: serde_norway::Value = serde_norway::from_str(raw).expect("yaml parses");

        assert_eq!(
            value.get("name").and_then(serde_norway::Value::as_str),
            Some("sample-skill")
        );
        assert_eq!(
            value.get("enabled").and_then(serde_norway::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value
                .get("allowed-tools")
                .and_then(serde_norway::Value::as_sequence)
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            value
                .get("metadata")
                .and_then(|item| item.get("openclaw"))
                .and_then(|item| item.get("skillKey"))
                .and_then(serde_norway::Value::as_str),
            Some("routed-key")
        );

        let parsed = parse_skill_content(&format!("---\n{raw}---\nBody.\n"))
            .expect("adapter frontmatter parses");
        assert_eq!(parsed.name, "sample-skill");
        assert_eq!(parsed.description, "Sample");
    }

    #[test]
    fn yaml_contract_malformed_frontmatter_returns_error() {
        let result =
            parse_skill_content("---\nname: [unterminated\ndescription: Sample\n---\nBody.\n");

        assert!(result.is_err());
    }

    #[test]
    fn exposes_active_hermes_home_without_inferred_project_or_extra_roots() {
        let adapter = HermesAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/project")),
            project_cwd: Some(PathBuf::from("/tmp/project/nested")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from("/tmp/unverified"),
                source: RootSource::Extra,
                logical_source_id: None,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].scope, Scope::AgentGlobal);
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(roots[0].path, PathBuf::from("/tmp/home/.hermes/skills"));
    }

    #[test]
    fn exposes_explicit_external_dirs_from_hermes_config_as_read_only_extra_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-hermes-external-roots-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let external_one = temp_root.join("external-one");
        std::fs::create_dir_all(home.join(".hermes")).expect("create Hermes home");
        std::fs::write(
            home.join(".hermes/config.yaml"),
            format!(
                "skills:\n  external_dirs:\n    - {}\n    - ~/shared-hermes\n    - ../relative-external\n    - {}\n    - 42\n",
                external_one.display(),
                external_one.display()
            ),
        )
        .expect("write Hermes config");
        let adapter = HermesAdapter;
        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: Some(temp_root.join("project")),
            project_cwd: Some(temp_root.join("project/nested")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: temp_root.join("unverified"),
                source: RootSource::Extra,
                logical_source_id: None,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots.len(), 4);
        assert_eq!(roots[0].path, home.join(".hermes/skills"));
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(roots[1].path, external_one);
        assert_eq!(roots[1].scope, Scope::AgentGlobal);
        assert_eq!(roots[1].source, RootSource::Extra);
        assert_eq!(roots[2].path, home.join("shared-hermes"));
        assert_eq!(roots[2].source, RootSource::Extra);
        assert_eq!(roots[3].path, home.join("relative-external"));
        assert_eq!(roots[3].source, RootSource::Extra);
        assert!(
            roots
                .iter()
                .all(|root| root.path != temp_root.join("project")),
            "Hermes must not infer generic project roots"
        );
        assert!(
            roots
                .iter()
                .all(|root| root.path != temp_root.join("unverified")),
            "Hermes must not consume AdapterContext extra_roots as external_dirs"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn parses_nested_hermes_skill_frontmatter() {
        let adapter = HermesAdapter;
        let fixture = fixture_path(
            "fixtures/hermes/active-home/.hermes/skills/nested/research-brief/SKILL.md",
        );

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Hermes);
        assert_eq!(skill.scope, Scope::AgentGlobal);
        assert_eq!(skill.name, "research-brief");
        assert_eq!(
            skill.description,
            "Prepare read-only research summaries for Hermes sessions."
        );
        assert_eq!(skill.version.as_deref(), Some("0.1.0"));
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = HermesAdapter;
        let fixture = fixture_path(
            "fixtures/hermes/active-home/.hermes/skills/nested/research-brief/SKILL.md",
        );

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_malformed_frontmatter_as_broken_without_failing_parse() {
        let adapter = HermesAdapter;
        let fixture = fixture_path(
            "fixtures/hermes/active-home/.hermes/skills/broken/malformed-metadata/SKILL.md",
        );

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.agent, AgentId::Hermes);
        assert_eq!(skill.name, "malformed-metadata");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
    }

    #[test]
    fn yaml_contract_patch_enabled_preserves_unrelated_mappings_and_scalars() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/home/.hermes/config.yaml"),
            format: skills_copilot_core::ConfigFormat::Yaml,
            text: "telemetry: false\nui:\n  theme: dark\n  compact: true\nskills:\n  external_dirs:\n    - ~/team-skills\n  disabled:\n    - old-skill\n"
                .to_string(),
        };
        let skill = SkillInstance {
            id: "hermes:test".to_string(),
            agent: AgentId::Hermes,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from("/tmp/home/.hermes/skills/new-skill/SKILL.md"),
            display_path: PathBuf::from("/tmp/home/.hermes/skills/new-skill/SKILL.md"),
            definition_id: "new-skill".to_string(),
            name: "new-skill".to_string(),
            display_name: "new-skill".to_string(),
            description: String::new(),
            version: None,
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: String::new(),
            body: String::new(),
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: String::new(),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        };

        HermesAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable succeeds");
        let disabled = hermes_disabled_skill_names(&doc.text);
        assert!(disabled.contains(&"old-skill".to_string()));
        assert!(disabled.contains(&"new-skill".to_string()));
        assert!(doc.text.contains("external_dirs"));
        let value: serde_norway::Value =
            serde_norway::from_str(&doc.text).expect("patched yaml parses");
        assert_eq!(
            value
                .get("telemetry")
                .and_then(serde_norway::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .get("ui")
                .and_then(|ui| ui.get("theme"))
                .and_then(serde_norway::Value::as_str),
            Some("dark")
        );
        assert_eq!(
            value
                .get("ui")
                .and_then(|ui| ui.get("compact"))
                .and_then(serde_norway::Value::as_bool),
            Some(true)
        );

        HermesAdapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable succeeds");
        let disabled = hermes_disabled_skill_names(&doc.text);
        assert!(disabled.contains(&"old-skill".to_string()));
        assert!(!disabled.contains(&"new-skill".to_string()));
        let value: serde_norway::Value =
            serde_norway::from_str(&doc.text).expect("re-enabled yaml parses");
        assert_eq!(
            value
                .get("telemetry")
                .and_then(serde_norway::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .get("ui")
                .and_then(|ui| ui.get("theme"))
                .and_then(serde_norway::Value::as_str),
            Some("dark")
        );
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
