use super::*;

pub(crate) fn llm_prompt_run_record_sort(
    left: &LlmPromptRunRecord,
    right: &LlmPromptRunRecord,
) -> std::cmp::Ordering {
    right
        .completed_at
        .cmp(&left.completed_at)
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| left.action.cmp(&right.action))
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn generated_llm_prompt_run_id(
    preview_id: &str,
    confirmation_id: &str,
    completed_at: i64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(preview_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(confirmation_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(completed_at.to_string().as_bytes());
    let digest = hasher.finalize();
    format!("prompt-run-{}", hex_prefix(&digest, 12))
}

pub(crate) fn trace_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    hex_prefix(&digest, 16)
}

pub(crate) fn redact_normalized_string_list(
    values: Vec<String>,
    roots: &[(String, &'static str)],
) -> Vec<String> {
    let mut redactor = PromptRedactor::new(roots);
    normalize_string_list(
        values
            .into_iter()
            .map(|value| redactor.redact(&value))
            .collect(),
    )
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}
