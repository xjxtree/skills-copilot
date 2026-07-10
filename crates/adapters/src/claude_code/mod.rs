use std::path::{Path, PathBuf};

use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

#[derive(Debug, Default)]
pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn id(&self) -> AgentId {
        AgentId::ClaudeCode
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let mut roots = vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".claude/skills"),
            source: RootSource::UserHome,
        }];

        if let Some(project_root) = &ctx.project_root {
            roots.push(AdapterRoot {
                scope: Scope::AgentProject,
                path: project_root.join(".claude/skills"),
                source: RootSource::Project,
            });
        }

        roots.extend(ctx.extra_roots.clone());
        roots
    }

    fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".agents/skills"),
            source: RootSource::Compatibility,
        }]
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let display_name = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        let parsed = parse_skill_content(&content);
        let (frontmatter_raw, body, name, description, permissions, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name.unwrap_or_else(|| display_name.clone()),
                parsed.description,
                parsed.permissions,
                SkillState::Loaded,
                true,
            ),
            Err(_message) => (
                String::new(),
                content,
                display_name.clone(),
                String::new(),
                PermissionRequest::default(),
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id(path),
            agent: AgentId::ClaudeCode,
            scope: Scope::AgentProject,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name,
            description,
            version: None,
            state,
            enabled,
            frontmatter_raw,
            body,
            scripts: Vec::new(),
            permissions,
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
        let mut paths = vec![ctx.user_home.join(".claude/settings.json")];
        if let Some(project_root) = &ctx.project_root {
            paths.push(project_root.join(".claude/settings.local.json"));
        }
        paths
    }
}

impl AgentConfigAdapter for ClaudeCodeAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        let mut root = if doc.text.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&doc.text)
                .map_err(|err| AdapterError::new(format!("invalid Claude settings JSON: {err}")))?
        };

        let root_obj = root
            .as_object_mut()
            .ok_or_else(|| AdapterError::new("Claude settings must be a JSON object"))?;
        let overrides = root_obj
            .entry("skillOverrides")
            .or_insert_with(|| serde_json::json!({}));
        let overrides_obj = overrides
            .as_object_mut()
            .ok_or_else(|| AdapterError::new("skillOverrides must be a JSON object"))?;

        if on {
            overrides_obj.remove(&instance.name);
        } else {
            overrides_obj.insert(instance.name.clone(), serde_json::json!("off"));
        }

        doc.text = serde_json::to_string_pretty(&root)
            .map_err(|err| AdapterError::new(format!("failed to serialize settings: {err}")))?;
        doc.text.push('\n');
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: Option<String>,
    description: String,
    permissions: PermissionRequest,
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_frontmatter(rest)?;
    let frontmatter: serde_yaml::Value =
        serde_yaml::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = frontmatter
        .get("name")
        .and_then(serde_yaml::Value::as_str)
        .map(ToString::to_string);
    let description = frontmatter
        .get("description")
        .and_then(serde_yaml::Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| first_markdown_paragraph(&body));
    let permissions = PermissionRequest {
        tools: parse_allowed_tools(frontmatter.get("allowed-tools")),
        ..PermissionRequest::default()
    };

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        permissions,
    })
}

fn split_frontmatter(rest: &str) -> Result<(&str, String), String> {
    let mut line_start = 0;
    for line in rest.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let line_content = line_without_newline
            .strip_suffix('\r')
            .unwrap_or(line_without_newline);
        if line_content == "---" {
            let frontmatter = rest[..line_start]
                .strip_suffix("\r\n")
                .or_else(|| rest[..line_start].strip_suffix('\n'))
                .unwrap_or(&rest[..line_start]);
            return Ok((frontmatter, rest[line_start + line.len()..].to_string()));
        }
        line_start += line.len();
    }
    Err("unterminated YAML frontmatter".to_string())
}

fn parse_allowed_tools(value: Option<&serde_yaml::Value>) -> Vec<String> {
    match value {
        Some(serde_yaml::Value::String(raw)) => raw
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect(),
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .filter_map(serde_yaml::Value::as_str)
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn first_markdown_paragraph(body: &str) -> String {
    body.split("\n\n")
        .map(str::trim)
        .find(|part| !part.is_empty() && !part.starts_with('#'))
        .unwrap_or_default()
        .to_string()
}

fn stable_path_id(path: &Path) -> String {
    format!("claude-code:{}", path.display())
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AgentConfigDocument, ConfigFormat, SkillState};

    use super::*;

    #[test]
    fn parses_valid_skill_frontmatter() {
        let adapter = ClaudeCodeAdapter;
        let fixture = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.name, "summarize-changes");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.permissions.tools.iter().any(|tool| tool == "Read"));
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = ClaudeCodeAdapter;
        let fixture = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_missing_frontmatter_as_broken() {
        let adapter = ClaudeCodeAdapter;
        let fixture =
            fixture_path("fixtures/claude-code/project/broken-missing-frontmatter/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
    }

    #[test]
    fn parses_crlf_frontmatter() {
        let adapter = ClaudeCodeAdapter;
        let root = temp_test_root("skills-copilot-claude-crlf");
        let skill_dir = root.join("crlf-skill");
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\r\nname: crlf-skill\r\ndescription: CRLF frontmatter\r\nallowed-tools: Read\r\n---\r\nBody.\r\n",
        )
        .expect("write skill");

        let skill = adapter.parse(&skill_path).expect("skill parses");

        assert_eq!(skill.name, "crlf-skill");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
        assert!(skill.permissions.tools.iter().any(|tool| tool == "Read"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_garbage_frontmatter_closing_delimiter() {
        let adapter = ClaudeCodeAdapter;
        let root = temp_test_root("skills-copilot-claude-garbage-closing");
        let skill_dir = root.join("garbage-closing");
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: garbage-closing\ndescription: invalid delimiter\n---garbage\nBody.\n",
        )
        .expect("write skill");

        let skill = adapter
            .parse(&skill_path)
            .expect("broken skill is returned");

        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn patches_skill_overrides_off_and_on() {
        let adapter = ClaudeCodeAdapter;
        let mut doc = AgentConfigDocument {
            path: PathBuf::from(".claude/settings.local.json"),
            format: ConfigFormat::Json,
            text: "{}".to_string(),
        };
        let mut skill = adapter
            .parse(&fixture_path(
                "fixtures/claude-code/project/valid-review/SKILL.md",
            ))
            .expect("skill parses");
        skill.name = "review-pr".to_string();

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");
        assert!(doc.text.contains("\"review-pr\": \"off\""));

        adapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable patch succeeds");
        assert!(!doc.text.contains("review-pr"));
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn temp_test_root(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }
}
