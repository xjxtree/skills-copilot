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
mod dispatch_fixtures;
mod list_page_local_catalog;
mod llm_provider;
#[cfg(unix)]
mod local_session_inventory;
#[cfg(unix)]
mod local_session_preview;
#[cfg(unix)]
mod local_session_project_scope;
#[cfg(unix)]
mod local_session_summary_detail;
mod method_effects;
mod protocol_fixtures;
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
