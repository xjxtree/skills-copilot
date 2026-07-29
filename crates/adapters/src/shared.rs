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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_split_accepts_lf_and_crlf_closing_delimiters() {
        assert_eq!(
            split_yaml_frontmatter("name: sample\n---\nBody."),
            Ok(("name: sample", "Body.".to_string()))
        );
        assert_eq!(
            split_yaml_frontmatter("name: sample\r\n---\r\nBody."),
            Ok(("name: sample\r", "Body.".to_string()))
        );
    }

    #[test]
    fn frontmatter_split_accepts_an_empty_body_without_fabricating_content() {
        assert_eq!(
            split_yaml_frontmatter("name: sample\n---"),
            Ok(("name: sample", String::new()))
        );
        assert!(split_yaml_frontmatter("name: sample\n----").is_err());
    }

    #[test]
    fn required_frontmatter_strings_trim_values_and_reject_blank_or_non_string_fields() {
        let value: serde_norway::Value =
            serde_norway::from_str("name: \"  review  \"\nblank: \"  \"\ncount: 3\n")
                .expect("fixture parses");

        assert_eq!(
            required_frontmatter_string(&value, "name", "fixture"),
            Ok("review".to_string())
        );
        assert!(required_frontmatter_string(&value, "blank", "fixture").is_err());
        assert!(required_frontmatter_string(&value, "count", "fixture").is_err());
        assert!(required_frontmatter_string(&value, "missing", "fixture").is_err());
    }

    #[test]
    fn kebab_skill_name_validation_covers_length_and_separator_boundaries() {
        for valid in ["a", "review", "review-2", &"a".repeat(64)] {
            assert!(
                validate_kebab_skill_name(valid, "fixture").is_ok(),
                "expected valid name: {valid}"
            );
        }
        for invalid in [
            "",
            "-review",
            "review-",
            "review--tool",
            "Review",
            "review_tool",
            &"a".repeat(65),
        ] {
            assert!(
                validate_kebab_skill_name(invalid, "fixture").is_err(),
                "expected invalid name: {invalid}"
            );
        }
    }
}
