use std::path::{Path, PathBuf};

use crate::{AgentId, Scope, SkillInstance};

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn display_name(&self) -> &'static str;
    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot>;
    fn link_target_roots(&self, _ctx: &AdapterContext) -> Vec<AdapterRoot> {
        Vec::new()
    }
    /// Returns whether scanner traversal may enter a directory below one of
    /// this adapter's declared roots. Adapters may further narrow traversal,
    /// while the default rejects well-known generated, cache, quarantine, and
    /// VCS directories that are never authoritative skill sources.
    fn should_descend(&self, _root: &AdapterRoot, relative_dir: &Path) -> bool {
        !relative_dir.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(
                    ".git"
                        | ".svn"
                        | ".hg"
                        | ".cache"
                        | "cache"
                        | "caches"
                        | "tmp"
                        | "temp"
                        | "dist"
                        | "build"
                        | "target"
                        | "out"
                        | "coverage"
                        | "__pycache__"
                        | ".hub"
                        | "quarantine"
                        | "archive"
                        | "archives"
                )
            )
        })
    }
    /// Returns whether an otherwise valid `SKILL.md` has the documented shape
    /// for this adapter and root. This keeps source-specific layout semantics
    /// out of the generic filesystem walker.
    fn accepts_skill_path(&self, _root: &AdapterRoot, _relative_path: &Path) -> bool {
        true
    }
    /// Returns whether a regular file can represent a skill entry. Most
    /// adapters use directory-based `SKILL.md`; Pi also supports top-level
    /// Markdown skill files in selected roots.
    fn is_skill_file(&self, path: &Path) -> bool {
        path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
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
    /// Adapter-owned logical provenance. It must not contain a physical path
    /// or be reconstructed from one. `None` means product projection identity
    /// is unavailable for instances discovered through this root.
    pub logical_source_id: Option<String>,
}

/// Encodes a manifest/config-owned logical identifier without treating a
/// filesystem path as provenance.
pub fn adapter_logical_source_token(namespace: &str, logical_id: &str) -> Option<String> {
    let logical_id = logical_id.trim();
    if namespace.is_empty()
        || logical_id.is_empty()
        || logical_id.len() > 512
        || logical_id.chars().any(char::is_control)
    {
        return None;
    }
    let mut encoded = String::with_capacity(logical_id.len().saturating_mul(2));
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in logical_id.as_bytes() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Some(format!("{namespace}:hex:{encoded}"))
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
