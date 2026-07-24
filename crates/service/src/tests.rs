use super::*;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use skills_copilot_catalog::RuleFindingDraft;
use skills_copilot_core::{AgentId, PermissionRequest, Scope, SkillInstance, SkillState};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

mod app_wire_fixtures;
mod config_consistency;
mod contextual_intelligence;
mod dispatch_fixtures;
mod list_page_local_catalog;
mod llm_provider;
#[cfg(unix)]
mod local_session_codex_home;
#[cfg(unix)]
mod local_session_codex_wrappers;
#[cfg(unix)]
mod local_session_internal_filtering;
#[cfg(unix)]
mod local_session_inventory;
#[cfg(unix)]
mod local_session_message_paging;
#[cfg(unix)]
mod local_session_preview;
#[cfg(unix)]
mod local_session_project_scope;
#[cfg(unix)]
mod local_session_sqlite;
#[cfg(unix)]
mod local_session_summary_detail;
mod method_effects;
mod privacy_cleanup;
mod product_reads;
mod protocol_fixtures;
mod provider_action_lifecycle;
mod skill_manager_fixtures;
mod support_and_status;
mod support_seed;

use support_and_status::EnvVarGuard;
use support_seed::*;

#[cfg(unix)]
fn encoded_project_session_dir(project: &Path) -> String {
    project
        .to_string_lossy()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' => '-',
            other => other,
        })
        .collect()
}

fn json_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn confirmed_action_request(
    host: &ServiceHost,
    preview_method: &str,
    apply_method: &str,
    mut params: Value,
) -> (Value, ServiceResponse) {
    let preview = host.handle(ServiceRequest {
        id: Some(format!("{preview_method}-preview")),
        method: preview_method.to_string(),
        params: params.clone(),
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_result = preview.result.expect("action preview result");
    let action_confirmation = action_confirmation_from_preview(&preview_result);
    params
        .as_object_mut()
        .expect("action params object")
        .insert("action_confirmation".to_string(), action_confirmation);
    let response = host.handle(ServiceRequest {
        id: Some(format!("{apply_method}-apply")),
        method: apply_method.to_string(),
        params,
    });
    (preview_result, response)
}

fn confirmed_llm_prompt_request(
    host: &ServiceHost,
    request: Value,
    timeout_ms: u64,
) -> (Value, ServiceResponse) {
    let preview = host.handle(ServiceRequest {
        id: Some("llm-preview".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: request.clone(),
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_result = preview.result.expect("LLM prompt preview result");
    let response = host.handle(ServiceRequest {
        id: Some("llm-confirm".to_string()),
        method: "llm.confirmPromptAndSend".to_string(),
        params: json!({
            "action_confirmation": action_confirmation_from_preview(&preview_result),
            "request": request,
            "timeout_ms": timeout_ms
        }),
    });
    (preview_result, response)
}

fn action_confirmation_from_preview(preview_result: &Value) -> Value {
    let action = preview_result
        .get("action")
        .cloned()
        .expect("action descriptor");
    let preview_token = preview_result
        .get("preview_token")
        .and_then(Value::as_str)
        .expect("preview token")
        .to_string();
    json!({
        "reference": {
            "action_id": action.get("id").and_then(Value::as_str).expect("action id"),
            "source_revision": action
                .get("source_revision")
                .and_then(Value::as_str)
                .expect("action source revision"),
            "project_id": action.get("project_id").cloned().unwrap_or(Value::Null),
            "target": action.get("target").cloned().expect("action target")
        },
        "preview_token": preview_token,
        "confirmed": true
    })
}

fn project_context_state(host: &ServiceHost) -> Value {
    let response = host.handle(ServiceRequest {
        id: Some("project-context-state".to_string()),
        method: "project.getContext".to_string(),
        params: Value::Null,
    });
    assert!(response.ok, "{:?}", response.error);
    response.result.expect("project context state")
}

fn confirmed_project_set_context(host: &ServiceHost, params: Value) -> (Value, ServiceResponse) {
    let mut preview_params = params.clone();
    preview_params
        .as_object_mut()
        .expect("project set params object")
        .insert(
            "expected_revision".to_string(),
            project_context_state(host)
                .get("revision")
                .cloned()
                .expect("project context revision"),
        );
    let preview = host.handle(ServiceRequest {
        id: Some("project-set-preview".to_string()),
        method: "project.previewSetContext".to_string(),
        params: preview_params,
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_result = preview.result.expect("project set preview result");
    let mut apply_params = params;
    let apply = apply_params
        .as_object_mut()
        .expect("project set params object");
    apply.insert(
        "candidate_last_used_at".to_string(),
        preview_result
            .pointer("/candidate/active/last_used_at")
            .cloned()
            .expect("candidate project timestamp"),
    );
    apply.insert(
        "action_confirmation".to_string(),
        action_confirmation_from_preview(&preview_result),
    );
    let response = host.handle(ServiceRequest {
        id: Some("project-set-apply".to_string()),
        method: "project.setContext".to_string(),
        params: apply_params,
    });
    (preview_result, response)
}

fn confirmed_project_revision_action(
    host: &ServiceHost,
    preview_method: &str,
    apply_method: &str,
    params: Value,
) -> (Value, ServiceResponse) {
    let mut preview_params = params.clone();
    preview_params
        .as_object_mut()
        .expect("project action params object")
        .insert(
            "expected_revision".to_string(),
            project_context_state(host)
                .get("revision")
                .cloned()
                .expect("project context revision"),
        );
    let preview = host.handle(ServiceRequest {
        id: Some(format!("{preview_method}-preview")),
        method: preview_method.to_string(),
        params: preview_params,
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_result = preview.result.expect("project action preview result");
    let mut apply_params = params;
    apply_params
        .as_object_mut()
        .expect("project action params object")
        .insert(
            "action_confirmation".to_string(),
            action_confirmation_from_preview(&preview_result),
        );
    let response = host.handle(ServiceRequest {
        id: Some(format!("{apply_method}-apply")),
        method: apply_method.to_string(),
        params: apply_params,
    });
    (preview_result, response)
}
