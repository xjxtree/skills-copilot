use std::path::PathBuf;

use crate::{AgentId, Scope, SkillInstance};

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn display_name(&self) -> &'static str;
    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot>;
    fn link_target_roots(&self, _ctx: &AdapterContext) -> Vec<AdapterRoot> {
        Vec::new()
    }
    fn parse(&self, path: &std::path::Path) -> Result<SkillInstance, AdapterError>;
    fn parse_content(
        &self,
        path: &std::path::Path,
        content: String,
    ) -> Result<SkillInstance, AdapterError>;
    fn is_enabled(&self, instance: &SkillInstance) -> bool;
    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf>;
}

pub trait AgentConfigAdapter: Send + Sync {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdapterContext {
    pub user_home: PathBuf,
    /// Current project working directory for adapters whose discovery walks
    /// upward from cwd. `project_root` remains the safety boundary.
    pub project_cwd: Option<PathBuf>,
    pub project_root: Option<PathBuf>,
    pub extra_roots: Vec<AdapterRoot>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdapterRoot {
    pub scope: Scope,
    pub path: PathBuf,
    pub source: RootSource,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RootSource {
    UserHome,
    Project,
    Extra,
    Compatibility,
    Configured,
    Admin,
    Plugin,
    System,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentConfigDocument {
    pub path: PathBuf,
    pub format: ConfigFormat,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConfigFormat {
    Json,
    Toml,
    Yaml,
    Markdown,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdapterError {
    pub message: String,
}

impl AdapterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
