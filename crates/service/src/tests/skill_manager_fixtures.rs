use super::*;

pub(super) fn skill_manager_dispatch_params(method: &str) -> Value {
    match method {
        "skillManager.search" => {
            json!({ "query": "frontend", "owner": "vercel-labs", "network_allowed": false })
        }
        "skillManager.listInstalled" => json!({ "scope": "project" }),
        "skillManager.previewInstall" | "skillManager.applyInstall" => {
            json!({
                "source": "vercel-labs/agent-skills",
                "skills": ["frontend-design"],
                "agents": ["claude-code", "codex"],
                "scope": "project",
                "network_allowed": false,
                "confirmed": false
            })
        }
        "skillManager.previewRemove" | "skillManager.applyRemove" => {
            json!({ "skill": "frontend-design", "agents": ["claude-code", "codex"], "scope": "project", "confirmed": false })
        }
        "skillManager.previewUpdate" | "skillManager.applyUpdate" => {
            json!({
                "skills": ["frontend-design"],
                "scope": "project",
                "network_allowed": false,
                "confirmed": false
            })
        }
        "skillManager.previewLocalCreate" | "skillManager.applyLocalCreate" => {
            json!({ "name": "dispatch-local-skill", "confirmed": false })
        }
        "skillManager.deleteLocal" => {
            json!({ "instance_id": "missing-skill", "confirmed": false })
        }
        "skillManager.previewLocalArchiveImport" | "skillManager.applyLocalArchiveImport" => {
            json!({
                "archive_path": "/tmp/missing-local-skill.zip",
                "confirmed": false
            })
        }
        "skillManager.previewLocalArchiveUpdate" | "skillManager.applyLocalArchiveUpdate" => {
            json!({
                "instance_id": "missing-skill",
                "archive_path": "/tmp/missing-local-skill.zip",
                "confirmed": false
            })
        }
        _ => Value::Null,
    }
}

pub(super) fn assert_skill_manager_page_metadata(method: &str, result: &Value) {
    let returned_count = result
        .get("returned_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("{method} fixture missing returned_count"));
    let rows = match method {
        "skillManager.search" => result.get("results"),
        "skillManager.listInstalled" => result.get("installed"),
        _ => None,
    }
    .and_then(Value::as_array)
    .unwrap_or_else(|| panic!("{method} fixture missing result rows"));
    assert_eq!(returned_count as usize, rows.len());

    match method {
        "skillManager.search" => {
            assert!(result.get("total_count").is_some_and(Value::is_null));
            assert_eq!(
                result.get("source_completeness").and_then(Value::as_str),
                Some("unknown")
            );
            assert_eq!(
                result.get("incomplete_reason").and_then(Value::as_str),
                Some("source_limited")
            );
        }
        "skillManager.listInstalled" => {
            assert_eq!(
                result.get("total_count").and_then(Value::as_u64),
                Some(rows.len() as u64)
            );
            assert_eq!(
                result.get("source_completeness").and_then(Value::as_str),
                Some("enumerable")
            );
            assert!(result.get("incomplete_reason").is_none());
        }
        _ => {}
    }
}
