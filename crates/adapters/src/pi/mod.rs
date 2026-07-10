use std::path::{Path, PathBuf};

use crate::shared::{
    required_frontmatter_string, split_yaml_frontmatter, stable_path_id, validate_kebab_skill_name,
};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

#[derive(Debug, Default)]
pub struct PiAdapter;

impl AgentAdapter for PiAdapter {
    fn id(&self) -> AgentId {
        AgentId::Pi
    }

    fn display_name(&self) -> &'static str {
        "Pi"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let mut roots = vec![
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".pi/agent/skills"),
                source: RootSource::UserHome,
            },
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".agents/skills"),
                source: RootSource::Compatibility,
            },
        ];

        if let Some(project_root) = &ctx.project_root {
            roots.extend(pi_project_skill_roots(
                project_root,
                ctx.project_cwd.as_deref(),
            ));
        }

        roots
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let fallback_name = fallback_skill_name(path);
        let parsed = parse_skill_content(&content);
        let (frontmatter_raw, body, name, description, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name.clone(),
                parsed.description,
                SkillState::Loaded,
                true,
            ),
            Err(message) => (
                String::new(),
                content,
                fallback_name.clone(),
                message,
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id("pi", path),
            agent: AgentId::Pi,
            scope: Scope::AgentProject,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name: name,
            description,
            version: None,
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
        let mut paths = vec![ctx.user_home.join(".pi/agent/settings.json")];
        if let Some(project_root) = &ctx.project_root {
            paths.push(project_root.join(".pi/settings.json"));
        }
        paths
    }
}

impl AgentConfigAdapter for PiAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        doc.text = patch_pi_config(&doc.text, &instance.name, on, instance.scope)?;
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_yaml_frontmatter(rest)?;
    let frontmatter: serde_yaml::Value =
        serde_yaml::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = required_frontmatter_string(&frontmatter, "name", "Pi")?;
    validate_kebab_skill_name(&name, "Pi")?;
    let description = required_frontmatter_string(&frontmatter, "description", "Pi")?;

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
    })
}

fn pi_project_skill_roots(project_root: &Path, project_cwd: Option<&Path>) -> Vec<AdapterRoot> {
    let start = project_cwd
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    let mut roots = Vec::new();
    let mut current = Some(start);

    while let Some(dir) = current {
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".pi/skills"),
            source: RootSource::Project,
        });
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".agents/skills"),
            source: RootSource::Compatibility,
        });
        if dir == project_root {
            break;
        }
        current = dir
            .parent()
            .filter(|parent| parent.starts_with(project_root));
    }

    roots
}

fn fallback_skill_name(path: &Path) -> String {
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        return path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
    }
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn patch_pi_config(
    content: &str,
    skill_name: &str,
    enabled: bool,
    scope: Scope,
) -> Result<String, AdapterError> {
    let mut value = if content.trim().is_empty() {
        serde_json::json!({
            "skills": {
                "disabled": []
            }
        })
    } else {
        serde_json::from_str(content)
            .map_err(|err| AdapterError::new(format!("invalid Pi settings JSON: {err}")))?
    };

    if scope == Scope::AgentProject && pi_project_explicitly_untrusted(&value) {
        return Err(AdapterError::new(
            "Pi project settings explicitly mark this project untrusted; project/package toggles are blocked",
        ));
    }

    let disabled = pi_disabled_array_mut(&mut value)?;
    if enabled {
        disabled.retain(|value| value.as_str() != Some(skill_name));
    } else if !disabled
        .iter()
        .any(|value| value.as_str() == Some(skill_name))
    {
        disabled.push(serde_json::Value::String(skill_name.to_string()));
    }

    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|err| AdapterError::new(format!("failed to serialize Pi settings: {err}")))?;
    text.push('\n');
    Ok(text)
}

fn pi_project_explicitly_untrusted(value: &serde_json::Value) -> bool {
    value
        .get("project")
        .and_then(|project| project.get("trusted"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
        || value
            .get("trust")
            .and_then(|trust| trust.get("projectRootTrusted"))
            .and_then(serde_json::Value::as_bool)
            == Some(false)
}

fn pi_disabled_array_mut(
    value: &mut serde_json::Value,
) -> Result<&mut Vec<serde_json::Value>, AdapterError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| AdapterError::new("Pi settings must be a JSON object"))?;
    if object.contains_key("disabledSkills") {
        return object
            .get_mut("disabledSkills")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| AdapterError::new("Pi disabledSkills must be an array"));
    }
    let skills = object
        .entry("skills")
        .or_insert_with(|| serde_json::json!({}));
    let skills_obj = skills
        .as_object_mut()
        .ok_or_else(|| AdapterError::new("Pi skills settings must be a JSON object"))?;
    let disabled = skills_obj
        .entry("disabled")
        .or_insert_with(|| serde_json::json!([]));
    disabled
        .as_array_mut()
        .ok_or_else(|| AdapterError::new("Pi skills.disabled must be an array"))
}

pub fn pi_disabled_skill_names(content: &str) -> Result<Vec<String>, AdapterError> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|err| AdapterError::new(format!("invalid Pi settings JSON: {err}")))?;
    let disabled = value
        .get("disabledSkills")
        .and_then(serde_json::Value::as_array)
        .or_else(|| {
            value
                .get("skills")
                .and_then(|skills| skills.get("disabled"))
                .and_then(serde_json::Value::as_array)
        });
    Ok(disabled
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AdapterRoot, RootSource};

    use super::*;

    #[test]
    fn yaml_contract_preserves_scalar_sequence_bool_and_nested_mapping() {
        let raw = "name: sample-skill\ndescription: Sample\nenabled: true\nallowed-tools:\n  - Read\n  - Search\nmetadata:\n  openclaw:\n    skillKey: routed-key\n";
        let value: serde_yaml::Value = serde_yaml::from_str(raw).expect("yaml parses");

        assert_eq!(
            value.get("name").and_then(serde_yaml::Value::as_str),
            Some("sample-skill")
        );
        assert_eq!(
            value.get("enabled").and_then(serde_yaml::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value
                .get("allowed-tools")
                .and_then(serde_yaml::Value::as_sequence)
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            value
                .get("metadata")
                .and_then(|item| item.get("openclaw"))
                .and_then(|item| item.get("skillKey"))
                .and_then(serde_yaml::Value::as_str),
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
    fn exposes_native_and_agent_compatibility_roots() {
        let adapter = PiAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/project")),
            project_cwd: Some(PathBuf::from("/tmp/project/nested/deeper")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from("/tmp/unverified"),
                source: RootSource::Extra,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots[0].path, PathBuf::from("/tmp/home/.pi/agent/skills"));
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(roots[1].path, PathBuf::from("/tmp/home/.agents/skills"));
        assert_eq!(roots[1].source, RootSource::Compatibility);
        assert_eq!(
            roots[2].path,
            PathBuf::from("/tmp/project/nested/deeper/.pi/skills")
        );
        assert_eq!(roots[2].source, RootSource::Project);
        assert_eq!(
            roots[3].path,
            PathBuf::from("/tmp/project/nested/deeper/.agents/skills")
        );
        assert_eq!(roots[3].source, RootSource::Compatibility);
        assert_eq!(
            roots[4].path,
            PathBuf::from("/tmp/project/nested/.pi/skills")
        );
        assert_eq!(
            roots[5].path,
            PathBuf::from("/tmp/project/nested/.agents/skills")
        );
        assert_eq!(roots[6].path, PathBuf::from("/tmp/project/.pi/skills"));
        assert_eq!(roots[7].path, PathBuf::from("/tmp/project/.agents/skills"));
        assert_eq!(roots.len(), 8);
    }

    #[test]
    fn parses_valid_directory_skill_frontmatter() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/global/agent/skills/global-pdf/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Pi);
        assert_eq!(skill.name, "global-pdf");
        assert_eq!(
            skill.description,
            "Extracts and reviews PDF text during Pi sessions. Use when a workflow needs PDF inspection steps."
        );
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/global/agent/skills/global-pdf/SKILL.md");

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_missing_description_as_broken() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/broken/missing-description/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "missing-description");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("description"));
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
