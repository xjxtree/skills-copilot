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
mod dispatch_fixtures;
mod llm_provider;
mod local_session_preview;
mod local_session_project_scope;
mod protocol_fixtures;
mod skill_manager_fixtures;
mod support_and_status;
mod support_seed;

use support_and_status::EnvVarGuard;
use support_seed::*;

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
