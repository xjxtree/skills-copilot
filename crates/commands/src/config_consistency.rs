use crate::{config_support::scope_from_snapshot, project_record_matches_context, CommandError};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::ConfigSnapshotRecord;
use skills_copilot_core::{AdapterContext, Scope};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ConfigState {
    pub exists: bool,
    pub content: String,
    pub revision: String,
}

pub(crate) fn config_revision(exists: bool, content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(if exists { b"present\0" } else { b"missing\0" });
    if exists {
        digest.update(content.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

pub(crate) fn config_content_digest(content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"agent-copilot/config-content/v1\0");
    digest.update(content.as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

pub(crate) fn snapshot_binding_revision(snapshot: &ConfigSnapshotRecord) -> String {
    let mut digest = Sha256::new();
    digest.update(b"agent-copilot/config-snapshot/v1\0");
    digest.update(snapshot.id.as_bytes());
    digest.update(b"\0");
    digest.update(snapshot.agent.as_bytes());
    digest.update(b"\0");
    digest.update(snapshot.scope.as_bytes());
    digest.update(b"\0");
    digest.update(
        snapshot
            .project_root
            .as_deref()
            .unwrap_or_default()
            .as_bytes(),
    );
    digest.update(b"\0");
    digest.update(snapshot.target.as_bytes());
    digest.update(b"\0");
    digest.update(config_content_digest(&snapshot.content).as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

pub(crate) fn read_config_state(path: &Path) -> Result<ConfigState, CommandError> {
    let (exists, content) = match fs::read_to_string(path) {
        Ok(content) => (true, content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (false, String::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(ConfigState {
        exists,
        revision: config_revision(exists, &content),
        content,
    })
}

pub(crate) fn ensure_expected_revision(
    expected: &str,
    actual: &ConfigState,
) -> Result<(), CommandError> {
    if expected == actual.revision {
        Ok(())
    } else {
        Err(CommandError::ConfigConflict {
            expected: expected.to_string(),
            actual: actual.revision.clone(),
        })
    }
}

pub(crate) fn canonical_snapshot_project_root(
    ctx: &AdapterContext,
) -> Result<Option<PathBuf>, CommandError> {
    ctx.project_root
        .as_deref()
        .map(|project_root| {
            project_root.canonicalize().map_err(|error| {
                CommandError::UnsafeConfigPath(format!(
                    "current project root {} cannot be canonicalized for snapshot isolation: {error}",
                    project_root.display()
                ))
            })
        })
        .transpose()
}

pub(crate) fn snapshot_project_root_for_scope(
    ctx: &AdapterContext,
    scope: Scope,
) -> Result<Option<String>, CommandError> {
    if scope != Scope::AgentProject {
        return Ok(None);
    }
    let project_root = canonical_snapshot_project_root(ctx)?.ok_or_else(|| {
        CommandError::UnsafeConfigPath(
            "project-scoped config snapshot requires an active project root".to_string(),
        )
    })?;
    Ok(Some(project_root.to_string_lossy().into_owned()))
}

pub(crate) fn validate_snapshot_project_binding(
    ctx: &AdapterContext,
    snapshot: &ConfigSnapshotRecord,
) -> Result<(), CommandError> {
    if scope_from_snapshot(&snapshot.scope)? != Scope::AgentProject {
        return Ok(());
    }
    let recorded = snapshot.project_root.as_deref().map(Path::new);
    if project_record_matches_context(recorded, ctx.project_root.as_deref()) {
        return Ok(());
    }
    Err(CommandError::UnsafeConfigPath(
        "project-scoped snapshot does not belong to the active project context".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_present_empty_files_have_different_revisions() {
        assert_ne!(config_revision(false, ""), config_revision(true, ""));
    }

    #[test]
    fn missing_revision_ignores_display_only_default_content() {
        assert_eq!(config_revision(false, ""), config_revision(false, "{}\n"));
    }

    #[test]
    fn revision_never_contains_config_content() {
        let content = ["private", "-", "value"].join("");
        let revision = config_revision(true, &content);
        assert!(revision.starts_with("sha256:"));
        assert!(!revision.contains(&content));
    }

    #[test]
    fn stale_revision_returns_config_conflict() {
        let actual = ConfigState {
            exists: true,
            content: "{}\n".to_string(),
            revision: config_revision(true, "{}\n"),
        };
        assert!(matches!(
            ensure_expected_revision("sha256:stale", &actual),
            Err(CommandError::ConfigConflict { .. })
        ));
    }

    #[test]
    fn content_digest_covers_all_candidate_bytes() {
        assert_ne!(
            config_content_digest("{\"value\":1}\n"),
            config_content_digest("{\"value\":1}")
        );
    }
}
