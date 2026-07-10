use std::path::Path;

pub(crate) fn split_yaml_frontmatter(rest: &str) -> Result<(&str, String), String> {
    if let Some((frontmatter, body)) = rest.split_once("\n---\n") {
        return Ok((frontmatter, body.to_string()));
    }
    if let Some((frontmatter, body)) = rest.split_once("\n---\r\n") {
        return Ok((frontmatter, body.to_string()));
    }
    if let Some(frontmatter) = rest.strip_suffix("\n---") {
        return Ok((frontmatter, String::new()));
    }
    if let Some(frontmatter) = rest.strip_suffix("\r\n---") {
        return Ok((frontmatter, String::new()));
    }
    Err("unterminated YAML frontmatter".to_string())
}

pub(crate) fn required_frontmatter_string(
    frontmatter: &serde_norway::Value,
    key: &str,
    adapter_label: &str,
) -> Result<String, String> {
    optional_frontmatter_string(frontmatter, key)
        .ok_or_else(|| format!("missing required {adapter_label} frontmatter field `{key}`"))
}

pub(crate) fn optional_frontmatter_string(
    frontmatter: &serde_norway::Value,
    key: &str,
) -> Option<String> {
    frontmatter
        .get(key)
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn validate_kebab_skill_name(name: &str, adapter_label: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!(
            "invalid {adapter_label} skill name `{name}`: must be 1-64 characters"
        ));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(format!(
            "invalid {adapter_label} skill name `{name}`: use single hyphen separators with no leading or trailing hyphen"
        ));
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!(
            "invalid {adapter_label} skill name `{name}`: use lowercase alphanumeric characters and hyphens only"
        ));
    }
    Ok(())
}

pub(crate) fn stable_path_id(agent: &str, path: &Path) -> String {
    format!("{agent}:{}", path.display())
}
