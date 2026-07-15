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

pub(crate) fn rollback_preview_token(
    snapshot: &ConfigSnapshotRecord,
    current_revision: &str,
) -> String {
    let mut snapshot_content = Sha256::new();
    snapshot_content.update(snapshot.content.as_bytes());
    let snapshot_content_hash = format!("{:x}", snapshot_content.finalize());
    let mut token = Sha256::new();
    token.update(b"snapshot-rollback-preview\0");
    token.update(snapshot.id.as_bytes());
    token.update(b"\0");
    token.update(snapshot.target.as_bytes());
    token.update(b"\0");
    token.update(
        snapshot
            .project_root
            .as_deref()
            .unwrap_or_default()
            .as_bytes(),
    );
    token.update(b"\0");
    token.update(snapshot_content_hash.as_bytes());
    token.update(b"\0");
    token.update(current_revision.as_bytes());
    format!("sha256:{:x}", token.finalize())
}

pub(crate) fn ensure_rollback_preview_token(
    provided: &str,
    snapshot: &ConfigSnapshotRecord,
    current_revision: &str,
) -> Result<(), CommandError> {
    let expected = rollback_preview_token(snapshot, current_revision);
    if provided == expected {
        Ok(())
    } else {
        Err(CommandError::StalePreviewToken)
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

    fn config_snapshot(id: &str, target: &str, content: &str) -> ConfigSnapshotRecord {
        ConfigSnapshotRecord {
            id: id.to_string(),
            agent: "claude-code".to_string(),
            scope: "agent-global".to_string(),
            project_root: None,
            target: target.to_string(),
            content: content.to_string(),
            reason: "test".to_string(),
            created_at: 1,
        }
    }

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
    fn rollback_token_changes_with_snapshot_id_target_content_or_current_revision() {
        let base = config_snapshot("snap-1", "/tmp/config.json", "{}\n");
        let base_token = rollback_preview_token(&base, "sha256:current-a");
        assert_ne!(
            base_token,
            rollback_preview_token(&base, "sha256:current-b")
        );
        assert_ne!(
            base_token,
            rollback_preview_token(
                &config_snapshot("snap-2", "/tmp/config.json", "{}\n"),
                "sha256:current-a",
            )
        );
        assert_ne!(
            base_token,
            rollback_preview_token(
                &config_snapshot("snap-1", "/tmp/other.json", "{}\n"),
                "sha256:current-a",
            )
        );
        assert_ne!(
            base_token,
            rollback_preview_token(
                &config_snapshot("snap-1", "/tmp/config.json", "{\"changed\":true}\n"),
                "sha256:current-a",
            )
        );
    }

    #[test]
    fn forged_rollback_token_is_rejected() {
        let snapshot = config_snapshot("snap-1", "/tmp/config.json", "{}\n");
        assert!(matches!(
            ensure_rollback_preview_token("sha256:forged", &snapshot, "sha256:current"),
            Err(CommandError::StalePreviewToken)
        ));
    }
}
