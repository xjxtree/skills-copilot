use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use skills_copilot_core::AdapterContext;

use super::CommandError;

pub const SCRIPT_EXECUTION_DISABLED_REASON: &str =
    "Script execution is disabled by default; the service will not spawn a process.";
pub const SCRIPT_EXECUTION_COMMAND_UNAVAILABLE_REASON: &str =
    "No verified script command was supplied; Agent Copilot will not infer or execute one.";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptExecutionInitiator {
    User,
    Llm,
}

impl ScriptExecutionInitiator {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Llm => "llm",
        }
    }
}

fn default_script_execution_initiator() -> ScriptExecutionInitiator {
    ScriptExecutionInitiator::User
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptExecutionRequest {
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    #[serde(alias = "instance_id")]
    pub skill_instance_id: Option<String>,
    #[serde(default = "default_script_execution_initiator")]
    pub initiated_by: ScriptExecutionInitiator,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionPreviewRecord {
    pub skill_instance_id: Option<String>,
    pub initiated_by: String,
    pub initiator_allowed: bool,
    pub cwd: ScriptExecutionCwdScope,
    pub env: ScriptExecutionEnvScope,
    pub network: ScriptExecutionNetworkScope,
    pub files: ScriptExecutionFilesScope,
    pub command_preview: ScriptExecutionCommandPreview,
    pub risks: Vec<String>,
    pub confirmation: ScriptExecutionConfirmation,
    pub execution_allowed: bool,
    pub disabled_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionCwdScope {
    pub requested: Option<String>,
    pub effective: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionEnvScope {
    pub inherit_parent: bool,
    pub provided_keys: Vec<String>,
    pub redacted_keys: Vec<String>,
    pub value_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionNetworkScope {
    pub requested: String,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionFilesScope {
    pub requested: Vec<String>,
    pub read_allowed: bool,
    pub write_allowed: bool,
    pub allowed_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionCommandPreview {
    pub argv: Vec<String>,
    pub display: String,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptExecutionConfirmation {
    pub required: bool,
    pub confirmed: bool,
    pub fields: Vec<String>,
    pub message: String,
}

pub fn preview_script_execution(
    ctx: &AdapterContext,
    request: &ScriptExecutionRequest,
) -> Result<ScriptExecutionPreviewRecord, CommandError> {
    let (effective_cwd, cwd_source) = effective_script_cwd(ctx, request.cwd.as_deref());
    let provided_keys: Vec<String> = request.env.keys().cloned().collect();
    let requested_network = request
        .network
        .as_deref()
        .filter(|network| !network.trim().is_empty())
        .unwrap_or("none")
        .to_string();
    let initiator_allowed = request.initiated_by != ScriptExecutionInitiator::Llm;
    let disabled_reason = if request.command.is_empty() {
        SCRIPT_EXECUTION_COMMAND_UNAVAILABLE_REASON
    } else {
        SCRIPT_EXECUTION_DISABLED_REASON
    };
    let mut risks = vec![disabled_reason.to_string()];
    if !initiator_allowed {
        risks.push("LLM-initiated script execution is rejected by the service.".to_string());
    }
    if requested_network != "none" {
        risks.push(
            "Network access was requested but is unavailable while execution is disabled."
                .to_string(),
        );
    }
    if !request.files.is_empty() {
        risks.push("File access was requested but no read or write roots are granted.".to_string());
    }
    if !provided_keys.is_empty() {
        risks.push("Environment values are accepted only for preview and audit metadata; values are redacted.".to_string());
    }

    Ok(ScriptExecutionPreviewRecord {
        skill_instance_id: request.skill_instance_id.clone(),
        initiated_by: request.initiated_by.as_str().to_string(),
        initiator_allowed,
        cwd: ScriptExecutionCwdScope {
            requested: request
                .cwd
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            effective: effective_cwd.to_string_lossy().to_string(),
            source: cwd_source.to_string(),
        },
        env: ScriptExecutionEnvScope {
            inherit_parent: false,
            redacted_keys: provided_keys.clone(),
            provided_keys,
            value_policy: "values-redacted".to_string(),
        },
        network: ScriptExecutionNetworkScope {
            requested: requested_network,
            allowed: false,
            reason: "Network access is not granted because script execution is disabled."
                .to_string(),
        },
        files: ScriptExecutionFilesScope {
            requested: request.files.clone(),
            read_allowed: false,
            write_allowed: false,
            allowed_roots: Vec::new(),
        },
        command_preview: ScriptExecutionCommandPreview {
            argv: request.command.clone(),
            display: display_argv(&request.command),
            shell: None,
        },
        risks,
        confirmation: ScriptExecutionConfirmation {
            required: true,
            confirmed: request.confirmed,
            fields: vec![
                "command_preview".to_string(),
                "cwd".to_string(),
                "env.provided_keys".to_string(),
                "network".to_string(),
                "files".to_string(),
                "initiated_by".to_string(),
            ],
            message: "Per-request user confirmation is required before any execution attempt."
                .to_string(),
        },
        execution_allowed: false,
        disabled_reason: disabled_reason.to_string(),
    })
}

fn effective_script_cwd(ctx: &AdapterContext, requested: Option<&Path>) -> (PathBuf, &'static str) {
    if let Some(path) = requested {
        if path.is_absolute() {
            return (path.to_path_buf(), "request");
        }
        let base = ctx
            .project_cwd
            .as_deref()
            .or(ctx.project_root.as_deref())
            .map(Path::to_path_buf)
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        return (base.join(path), "request-relative");
    }
    if let Some(path) = &ctx.project_cwd {
        return (path.clone(), "project_cwd");
    }
    if let Some(path) = &ctx.project_root {
        return (path.clone(), "project_root");
    }
    (
        env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        "process_cwd",
    )
}

fn display_argv(argv: &[String]) -> String {
    argv.iter()
        .map(|part| {
            if part.is_empty()
                || part
                    .chars()
                    .any(|ch| ch.is_whitespace() || matches!(ch, '\'' | '"' | '\\' | '$' | '`'))
            {
                format!("'{}'", part.replace('\'', "'\\''"))
            } else {
                part.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
