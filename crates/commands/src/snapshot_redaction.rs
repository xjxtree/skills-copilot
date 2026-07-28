use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const REDACTED_SNAPSHOT_PREFIX: &str = "# skills-copilot: snapshot content redacted\n";
pub(crate) const REDACTED_VALUE: &str = "[REDACTED]";

pub(crate) fn redact_snapshot_content(content: &str) -> String {
    if content.is_empty() || is_redacted_snapshot_content(content) {
        return content.to_string();
    }

    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(content) {
        if redact_json_value(&mut value) {
            let rendered =
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string());
            return format!("{REDACTED_SNAPSHOT_PREFIX}{rendered}\n");
        }
        return content.to_string();
    }

    if let Ok(mut value) = json5::from_str::<serde_json::Value>(content) {
        if redact_json_value(&mut value) {
            let rendered =
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string());
            return format!("{REDACTED_SNAPSHOT_PREFIX}{rendered}\n");
        }
        return content.to_string();
    }

    let redacted = content
        .lines()
        .map(redact_simple_secret_line)
        .collect::<Vec<_>>()
        .join("\n");
    let redacted = if content.ends_with('\n') {
        format!("{redacted}\n")
    } else {
        redacted
    };
    if redacted == content {
        content.to_string()
    } else {
        format!("{REDACTED_SNAPSHOT_PREFIX}{redacted}")
    }
}

pub(crate) fn is_redacted_snapshot_content(content: &str) -> bool {
    content.starts_with(REDACTED_SNAPSHOT_PREFIX)
}

fn redact_json_value(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let mut changed = false;
            for (key, value) in map {
                if is_sensitive_key(key) {
                    if !matches!(
                        value,
                        serde_json::Value::String(redacted) if redacted == REDACTED_VALUE
                    ) {
                        *value = serde_json::Value::String(REDACTED_VALUE.to_string());
                        changed = true;
                    }
                } else {
                    changed |= redact_json_value(value);
                }
            }
            changed
        }
        serde_json::Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= redact_json_value(value);
            }
            changed
        }
        _ => false,
    }
}

fn redact_simple_secret_line(line: &str) -> String {
    let Some((key_part, value_part, separator)) = split_assignment_line(line) else {
        return line.to_string();
    };
    let key = key_part.trim().trim_matches('"').trim_matches('\'');
    if !is_sensitive_key(key) {
        return line.to_string();
    }
    let trailing_comma = value_part.trim_end().ends_with(',');
    let comment = value_part.find('#').map(|idx| &value_part[idx..]);
    let suffix = match (trailing_comma, comment) {
        (true, Some(comment)) => format!(", {comment}"),
        (true, None) => ",".to_string(),
        (false, Some(comment)) => format!(" {comment}"),
        (false, None) => String::new(),
    };
    format!("{key_part}{separator} \"{REDACTED_VALUE}\"{suffix}")
}

fn split_assignment_line(line: &str) -> Option<(&str, &str, &'static str)> {
    if let Some((key, value)) = line.split_once('=') {
        return Some((key, value, "="));
    }
    line.split_once(':').map(|(key, value)| (key, value, ":"))
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    matches!(
        normalized.as_str(),
        "apikey"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "secret"
            | "clientsecret"
            | "password"
            | "passwd"
    ) || normalized.ends_with("token")
        || normalized.ends_with("apikey")
        || normalized.ends_with("secret")
        || normalized.ends_with("password")
}

pub(crate) fn generate_snapshot_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("snap-{nanos:x}")
}

pub(crate) fn current_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
