use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_commands::{
    action_descriptor, action_preview_binding, action_source_revision, ensure_action_confirmed,
    lock_app_mutations, ActionConfirmation, ActionPrecondition, ActionPreconditionKind,
    ActionPreviewBinding, ActionReadbackObservation, ActionReadbackRecord, CommandError,
};
use skills_copilot_core::{
    canonical_project_id, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef,
};

use crate::{
    create_private_dir_all, display_path, unix_timestamp_millis, write_private_bytes_file,
    ServiceError,
};

const PROJECT_CONTEXT_SCHEMA_VERSION: u32 = 1;
const PROJECT_CONTEXT_FILE: &str = "project-context.json";
const PROJECT_CONTEXT_TARGET_ID: &str = "project-context";
const PROJECT_RECENTS_TARGET_ID: &str = "project-context-recents";
const PROJECT_CONTEXT_MAX_BYTES: usize = 256 * 1024;
const MAX_RECENT_PROJECTS: usize = 12;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub current_cwd: String,
    pub last_used_at: i64,
    pub is_active: bool,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectContextState {
    pub revision: String,
    pub active: Option<ProjectContext>,
    pub recent: Vec<ProjectContext>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectContextSummary {
    pub source: &'static str,
    pub revision: String,
    pub active: Option<ProjectContext>,
    pub recent_count: usize,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextActionPreview {
    pub action: skills_copilot_core::ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
    pub current: ProjectContextState,
    pub candidate: ProjectContextState,
    pub affected_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextApplyResult {
    pub action: skills_copilot_core::ActionDescriptor,
    pub preview_token: String,
    pub state: ProjectContextState,
    pub affected_count: usize,
    pub readback: ActionReadbackRecord,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextParams {
    pub root_path: String,
    #[serde(default)]
    pub current_cwd: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextSetPreviewParams {
    #[serde(flatten)]
    pub context: ProjectContextParams,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextSetApplyParams {
    #[serde(flatten)]
    pub context: ProjectContextParams,
    pub candidate_last_used_at: i64,
    pub action_confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextRevisionParams {
    pub expected_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextConfirmationParams {
    pub action_confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextIDPreviewParams {
    pub id: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectContextIDApplyParams {
    pub id: String,
    pub action_confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectContextStore {
    schema_version: u32,
    active: Option<ProjectContext>,
    recent: Vec<ProjectContext>,
}

impl Default for ProjectContextStore {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_CONTEXT_SCHEMA_VERSION,
            active: None,
            recent: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ProjectContextStoreSnapshot {
    store: ProjectContextStore,
    revision: String,
}

#[derive(Debug, Clone)]
enum ProjectContextMutation {
    Set {
        context: ProjectContextParams,
        last_used_at: i64,
    },
    ClearActive,
    RemoveRecent {
        id: String,
    },
    ClearRecent,
}

struct PreparedProjectContextMutation {
    preview: ProjectContextActionPreview,
    candidate_bytes: Vec<u8>,
}

pub fn project_context_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PROJECT_CONTEXT_FILE)
}

pub fn load_project_context_state(
    app_data_dir: &Path,
) -> Result<ProjectContextState, ServiceError> {
    let snapshot = load_store_snapshot(app_data_dir)?;
    Ok(snapshot.state())
}

pub fn preview_set_project_context(
    app_data_dir: &Path,
    params: ProjectContextSetPreviewParams,
) -> Result<ProjectContextActionPreview, ServiceError> {
    prepare_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::Set {
            context: params.context,
            last_used_at: unix_timestamp_millis(),
        },
        Some(&params.expected_revision),
    )
    .map(|prepared| prepared.preview)
}

pub fn set_project_context(
    app_data_dir: &Path,
    params: ProjectContextSetApplyParams,
) -> Result<ProjectContextApplyResult, ServiceError> {
    apply_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::Set {
            context: params.context,
            last_used_at: params.candidate_last_used_at,
        },
        &params.action_confirmation,
    )
}

pub fn preview_clear_project_context(
    app_data_dir: &Path,
    params: ProjectContextRevisionParams,
) -> Result<ProjectContextActionPreview, ServiceError> {
    prepare_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::ClearActive,
        Some(&params.expected_revision),
    )
    .map(|prepared| prepared.preview)
}

pub fn clear_project_context(
    app_data_dir: &Path,
    params: ProjectContextConfirmationParams,
) -> Result<ProjectContextApplyResult, ServiceError> {
    apply_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::ClearActive,
        &params.action_confirmation,
    )
}

pub fn preview_remove_recent_project_context(
    app_data_dir: &Path,
    params: ProjectContextIDPreviewParams,
) -> Result<ProjectContextActionPreview, ServiceError> {
    prepare_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::RemoveRecent { id: params.id },
        Some(&params.expected_revision),
    )
    .map(|prepared| prepared.preview)
}

pub fn remove_recent_project_context(
    app_data_dir: &Path,
    params: ProjectContextIDApplyParams,
) -> Result<ProjectContextApplyResult, ServiceError> {
    apply_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::RemoveRecent { id: params.id },
        &params.action_confirmation,
    )
}

pub fn preview_clear_recent_project_contexts(
    app_data_dir: &Path,
    params: ProjectContextRevisionParams,
) -> Result<ProjectContextActionPreview, ServiceError> {
    prepare_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::ClearRecent,
        Some(&params.expected_revision),
    )
    .map(|prepared| prepared.preview)
}

pub fn clear_recent_project_contexts(
    app_data_dir: &Path,
    params: ProjectContextConfirmationParams,
) -> Result<ProjectContextApplyResult, ServiceError> {
    apply_project_context_mutation(
        app_data_dir,
        ProjectContextMutation::ClearRecent,
        &params.action_confirmation,
    )
}

fn apply_project_context_mutation(
    app_data_dir: &Path,
    mutation: ProjectContextMutation,
    confirmation: &ActionConfirmation,
) -> Result<ProjectContextApplyResult, ServiceError> {
    let preflight = prepare_project_context_mutation(app_data_dir, mutation.clone(), None)?;
    ensure_action_confirmed(&preview_binding(&preflight.preview), Some(confirmation))?;

    create_private_dir_all(app_data_dir)?;
    let _owner = lock_app_mutations(app_data_dir)?;
    let locked = prepare_project_context_mutation(app_data_dir, mutation, None)?;
    ensure_action_confirmed(&preview_binding(&locked.preview), Some(confirmation))?;

    let original_revision = locked.preview.current.revision.clone();
    let candidate_revision = locked.preview.candidate.revision.clone();
    if let Err(write_error) =
        write_private_bytes_file(&project_context_path(app_data_dir), &locked.candidate_bytes)
    {
        match load_store_snapshot(app_data_dir) {
            Ok(current) if current.revision == candidate_revision => {}
            Ok(current) if current.revision == original_revision => {
                return Err(ServiceError::Io(write_error));
            }
            _ => {
                return Err(CommandError::PartialEffect {
                    operation: "project context".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: false,
                    detail:
                        "project context persistence failed after the outcome became unverifiable"
                            .to_string(),
                }
                .into());
            }
        }
    }

    let readback_state = load_store_snapshot(app_data_dir)?.state();
    if readback_state.revision != candidate_revision {
        return Err(CommandError::PartialEffect {
            operation: "project context".to_string(),
            state: "applied_unverified",
            cleanup_required: false,
            detail: "project context bytes did not match the confirmed candidate".to_string(),
        }
        .into());
    }
    let readback = ActionReadbackRecord::verified(
        &locked.preview.action,
        vec![ActionReadbackObservation {
            domain: ActionReadbackDomain::ProjectContext,
            target_id: PROJECT_CONTEXT_TARGET_ID.to_string(),
            revision: readback_state.revision.clone(),
        }],
    )?;
    Ok(ProjectContextApplyResult {
        action: locked.preview.action,
        preview_token: locked.preview.preview_token,
        state: readback_state,
        affected_count: locked.preview.affected_count,
        readback,
    })
}

fn prepare_project_context_mutation(
    app_data_dir: &Path,
    mutation: ProjectContextMutation,
    expected_revision: Option<&str>,
) -> Result<PreparedProjectContextMutation, ServiceError> {
    let current = load_store_snapshot(app_data_dir)?;
    if expected_revision.is_some_and(|expected| expected != current.revision) {
        return Err(CommandError::StaleActionReference.into());
    }

    let (mut candidate_store, affected_count) = candidate_store(&current.store, &mutation)
        .map_err(|error| match error {
            CommandError::UnsafeConfigPath(detail) => ServiceError::InvalidRequest(detail),
            error => ServiceError::Command(error),
        })?;
    normalize_store(&mut candidate_store);
    let candidate_bytes = serialize_store(&candidate_store)?;
    let candidate_revision = project_context_revision(true, &candidate_bytes);
    if candidate_revision == current.revision {
        return Err(CommandError::NoApplicableAction(
            "the project context mutation would not change app-local state".to_string(),
        )
        .into());
    }

    let (intent, preview_method, apply_method, target, project_id, operation) =
        mutation.action_shape(&candidate_store)?;
    let source_revision = action_source_revision(
        operation,
        &[
            ("project_context_revision", &current.revision),
            ("candidate_revision", &candidate_revision),
            ("affected_count", &affected_count.to_string()),
        ],
    )?;
    let action = action_descriptor(
        ActionKind::ProjectContext,
        intent,
        target,
        project_id,
        vec![ActionImpact::AppLocalData],
        preview_method,
        Some(apply_method),
        source_revision,
        true,
        ActionNetworkPosture::None,
        vec![ActionReadbackDomain::ProjectContext],
        vec![format!("project-context-state:{}", current.revision)],
    )?;
    let binding = action_preview_binding(
        action,
        vec![ActionPrecondition {
            kind: ActionPreconditionKind::ProjectContext,
            target_id: PROJECT_CONTEXT_TARGET_ID.to_string(),
            expected_revision: current.revision.clone(),
        }],
    )?;
    Ok(PreparedProjectContextMutation {
        preview: ProjectContextActionPreview {
            action: binding.action,
            preconditions: binding.preconditions,
            preview_token: binding.preview_token,
            current: current.state(),
            candidate: state_from_store(&candidate_store, candidate_revision),
            affected_count,
        },
        candidate_bytes,
    })
}

fn preview_binding(preview: &ProjectContextActionPreview) -> ActionPreviewBinding {
    ActionPreviewBinding {
        action: preview.action.clone(),
        preconditions: preview.preconditions.clone(),
        preview_token: preview.preview_token.clone(),
    }
}

fn candidate_store(
    current: &ProjectContextStore,
    mutation: &ProjectContextMutation,
) -> Result<(ProjectContextStore, usize), CommandError> {
    let mut candidate = current.clone();
    match mutation {
        ProjectContextMutation::Set {
            context,
            last_used_at,
        } => {
            if *last_used_at <= 0 {
                return Err(CommandError::MismatchedActionReference(
                    "project candidate timestamp is invalid".to_string(),
                ));
            }
            let mut context = validate_project_context(context.clone())
                .map_err(CommandError::UnsafeConfigPath)?;
            context.is_active = true;
            context.last_used_at = *last_used_at;
            candidate.active = Some(context.clone());
            candidate.recent.retain(|recent| recent.id != context.id);
            candidate.recent.insert(0, context);
            candidate.recent.truncate(MAX_RECENT_PROJECTS);
            Ok((candidate, 1))
        }
        ProjectContextMutation::ClearActive => {
            if candidate.active.is_none() {
                return Err(CommandError::NoApplicableAction(
                    "there is no active project to clear".to_string(),
                ));
            }
            candidate.active = None;
            Ok((candidate, 1))
        }
        ProjectContextMutation::RemoveRecent { id } => {
            let id = id.trim();
            if id.is_empty() {
                return Err(CommandError::MismatchedActionReference(
                    "recent project id is required".to_string(),
                ));
            }
            let before = candidate.recent.len();
            candidate.recent.retain(|context| context.id != id);
            if candidate.recent.len() == before {
                return Err(CommandError::NoApplicableAction(
                    "the recent project is no longer present".to_string(),
                ));
            }
            let affected_count = before - candidate.recent.len();
            Ok((candidate, affected_count))
        }
        ProjectContextMutation::ClearRecent => {
            let affected_count = candidate.recent.len();
            if affected_count == 0 {
                return Err(CommandError::NoApplicableAction(
                    "there are no recent projects to clear".to_string(),
                ));
            }
            candidate.recent.clear();
            Ok((candidate, affected_count))
        }
    }
}

impl ProjectContextMutation {
    #[allow(clippy::type_complexity)]
    fn action_shape(
        &self,
        candidate: &ProjectContextStore,
    ) -> Result<
        (
            ActionIntent,
            &'static str,
            &'static str,
            ActionTargetRef,
            Option<String>,
            &'static str,
        ),
        ServiceError,
    > {
        let shape = match self {
            Self::Set { .. } => {
                let context = candidate.active.as_ref().ok_or_else(|| {
                    ServiceError::InvalidRequest(
                        "project set candidate is missing its active context".to_string(),
                    )
                })?;
                (
                    ActionIntent::SetProjectContext,
                    "project.previewSetContext",
                    "project.setContext",
                    ActionTargetRef {
                        kind: ActionTargetKind::Project,
                        id: context.id.clone(),
                        agent: None,
                        scope: None,
                    },
                    Some(context.id.clone()),
                    "project.context.set",
                )
            }
            Self::ClearActive => (
                ActionIntent::ClearProjectContext,
                "project.previewClearContext",
                "project.clearContext",
                ActionTargetRef {
                    kind: ActionTargetKind::Config,
                    id: PROJECT_CONTEXT_TARGET_ID.to_string(),
                    agent: None,
                    scope: None,
                },
                None,
                "project.context.clear",
            ),
            Self::RemoveRecent { id } => (
                ActionIntent::RemoveRecentProjectContext,
                "project.previewRemoveRecentContext",
                "project.removeRecentContext",
                ActionTargetRef {
                    kind: ActionTargetKind::Project,
                    id: id.trim().to_string(),
                    agent: None,
                    scope: None,
                },
                Some(id.trim().to_string()),
                "project.context.remove_recent",
            ),
            Self::ClearRecent => (
                ActionIntent::ClearRecentProjectContexts,
                "project.previewClearRecentContexts",
                "project.clearRecentContexts",
                ActionTargetRef {
                    kind: ActionTargetKind::Config,
                    id: PROJECT_RECENTS_TARGET_ID.to_string(),
                    agent: None,
                    scope: None,
                },
                candidate.active.as_ref().map(|context| context.id.clone()),
                "project.context.clear_recent",
            ),
        };
        Ok(shape)
    }
}

pub fn validate_project_context_for_response(params: ProjectContextParams) -> ProjectContext {
    match validate_project_context(params.clone()) {
        Ok(mut context) => {
            context.is_active = false;
            context
        }
        Err(message) => invalid_project_context(params, message),
    }
}

pub fn stored_active_context(app_data_dir: &Path) -> Result<Option<ProjectContext>, ServiceError> {
    Ok(load_store_snapshot(app_data_dir)?.store.active)
}

pub fn stored_active_adapter_paths(
    app_data_dir: &Path,
) -> Result<Option<(PathBuf, PathBuf)>, ServiceError> {
    let Some(active) = stored_active_context(app_data_dir)? else {
        return Ok(None);
    };
    let active = revalidate_stored_context(active);
    if active.validation_error.is_some() {
        return Ok(None);
    }
    Ok(Some((
        PathBuf::from(active.root_path),
        PathBuf::from(active.current_cwd),
    )))
}

pub fn project_context_summary(
    app_data_dir: &Path,
    env_context: Option<ProjectContext>,
) -> ProjectContextSummary {
    if let Some(active) = env_context {
        let revision = env_project_context_revision(&active);
        return ProjectContextSummary {
            source: "env",
            revision,
            active: Some(active),
            recent_count: load_store_snapshot(app_data_dir)
                .map(|snapshot| snapshot.store.recent.len())
                .unwrap_or(0),
            validation_error: None,
        };
    }

    match load_store_snapshot(app_data_dir) {
        Ok(snapshot) => {
            let revision = snapshot.revision.clone();
            let store = snapshot.store;
            let active = store.active.map(revalidate_stored_context);
            let validation_error = active
                .as_ref()
                .and_then(|context| context.validation_error.clone());
            ProjectContextSummary {
                source: if active.is_some() { "stored" } else { "none" },
                revision,
                active,
                recent_count: store.recent.len(),
                validation_error,
            }
        }
        Err(error) => ProjectContextSummary {
            source: "none",
            revision: project_context_revision(false, &[]),
            active: None,
            recent_count: 0,
            validation_error: Some(error.to_string()),
        },
    }
}

pub fn effective_project_context_revision(
    app_data_dir: &Path,
    env_context: Option<&ProjectContext>,
) -> Result<String, ServiceError> {
    Ok(match env_context {
        Some(context) => env_project_context_revision(context),
        None => load_store_snapshot(app_data_dir)?.revision,
    })
}

pub fn context_from_paths(root_path: &Path, current_cwd: &Path, is_active: bool) -> ProjectContext {
    let root_path = display_path(root_path);
    let current_cwd = display_path(current_cwd);
    ProjectContext {
        id: canonical_project_id(&root_path),
        name: default_project_name(Path::new(&root_path)),
        root_path,
        current_cwd,
        last_used_at: unix_timestamp_millis(),
        is_active,
        validation_error: None,
    }
}

fn validate_project_context(params: ProjectContextParams) -> Result<ProjectContext, String> {
    if params.root_path.trim().is_empty() {
        return Err("root_path is required".to_string());
    }

    let root_input = PathBuf::from(params.root_path.trim());
    let root = canonical_readable_dir(&root_input, "root_path")?;
    let cwd_input = params
        .current_cwd
        .as_ref()
        .map(|cwd| PathBuf::from(cwd.trim()))
        .filter(|cwd| !cwd.as_os_str().is_empty())
        .unwrap_or_else(|| root.clone());
    let cwd = canonical_readable_dir(&cwd_input, "current_cwd")?;
    if !cwd.starts_with(&root) {
        return Err("current_cwd must be under root_path after canonicalization".to_string());
    }

    let root_path = display_path(&root);
    let current_cwd = display_path(&cwd);
    let name = params
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_project_name(&root));
    Ok(ProjectContext {
        id: canonical_project_id(&root_path),
        name,
        root_path,
        current_cwd,
        last_used_at: unix_timestamp_millis(),
        is_active: false,
        validation_error: None,
    })
}

fn canonical_readable_dir(path: &Path, field: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("{field} is not a readable directory: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("{field} is not a readable directory: {error}"))?;
    if !metadata.is_dir() {
        return Err(format!("{field} is not a directory"));
    }
    fs::read_dir(&canonical)
        .map_err(|error| format!("{field} is not a readable directory: {error}"))?;
    Ok(canonical)
}

fn invalid_project_context(params: ProjectContextParams, message: String) -> ProjectContext {
    let root_path = params.root_path;
    let current_cwd = params.current_cwd.unwrap_or_else(|| root_path.clone());
    ProjectContext {
        id: if root_path.is_empty() {
            String::new()
        } else {
            canonical_project_id(&root_path)
        },
        name: params
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| default_project_name(Path::new(&root_path))),
        root_path,
        current_cwd,
        last_used_at: unix_timestamp_millis(),
        is_active: false,
        validation_error: Some(message),
    }
}

fn revalidate_stored_context(context: ProjectContext) -> ProjectContext {
    let params = ProjectContextParams {
        root_path: context.root_path.clone(),
        current_cwd: Some(context.current_cwd.clone()),
        name: Some(context.name.clone()),
    };
    match validate_project_context(params) {
        Ok(mut validated) => {
            validated.last_used_at = context.last_used_at;
            validated.is_active = context.is_active;
            validated
        }
        Err(message) => ProjectContext {
            validation_error: Some(message),
            ..context
        },
    }
}

fn load_store_snapshot(app_data_dir: &Path) -> Result<ProjectContextStoreSnapshot, ServiceError> {
    let path = project_context_path(app_data_dir);
    let content = match read_project_context_bytes(&path)? {
        Some(content) => content,
        None => {
            return Ok(ProjectContextStoreSnapshot {
                store: ProjectContextStore::default(),
                revision: project_context_revision(false, &[]),
            });
        }
    };
    let mut store: ProjectContextStore = serde_json::from_slice(&content)?;
    if store.schema_version != PROJECT_CONTEXT_SCHEMA_VERSION {
        return Err(ServiceError::InvalidRequest(format!(
            "unsupported project context schema version: {}",
            store.schema_version
        )));
    }
    normalize_store(&mut store);
    Ok(ProjectContextStoreSnapshot {
        store,
        revision: project_context_revision(true, &content),
    })
}

fn read_project_context_bytes(path: &Path) -> Result<Option<Vec<u8>>, ServiceError> {
    let mut file = match open_project_context_file(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(invalid_project_context_state()),
    };
    let metadata = file
        .metadata()
        .map_err(|_| invalid_project_context_state())?;
    if !metadata.is_file() || metadata.len() > PROJECT_CONTEXT_MAX_BYTES as u64 {
        return Err(invalid_project_context_state());
    }
    let mut content = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take((PROJECT_CONTEXT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut content)
        .map_err(|_| invalid_project_context_state())?;
    if content.len() > PROJECT_CONTEXT_MAX_BYTES {
        return Err(invalid_project_context_state());
    }
    Ok(Some(content))
}

#[cfg(unix)]
fn open_project_context_file(path: &Path) -> io::Result<fs::File> {
    use rustix::fs::{open, Mode, OFlags};

    let flags =
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
    open(path, flags, Mode::empty())
        .map(fs::File::from)
        .map_err(io::Error::from)
}

#[cfg(not(unix))]
fn open_project_context_file(path: &Path) -> io::Result<fs::File> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink"))
        }
        _ => fs::File::open(path),
    }
}

fn invalid_project_context_state() -> ServiceError {
    ServiceError::InvalidRequest("project context state is invalid".to_string())
}

fn serialize_store(store: &ProjectContextStore) -> Result<Vec<u8>, ServiceError> {
    let mut content = serde_json::to_vec_pretty(store)?;
    content.push(b'\n');
    Ok(content)
}

fn project_context_revision(present: bool, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/project-context/v1\0");
    hasher.update(if present {
        b"present\0".as_slice()
    } else {
        b"missing\0".as_slice()
    });
    hasher.update((content.len() as u64).to_be_bytes());
    hasher.update(content);
    format!("sha256:{:x}", hasher.finalize())
}

fn env_project_context_revision(context: &ProjectContext) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/project-context/env/v1\0");
    for value in [&context.id, &context.root_path, &context.current_cwd] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn normalize_store(store: &mut ProjectContextStore) {
    let active_id = store.active.as_ref().map(|context| context.id.clone());
    if let Some(active) = &mut store.active {
        active.is_active = true;
        active.validation_error = None;
    }
    for recent in &mut store.recent {
        recent.is_active = active_id.as_ref() == Some(&recent.id);
        recent.validation_error = None;
    }
}

impl ProjectContextStoreSnapshot {
    fn state(&self) -> ProjectContextState {
        state_from_store(&self.store, self.revision.clone())
    }
}

fn state_from_store(store: &ProjectContextStore, revision: String) -> ProjectContextState {
    ProjectContextState {
        revision,
        active: store.active.clone().map(revalidate_stored_context),
        recent: store
            .recent
            .iter()
            .cloned()
            .map(revalidate_stored_context)
            .collect(),
    }
}

fn default_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Project")
        .to_string()
}
