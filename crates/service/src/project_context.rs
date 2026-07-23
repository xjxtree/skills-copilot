use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use skills_copilot_core::canonical_project_id;

use crate::{display_path, unix_timestamp_millis, write_private_bytes_file, ServiceError};

const PROJECT_CONTEXT_SCHEMA_VERSION: u32 = 1;
const PROJECT_CONTEXT_FILE: &str = "project-context.json";
const MAX_RECENT_PROJECTS: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub current_cwd: String,
    pub last_used_at: i64,
    pub is_active: bool,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextState {
    pub active: Option<ProjectContext>,
    pub recent: Vec<ProjectContext>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectContextSummary {
    pub source: &'static str,
    pub active: Option<ProjectContext>,
    pub recent_count: usize,
    pub validation_error: Option<String>,
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
pub struct ProjectContextIDParams {
    pub id: String,
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

pub fn project_context_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PROJECT_CONTEXT_FILE)
}

pub fn load_project_context_state(
    app_data_dir: &Path,
) -> Result<ProjectContextState, ServiceError> {
    let store = load_store(app_data_dir)?;
    Ok(store.into_state())
}

pub fn set_project_context(
    app_data_dir: &Path,
    params: ProjectContextParams,
) -> Result<ProjectContextState, ServiceError> {
    let mut context = validate_project_context(params).map_err(ServiceError::InvalidRequest)?;
    context.is_active = true;
    let mut store = load_store(app_data_dir)?;
    store.active = Some(context.clone());
    store.recent.retain(|recent| recent.id != context.id);
    store.recent.insert(0, context);
    store.recent.truncate(MAX_RECENT_PROJECTS);
    normalize_store(&mut store);
    save_store(app_data_dir, &store)?;
    Ok(store.into_state())
}

pub fn clear_project_context(app_data_dir: &Path) -> Result<ProjectContextState, ServiceError> {
    let mut store = load_store(app_data_dir)?;
    store.active = None;
    for recent in &mut store.recent {
        recent.is_active = false;
        recent.validation_error = None;
    }
    save_store(app_data_dir, &store)?;
    Ok(store.into_state())
}

pub fn remove_recent_project_context(
    app_data_dir: &Path,
    params: ProjectContextIDParams,
) -> Result<ProjectContextState, ServiceError> {
    let id = params.id.trim();
    if id.is_empty() {
        return Err(ServiceError::InvalidRequest("id is required".to_string()));
    }

    let mut store = load_store(app_data_dir)?;
    store.recent.retain(|context| context.id != id);
    normalize_store(&mut store);
    save_store(app_data_dir, &store)?;
    Ok(store.into_state())
}

pub fn clear_recent_project_contexts(
    app_data_dir: &Path,
) -> Result<ProjectContextState, ServiceError> {
    let mut store = load_store(app_data_dir)?;
    store.recent.clear();
    normalize_store(&mut store);
    save_store(app_data_dir, &store)?;
    Ok(store.into_state())
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
    Ok(load_store(app_data_dir)?.active)
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
        return ProjectContextSummary {
            source: "env",
            active: Some(active),
            recent_count: load_store(app_data_dir)
                .map(|store| store.recent.len())
                .unwrap_or(0),
            validation_error: None,
        };
    }

    match load_store(app_data_dir) {
        Ok(store) => {
            let active = store.active.map(revalidate_stored_context);
            let validation_error = active
                .as_ref()
                .and_then(|context| context.validation_error.clone());
            ProjectContextSummary {
                source: if active.is_some() { "stored" } else { "none" },
                active,
                recent_count: store.recent.len(),
                validation_error,
            }
        }
        Err(error) => ProjectContextSummary {
            source: "none",
            active: None,
            recent_count: 0,
            validation_error: Some(error.to_string()),
        },
    }
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

fn load_store(app_data_dir: &Path) -> Result<ProjectContextStore, ServiceError> {
    let path = project_context_path(app_data_dir);
    if !path.exists() {
        return Ok(ProjectContextStore::default());
    }
    let content = fs::read_to_string(path)?;
    let mut store: ProjectContextStore = serde_json::from_str(&content)?;
    if store.schema_version != PROJECT_CONTEXT_SCHEMA_VERSION {
        return Err(ServiceError::InvalidRequest(format!(
            "unsupported project context schema version: {}",
            store.schema_version
        )));
    }
    normalize_store(&mut store);
    Ok(store)
}

fn save_store(app_data_dir: &Path, store: &ProjectContextStore) -> Result<(), ServiceError> {
    let path = project_context_path(app_data_dir);
    let mut content = serde_json::to_vec_pretty(store)?;
    content.push(b'\n');
    write_private_bytes_file(&path, &content)?;
    Ok(())
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

impl ProjectContextStore {
    fn into_state(mut self) -> ProjectContextState {
        self.active = self.active.map(revalidate_stored_context);
        self.recent = self
            .recent
            .into_iter()
            .map(revalidate_stored_context)
            .collect();
        ProjectContextState {
            active: self.active,
            recent: self.recent,
        }
    }
}

fn default_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Project")
        .to_string()
}
