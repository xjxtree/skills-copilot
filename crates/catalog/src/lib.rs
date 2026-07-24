use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    convert::TryFrom,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OpenFlags, Row, Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use skills_copilot_core::{
    AgentId, NetworkAccess, PermissionRequest, Scope, SkillInstance, SkillState, SourceCoverage,
};
use thiserror::Error;

#[cfg(unix)]
mod anchored_vfs;
mod mapping;
mod queries;
mod refresh;
mod schema;

pub use mapping::{
    migration_count, ConfigSnapshotDraft, ConfigSnapshotRecord, ConflictGroupDraft,
    RuleFindingDraft, SkillDefinitionDraft, SkillEventDraft,
};

use mapping::*;

#[derive(Debug)]
pub struct Catalog {
    conn: Connection,
    // `conn` must be dropped before the registered VFS lease. Rust drops
    // struct fields in declaration order, so keep the lease after it.
    #[cfg(unix)]
    _anchored_vfs: Option<anchored_vfs::AnchoredVfsLease>,
    // Keep the target reservation until after the connection and VFS lease
    // close so path-open and descriptor-anchored connections cannot mix on
    // the same owner child.
    #[cfg(unix)]
    _open_safety: Option<anchored_vfs::CatalogOpenSafetyLease>,
    storage: CatalogStorageKind,
    fail_next_commit: Cell<bool>,
    fail_next_commit_outcome: Cell<bool>,
    fail_next_rollback: Cell<bool>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg(unix)]
struct UnixOwnerIdentity {
    device: u64,
    inode: u64,
}

#[derive(Debug)]
enum CatalogStorageKind {
    InMemory,
    #[cfg(unix)]
    Anchored(UnixOwnerIdentity),
    #[cfg(unix)]
    LegacyPathBacked(UnixOwnerIdentity),
    #[cfg(not(unix))]
    LegacyPathBacked,
}

/// Holds an immediate SQLite transaction on a catalog connection.
///
/// Commands use this guard when a filesystem mutation and its catalog update
/// must be protected from concurrent catalog writers. Dropping the guard
/// rolls the transaction back; callers must explicitly commit after their
/// read-back checks pass.
pub struct CatalogImmediateTransaction<'catalog> {
    transaction: Transaction<'catalog>,
    fail_commit: bool,
    fail_commit_outcome: bool,
    fail_rollback: bool,
}

#[derive(Debug, Error)]
pub enum CatalogCommitError {
    #[error("catalog commit was rejected before commit: {0}")]
    NotCommitted(CatalogError),
    #[error("catalog commit outcome is unknown: {0}")]
    OutcomeUnknown(CatalogError),
}

impl CatalogCommitError {
    pub fn outcome_unknown(&self) -> bool {
        matches!(self, Self::OutcomeUnknown(_))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SkillRecord {
    pub id: String,
    pub agent: String,
    pub scope: String,
    pub path: PathBuf,
    pub display_path: PathBuf,
    pub definition_id: String,
    pub name: String,
    pub state: String,
    pub enabled: bool,
    pub publisher: Option<String>,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub source_kind: Option<String>,
    pub read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SkillDetailRecord {
    pub id: String,
    pub agent: String,
    pub scope: String,
    pub path: PathBuf,
    pub display_path: PathBuf,
    pub definition_id: String,
    pub name: String,
    pub description: String,
    pub state: String,
    pub enabled: bool,
    pub frontmatter_raw: String,
    pub body: String,
    pub permissions: serde_json::Value,
    pub fingerprint: String,
    pub publisher: Option<String>,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub source_kind: Option<String>,
    pub read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct SkillSourceProvenance {
    publisher: Option<String>,
    package_name: Option<String>,
    package_version: Option<String>,
    source_kind: Option<String>,
    read_only_reason: Option<String>,
}

fn skill_source_provenance(agent: &str, path: &Path) -> SkillSourceProvenance {
    if agent != AgentId::Codex.as_str() {
        return SkillSourceProvenance::default();
    }
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    if components.contains(&".agent-copilot-runtime") {
        return SkillSourceProvenance {
            publisher: None,
            package_name: None,
            package_version: None,
            source_kind: Some("codex-runtime".to_string()),
            read_only_reason: Some(
                "Reported by the current Codex runtime; source cache paths are not scanned or persisted"
                    .to_string(),
            ),
        };
    }
    if let Some(cache_index) = components
        .windows(2)
        .position(|window| window == ["plugins", "cache"])
    {
        let package = cache_index + 2;
        let payload = components.get(package + 3..).unwrap_or_default();
        if payload.len() >= 2 && payload.last() == Some(&"SKILL.md") {
            let provenance = (
                components.get(package),
                components.get(package + 1),
                components.get(package + 2),
            );
            if let (Some(publisher), Some(package_name), Some(package_version)) = provenance {
                if [publisher, package_name, package_version]
                    .iter()
                    .all(|value| !value.is_empty() && !value.starts_with('.'))
                {
                    return SkillSourceProvenance {
                        publisher: Some((*publisher).to_string()),
                        package_name: Some((*package_name).to_string()),
                        package_version: Some((*package_version).to_string()),
                        source_kind: Some("chatgpt-plugin-cache".to_string()),
                        read_only_reason: Some(
                            "Installed Codex plugin files are read-only".to_string(),
                        ),
                    };
                }
            }
        }
    }

    SkillSourceProvenance::default()
}

fn is_ignored_current_skill_source(agent: &str, path: &Path) -> bool {
    skill_source_provenance(agent, path).source_kind.as_deref() == Some("codex-runtime")
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RuleFindingRecord {
    pub id: String,
    pub triage_key: String,
    pub triage_context: String,
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub rule_id: String,
    pub severity: String,
    pub effective_severity: String,
    pub severity_override: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
    pub created_at: i64,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
    pub suppression_note: Option<String>,
    pub rule_tuning_updated_at: Option<i64>,
    pub triage_status: String,
    pub triage_note: Option<String>,
    pub triage_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FindingTriageRecord {
    pub triage_key: String,
    pub triage_context: String,
    pub status: String,
    pub note: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RuleTuningRecord {
    pub rule_id: String,
    pub agent: Option<String>,
    pub scope: Option<String>,
    pub severity_override: Option<String>,
    pub suppression_reason: Option<String>,
    pub suppression_note: Option<String>,
    pub updated_at: i64,
}

struct RuleTuningUpdate<'a> {
    rule_id: &'a str,
    agent: Option<&'a str>,
    scope: Option<&'a str>,
    severity_override: Option<&'a str>,
    suppression_reason: Option<&'a str>,
    suppression_note: Option<&'a str>,
    updated_at: i64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SkillEventRecord {
    pub id: i64,
    pub instance_id: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CatalogPageSnapshot<Record> {
    pub records: Vec<Record>,
    pub source_revision: String,
    pub total_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CatalogScanRevisionRecord {
    pub generation: i64,
    pub revision: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CatalogScanCoverageRecord {
    pub agent: AgentId,
    pub context_revision: String,
    pub catalog_scan_generation: i64,
    pub catalog_scan_revision: String,
    pub coverage: SourceCoverage,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CatalogSkillProjectionDraft {
    pub instance_id: String,
    pub agent: AgentId,
    pub source_kind: String,
    pub source_identity: String,
    pub runtime_identity: String,
    pub linked: bool,
    pub precedence_proven: bool,
    pub coverage: SourceCoverage,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CatalogSkillProjectionRecord {
    pub instance_id: String,
    pub agent: AgentId,
    pub context_revision: String,
    pub catalog_scan_generation: i64,
    pub catalog_scan_revision: String,
    pub source_kind: String,
    pub source_identity: String,
    pub runtime_identity: String,
    pub linked: bool,
    pub precedence_proven: bool,
    pub coverage: SourceCoverage,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ConflictGroupRecord {
    pub id: String,
    pub definition_id: String,
    pub reason: String,
    pub winner_id: Option<String>,
    pub instance_ids: Vec<String>,
}

/// Slim view of a skill instance sufficient for config-patch operations
/// (e.g. toggling skillOverrides). Avoids materialising the full
/// `SkillInstance` (frontmatter, body, scripts) when only a few fields are
/// needed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillInstanceMeta {
    pub id: String,
    pub agent: AgentId,
    pub scope: Scope,
    pub project_root: Option<PathBuf>,
    pub path: PathBuf,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("list source changed during pagination")]
    SourceChanged,
    #[error("catalog schema migration did not reach the current version")]
    SchemaOutdated,
    #[error("injected catalog commit failure")]
    InjectedCommitFailure,
    #[error("injected catalog commit outcome uncertainty")]
    InjectedCommitOutcomeUnknown,
    #[error("injected catalog rollback failure")]
    InjectedRollbackFailure,
    #[error("descriptor-anchored catalog error: {0}")]
    AnchoredVfs(String),
    #[error("catalog mutation owner does not match catalog storage: {0}")]
    MutationOwner(String),
}

impl Catalog {
    /// Executes read-only queries against one stable SQLite snapshot.
    ///
    /// The catalog may have been opened read-only or writable by its caller;
    /// this boundary always uses a deferred transaction and never commits.
    pub fn with_read_snapshot<T, E>(
        &self,
        read: impl FnOnce(&Catalog) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<CatalogError>,
    {
        self.conn
            .execute_batch("BEGIN DEFERRED")
            .map_err(CatalogError::from)
            .map_err(E::from)?;
        let result = read(self);
        let rollback = self
            .conn
            .execute_batch("ROLLBACK")
            .map_err(CatalogError::from)
            .map_err(E::from);
        match (result, rollback) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn open(path: &Path) -> Result<Self, CatalogError> {
        let path = normalize_sqlite_root_alias(path);
        #[cfg(unix)]
        {
            Self::open_path_anchored(
                &path,
                OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            )
        }
        #[cfg(not(unix))]
        {
            let storage = legacy_path_storage(&path)?;
            Ok(Self {
                conn: Connection::open_with_flags(
                    &path,
                    OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW,
                )?,
                storage,
                fail_next_commit: Cell::new(false),
                fail_next_commit_outcome: Cell::new(false),
                fail_next_rollback: Cell::new(false),
            })
        }
    }

    pub fn open_read_only(path: &Path) -> Result<Self, CatalogError> {
        let path = normalize_sqlite_root_alias(path);
        #[cfg(unix)]
        {
            Self::open_path_anchored(
                &path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            )
        }
        #[cfg(not(unix))]
        {
            let storage = legacy_path_storage(&path)?;
            Ok(Self {
                conn: Connection::open_with_flags(
                    &path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
                )?,
                storage,
                fail_next_commit: Cell::new(false),
                fail_next_commit_outcome: Cell::new(false),
                fail_next_rollback: Cell::new(false),
            })
        }
    }

    #[cfg(unix)]
    fn open_path_anchored(path: &Path, flags: OpenFlags) -> Result<Self, CatalogError> {
        Self::open_path_anchored_with_hook(path, flags, || {})
    }

    #[cfg(unix)]
    fn open_path_anchored_with_hook(
        path: &Path,
        flags: OpenFlags,
        after_owner_open: impl FnOnce(),
    ) -> Result<Self, CatalogError> {
        use std::os::unix::ffi::OsStrExt;

        let child_name = path
            .file_name()
            .filter(|name| !name.as_bytes().is_empty())
            .ok_or_else(|| {
                CatalogError::AnchoredVfs("catalog path has no child filename".to_string())
            })?
            .to_owned();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let owner = std::fs::File::open(parent).map_err(|error| {
            CatalogError::MutationOwner(format!(
                "opening the legacy path-backed catalog parent failed: {error}"
            ))
        })?;
        let storage = CatalogStorageKind::LegacyPathBacked(unix_owner_identity(&owner)?);

        // Tests use this seam to prove that every later operation remains on
        // this one descriptor even if the display parent is rebound.
        after_owner_open();

        let open_safety =
            anchored_vfs::CatalogOpenSafetyLease::for_path_owner(&owner, child_name.as_bytes())
                .map_err(CatalogError::AnchoredVfs)?;
        let lease = anchored_vfs::AnchoredVfsLease::register_path_owner(owner)
            .map_err(CatalogError::AnchoredVfs)?;
        let conn =
            Connection::open_with_flags_and_vfs(Path::new(&child_name), flags, lease.name())?;
        Ok(Self {
            conn,
            _anchored_vfs: Some(lease),
            _open_safety: Some(open_safety),
            storage,
            fail_next_commit: Cell::new(false),
            fail_next_commit_outcome: Cell::new(false),
            fail_next_rollback: Cell::new(false),
        })
    }

    /// Open an existing catalog without performing or attempting migration.
    ///
    /// This is the pre-authorization path for actions whose rejected preview or
    /// confirmation must be byte-for-byte read-only. An outdated catalog fails
    /// closed instead of acquiring a writable connection.
    pub fn open_read_only_current(path: &Path) -> Result<Self, CatalogError> {
        let catalog = Self::open_read_only(path)?;
        if !schema::is_current(&catalog.conn)? {
            return Err(CatalogError::SchemaOutdated);
        }
        Ok(catalog)
    }

    /// Open an existing catalog for read-only service use, applying any
    /// required schema migration through a short-lived writable connection
    /// before returning the read-only handle.
    pub fn open_read_only_after_migration(path: &Path) -> Result<Self, CatalogError> {
        let catalog = Self::open_read_only(path)?;
        if schema::is_current(&catalog.conn)? {
            return Ok(catalog);
        }
        drop(catalog);

        let writable = Self::open(path)?;
        schema::init_schema(&writable.conn)?;
        drop(writable);

        let catalog = Self::open_read_only(path)?;
        if !schema::is_current(&catalog.conn)? {
            return Err(CatalogError::SchemaOutdated);
        }
        Ok(catalog)
    }

    pub fn in_memory() -> Result<Self, CatalogError> {
        Ok(Self {
            conn: Connection::open_in_memory()?,
            #[cfg(unix)]
            _anchored_vfs: None,
            #[cfg(unix)]
            _open_safety: None,
            storage: CatalogStorageKind::InMemory,
            fail_next_commit: Cell::new(false),
            fail_next_commit_outcome: Cell::new(false),
            fail_next_rollback: Cell::new(false),
        })
    }

    /// Opens `catalog.sqlite` relative to an already-validated app-data
    /// directory descriptor.
    ///
    /// The registered VFS keeps every SQLite main, journal, access, and delete
    /// operation relative to that descriptor. It therefore remains on the
    /// validated inode if the display path is renamed or replaced.
    #[cfg(unix)]
    pub fn open_anchored(owner: std::fs::File) -> Result<Self, CatalogError> {
        let owner_identity = unix_owner_identity(&owner)?;
        let open_safety = anchored_vfs::CatalogOpenSafetyLease::for_anchored_owner(&owner)
            .map_err(CatalogError::AnchoredVfs)?;
        let lease =
            anchored_vfs::AnchoredVfsLease::register(owner).map_err(CatalogError::AnchoredVfs)?;
        let conn = Connection::open_with_flags_and_vfs(
            "catalog.sqlite",
            OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            lease.name(),
        )?;
        configure_anchored_connection(&conn)?;
        Ok(Self {
            conn,
            _anchored_vfs: Some(lease),
            _open_safety: Some(open_safety),
            storage: CatalogStorageKind::Anchored(owner_identity),
            fail_next_commit: Cell::new(false),
            fail_next_commit_outcome: Cell::new(false),
            fail_next_rollback: Cell::new(false),
        })
    }

    /// Opens an existing current catalog read-only relative to an
    /// already-validated app-data directory descriptor.
    #[cfg(unix)]
    pub fn open_read_only_current_anchored(owner: std::fs::File) -> Result<Self, CatalogError> {
        let owner_identity = unix_owner_identity(&owner)?;
        let open_safety = anchored_vfs::CatalogOpenSafetyLease::for_anchored_owner(&owner)
            .map_err(CatalogError::AnchoredVfs)?;
        let lease =
            anchored_vfs::AnchoredVfsLease::register(owner).map_err(CatalogError::AnchoredVfs)?;
        let conn = Connection::open_with_flags_and_vfs(
            "catalog.sqlite",
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            lease.name(),
        )?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        let catalog = Self {
            conn,
            _anchored_vfs: Some(lease),
            _open_safety: Some(open_safety),
            storage: CatalogStorageKind::Anchored(owner_identity),
            fail_next_commit: Cell::new(false),
            fail_next_commit_outcome: Cell::new(false),
            fail_next_rollback: Cell::new(false),
        };
        if !schema::is_current(&catalog.conn)? {
            return Err(CatalogError::SchemaOutdated);
        }
        Ok(catalog)
    }

    /// Returns `None` when the descriptor-relative catalog child is absent.
    /// A symlink or non-regular child fails closed.
    #[cfg(unix)]
    pub fn open_read_only_current_anchored_if_exists(
        owner: std::fs::File,
    ) -> Result<Option<Self>, CatalogError> {
        use rustix::fs::{statat, AtFlags, FileType};

        match statat(&owner, "catalog.sqlite", AtFlags::SYMLINK_NOFOLLOW) {
            Ok(metadata) if FileType::from_raw_mode(metadata.st_mode) == FileType::RegularFile => {}
            Ok(_) => {
                return Err(CatalogError::AnchoredVfs(
                    "descriptor-relative catalog must be a regular non-symlink file".to_string(),
                ))
            }
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => {
                return Err(CatalogError::AnchoredVfs(format!(
                    "checking descriptor-relative catalog failed: {error}"
                )))
            }
        }
        Self::open_read_only_current_anchored(owner).map(Some)
    }

    pub fn init(&self) -> Result<(), CatalogError> {
        schema::init_schema(&self.conn)?;
        self.canonicalize_legacy_paths()?;
        Ok(())
    }

    /// Proves that a catalog and the caller's actual mutation lock refer to
    /// the same app-data owner before the first filesystem, SQLite, or process
    /// effect.
    ///
    /// In-memory catalogs are ownerless and remain valid for pure unit tests.
    /// Descriptor-anchored catalogs require an exact owner inode match.
    /// Legacy path-backed catalogs compare the parent identity captured at
    /// open time; production service code must use descriptor-anchored opens.
    pub fn ensure_mutation_owner(&self, _owner: &std::fs::File) -> Result<(), CatalogError> {
        match self.storage {
            CatalogStorageKind::InMemory => Ok(()),
            #[cfg(unix)]
            CatalogStorageKind::Anchored(expected)
            | CatalogStorageKind::LegacyPathBacked(expected) => {
                let actual = unix_owner_identity(_owner)?;
                if actual == expected {
                    Ok(())
                } else {
                    Err(CatalogError::MutationOwner(
                        "the locked app-data owner inode differs from the catalog owner"
                            .to_string(),
                    ))
                }
            }
            #[cfg(not(unix))]
            CatalogStorageKind::LegacyPathBacked => Ok(()),
        }
    }

    pub fn begin_immediate_transaction(
        &self,
    ) -> Result<CatalogImmediateTransaction<'_>, CatalogError> {
        Ok(CatalogImmediateTransaction {
            transaction: Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?,
            fail_commit: self.fail_next_commit.replace(false),
            fail_commit_outcome: self.fail_next_commit_outcome.replace(false),
            fail_rollback: self.fail_next_rollback.replace(false),
        })
    }

    pub fn catalog_scan_revision(&self) -> Result<CatalogScanRevisionRecord, CatalogError> {
        self.conn
            .query_row(
                "SELECT generation, revision FROM catalog_scan_state WHERE singleton = 1",
                [],
                |row| {
                    Ok(CatalogScanRevisionRecord {
                        generation: row.get(0)?,
                        revision: row.get(1)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn replace_catalog_product_projection(
        &self,
        context_revision: &str,
        scan_revision: &CatalogScanRevisionRecord,
        coverages: &[(AgentId, SourceCoverage)],
        scanned_agents: &[AgentId],
        skills: &[CatalogSkillProjectionDraft],
    ) -> Result<(), CatalogError> {
        self.conn.execute(
            "DELETE FROM catalog_scan_coverage WHERE context_revision <> ?1",
            [context_revision],
        )?;
        self.conn.execute(
            "DELETE FROM catalog_skill_projection WHERE context_revision <> ?1",
            [context_revision],
        )?;
        let mut statement = self.conn.prepare(
            "INSERT INTO catalog_scan_coverage (
                agent, context_revision, catalog_scan_generation,
                catalog_scan_revision, coverage_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(agent) DO UPDATE SET
                context_revision = excluded.context_revision,
                catalog_scan_generation = excluded.catalog_scan_generation,
                catalog_scan_revision = excluded.catalog_scan_revision,
                coverage_json = excluded.coverage_json",
        )?;
        for (agent, coverage) in coverages {
            statement.execute(params![
                agent.as_str(),
                context_revision,
                scan_revision.generation,
                scan_revision.revision,
                serde_json::to_string(coverage)?
            ])?;
        }

        for agent in scanned_agents {
            self.conn.execute(
                "DELETE FROM catalog_skill_projection WHERE agent = ?1",
                [agent.as_str()],
            )?;
        }
        let mut skill_statement = self.conn.prepare(
            "INSERT INTO catalog_skill_projection (
                instance_id, agent, context_revision, catalog_scan_generation,
                catalog_scan_revision, source_kind, source_identity,
                runtime_identity, linked, precedence_proven, coverage_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(instance_id) DO UPDATE SET
                agent = excluded.agent,
                context_revision = excluded.context_revision,
                catalog_scan_generation = excluded.catalog_scan_generation,
                catalog_scan_revision = excluded.catalog_scan_revision,
                source_kind = excluded.source_kind,
                source_identity = excluded.source_identity,
                runtime_identity = excluded.runtime_identity,
                linked = excluded.linked,
                precedence_proven = excluded.precedence_proven,
                coverage_json = excluded.coverage_json",
        )?;
        for skill in skills {
            skill_statement.execute(params![
                skill.instance_id,
                skill.agent.as_str(),
                context_revision,
                scan_revision.generation,
                scan_revision.revision,
                skill.source_kind,
                skill.source_identity,
                skill.runtime_identity,
                i64::from(skill.linked),
                i64::from(skill.precedence_proven),
                serde_json::to_string(&skill.coverage)?
            ])?;
        }
        Ok(())
    }

    pub fn list_catalog_scan_coverages(
        &self,
        context_revision: &str,
    ) -> Result<Vec<CatalogScanCoverageRecord>, CatalogError> {
        let mut statement = self.conn.prepare(
            "SELECT agent, context_revision, catalog_scan_generation,
                    catalog_scan_revision, coverage_json
             FROM catalog_scan_coverage
             WHERE context_revision = ?1
             ORDER BY agent",
        )?;
        let rows = statement.query_map([context_revision], |row| {
            let agent = agent_id_from_wire(row.get::<_, String>(0)?).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "catalog scan coverage contains an unknown agent",
                    )),
                )
            })?;
            let coverage_json: String = row.get(4)?;
            let coverage = serde_json::from_str(&coverage_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(CatalogScanCoverageRecord {
                agent,
                context_revision: row.get(1)?,
                catalog_scan_generation: row.get(2)?,
                catalog_scan_revision: row.get(3)?,
                coverage,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_catalog_skill_projections(
        &self,
        context_revision: &str,
    ) -> Result<Vec<CatalogSkillProjectionRecord>, CatalogError> {
        let mut statement = self.conn.prepare(
            "SELECT instance_id, agent, context_revision, catalog_scan_generation,
                    catalog_scan_revision, source_kind, source_identity,
                    runtime_identity, linked, precedence_proven, coverage_json
             FROM catalog_skill_projection
             WHERE context_revision = ?1
             ORDER BY instance_id",
        )?;
        let rows = statement.query_map([context_revision], |row| {
            let agent = agent_id_from_wire(row.get::<_, String>(1)?).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "catalog skill projection contains an unknown agent",
                    )),
                )
            })?;
            let coverage_json: String = row.get(10)?;
            let coverage = serde_json::from_str(&coverage_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    10,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(CatalogSkillProjectionRecord {
                instance_id: row.get(0)?,
                agent,
                context_revision: row.get(2)?,
                catalog_scan_generation: row.get(3)?,
                catalog_scan_revision: row.get(4)?,
                source_kind: row.get(5)?,
                source_identity: row.get(6)?,
                runtime_identity: row.get(7)?,
                linked: row.get::<_, i64>(8)? != 0,
                precedence_proven: row.get::<_, i64>(9)? != 0,
                coverage,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Advances the scan-only catalog revision inside the caller's transaction.
    ///
    /// Scan orchestration calls this only after every adapter row, finding, and
    /// conflict update has succeeded, so the revision commits or rolls back
    /// with the complete refresh.
    pub fn advance_catalog_scan_revision(
        &self,
        operation: &str,
        accepted_context_revision: &str,
    ) -> Result<CatalogScanRevisionRecord, CatalogError> {
        let previous = self.catalog_scan_revision()?;
        let generation = previous.generation.saturating_add(1);
        let generation_text = generation.to_string();
        let mut hasher = Sha256::new();
        for (label, value) in [
            ("domain", "agent-copilot/catalog-scan-revision/v1"),
            ("previous", previous.revision.as_str()),
            ("generation", generation_text.as_str()),
            ("operation", operation),
            ("context", accepted_context_revision),
        ] {
            hasher.update((label.len() as u64).to_be_bytes());
            hasher.update(label.as_bytes());
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        let revision = format!("sha256:{:x}", hasher.finalize());
        self.conn.execute(
            "UPDATE catalog_scan_state
             SET generation = ?1, revision = ?2
             WHERE singleton = 1",
            params![generation, revision],
        )?;
        Ok(CatalogScanRevisionRecord {
            generation,
            revision,
        })
    }

    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn inject_next_commit_failure_for_test(&self) {
        self.fail_next_commit.set(true);
    }

    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn inject_next_commit_outcome_unknown_for_test(&self) {
        self.fail_next_commit_outcome.set(true);
    }

    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn inject_next_rollback_failure_for_test(&self) {
        self.fail_next_rollback.set(true);
    }
    /// Migrate records whose `path` was stored as a display path (pre-refactor)
    /// to the canonical path. When a canonical path already exists for the same
    /// (agent, scope) the non-canonical duplicate is deleted.
    fn canonicalize_legacy_paths(&self) -> Result<usize, CatalogError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, agent, scope, path FROM skill_instance")?;
        let rows: Vec<(String, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut fixed = 0usize;
        for (id, agent, scope, current_path) in &rows {
            let canonical = match PathBuf::from(current_path).canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let canonical_str = canonical.to_string_lossy().to_string();
            if &canonical_str == current_path {
                continue;
            }
            // Does another record already occupy the canonical path?
            let conflict: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM skill_instance WHERE agent = ?1 AND scope = ?2 AND path = ?3",
                    params![agent, scope, canonical_str],
                    |row| row.get(0),
                )
                .ok();
            if let Some(conflict_id) = conflict {
                if conflict_id != *id {
                    // Merge: drop the non-canonical duplicate.
                    self.conn
                        .execute("DELETE FROM skill_instance WHERE id = ?1", params![id])?;
                    fixed += 1;
                }
            } else {
                self.conn.execute(
                    "UPDATE skill_instance SET path = ?1 WHERE id = ?2",
                    params![canonical_str, id],
                )?;
                fixed += 1;
            }
        }
        Ok(fixed)
    }

    pub fn upsert_skill_instance(&self, inst: &SkillInstance) -> Result<(), CatalogError> {
        self.conn.execute(
            r#"
            INSERT INTO skill_instance (
                id, agent, scope, project_root, path, display_path, definition_id, name, description,
                version, state, enabled, frontmatter, frontmatter_raw, body, scripts,
                permissions, fingerprint, mtime, first_seen, last_seen
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            ON CONFLICT(agent, scope, path) DO UPDATE SET
                id = excluded.id,
                agent = excluded.agent,
                scope = excluded.scope,
                project_root = excluded.project_root,
                display_path = excluded.display_path,
                definition_id = excluded.definition_id,
                name = excluded.name,
                description = excluded.description,
                version = excluded.version,
                state = excluded.state,
                enabled = excluded.enabled,
                frontmatter = excluded.frontmatter,
                frontmatter_raw = excluded.frontmatter_raw,
                body = excluded.body,
                scripts = excluded.scripts,
                permissions = excluded.permissions,
                fingerprint = excluded.fingerprint,
                mtime = excluded.mtime,
                last_seen = excluded.last_seen
            "#,
            params![
                inst.id,
                inst.agent.as_str(),
                inst.scope.as_str(),
                inst.project_root
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                inst.path.to_string_lossy(),
                inst.display_path.to_string_lossy(),
                inst.definition_id,
                inst.name,
                inst.description,
                inst.version,
                inst.state.as_str(),
                i64::from(inst.enabled),
                "{}",
                inst.frontmatter_raw,
                inst.body,
                "[]",
                permissions_json(inst)?,
                inst.fingerprint,
                inst.mtime,
                inst.first_seen,
                inst.last_seen,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_skill_instances(&self, instances: &[SkillInstance]) -> Result<(), CatalogError> {
        for inst in instances {
            self.upsert_skill_instance(inst)?;
        }
        Ok(())
    }

    /// Mark every record for `agent` whose path is under one of `scanned_roots`
    /// but not present in `seen` as `state = 'missing'`. Records whose path is
    /// outside all `scanned_roots` are left untouched — they belong to scopes
    /// the scanner did not visit this round and should not be penalised for it.
    ///
    /// `scanned_roots` and the record paths in the database are expected to be
    /// canonical (resolved through symlinks). The scanner is responsible for
    /// canonicalising both before this call.
    ///
    /// Returns the number of records transitioned to `missing`.
    pub fn mark_missing_except(
        &self,
        agent: &str,
        scanned_roots: &[PathBuf],
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        let scoped_roots = legacy_scoped_scan_roots(scanned_roots);
        self.mark_missing_except_with_project_context(agent, None, Some(&scoped_roots), seen)
    }

    /// Scope-aware missing sweep. A complete path in one scope must not make a
    /// row under the same canonical path eligible in a different scope.
    pub fn mark_missing_except_scoped(
        &self,
        agent: &str,
        scanned_roots: &[(Scope, PathBuf)],
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        self.mark_missing_except_with_project_context(agent, None, Some(scanned_roots), seen)
    }

    /// Project-aware variant of [`Catalog::mark_missing_except`]. AgentProject
    /// rows are eligible for a missing sweep only when their stored
    /// `project_root` matches the current project context. This keeps scans for
    /// one selected project, or no selected project, from changing catalog state
    /// for records that belong to another project.
    pub fn mark_missing_except_for_project_context(
        &self,
        agent: &str,
        current_project_root: Option<&Path>,
        scanned_roots: &[PathBuf],
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        let scoped_roots = legacy_scoped_scan_roots(scanned_roots);
        self.mark_missing_except_with_project_context(
            agent,
            Some(current_project_root),
            Some(&scoped_roots),
            seen,
        )
    }

    /// Scope-aware project-context variant used by scanner-backed refreshes.
    pub fn mark_missing_except_scoped_for_project_context(
        &self,
        agent: &str,
        current_project_root: Option<&Path>,
        scanned_roots: &[(Scope, PathBuf)],
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        self.mark_missing_except_with_project_context(
            agent,
            Some(current_project_root),
            Some(scanned_roots),
            seen,
        )
    }

    /// Agent-wide cleanup for an exact complete scan. Unlike the root-bounded
    /// sweep, this also retires rows whose previously declared source root no
    /// longer exists or is no longer selected by the adapter.
    pub fn mark_missing_except_agent_for_project_context(
        &self,
        agent: &str,
        current_project_root: Option<&Path>,
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        self.mark_missing_except_with_project_context(agent, Some(current_project_root), None, seen)
    }

    fn mark_missing_except_with_project_context(
        &self,
        agent: &str,
        project_context: Option<Option<&Path>>,
        scanned_roots: Option<&[(Scope, PathBuf)]>,
        seen: &[(String, PathBuf)],
    ) -> Result<usize, CatalogError> {
        let seen_set: HashSet<(String, String)> = seen
            .iter()
            .map(|(scope, path)| (scope.clone(), path.to_string_lossy().to_string()))
            .collect();
        let occurred_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(i64::MAX);

        let owns_transaction = self.conn.is_autocommit();
        if owns_transaction {
            self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        }
        let result = (|| -> Result<usize, CatalogError> {
            let mut stmt = self.conn.prepare(
                "SELECT id, scope, project_root, path, state
                 FROM skill_instance WHERE agent = ?1",
            )?;
            let rows = stmt.query_map(params![agent], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            let mut existing: Vec<(String, String, Option<String>, String, String)> = Vec::new();
            for row in rows {
                existing.push(row?);
            }

            let to_mark: Vec<String> = existing
                .into_iter()
                .filter(|(_, scope, project_root, path, state)| {
                    if scope == Scope::ToolGlobal.as_str() || state == SkillState::Missing.as_str()
                    {
                        return false;
                    }
                    if seen_set.contains(&(scope.clone(), path.clone())) {
                        return false;
                    }
                    if !record_matches_project_context(
                        scope,
                        project_root.as_deref(),
                        project_context,
                    ) {
                        return false;
                    }
                    let Some(scanned_roots) = scanned_roots else {
                        return true;
                    };
                    let record_path = PathBuf::from(path);
                    scanned_roots.iter().any(|(root_scope, root)| {
                        scope == root_scope.as_str() && record_path.starts_with(root)
                    })
                })
                .map(|(id, _, _, _, _)| id)
                .collect();

            let mut update = self.conn.prepare(
                "UPDATE skill_instance SET state = 'missing'
                 WHERE id = ?1 AND state <> 'missing'",
            )?;
            let mut insert_event = self.conn.prepare(
                "INSERT INTO skill_event (instance_id, kind, payload, occurred_at)
                 VALUES (?1, 'missing', ?2, ?3)",
            )?;
            let mut transitioned = 0;
            for id in &to_mark {
                if update.execute(params![id])? == 0 {
                    continue;
                }
                insert_event.execute(params![
                    id,
                    r#"{"reason":"not_seen_in_complete_scan"}"#,
                    occurred_at_ms,
                ])?;
                transitioned += 1;
            }
            Ok(transitioned)
        })();

        if !owns_transaction {
            return result;
        }
        match result {
            Ok(count) => match self.conn.execute_batch("COMMIT") {
                Ok(()) => Ok(count),
                Err(error) => {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    Err(CatalogError::Sqlite(error))
                }
            },
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    /// Update the `enabled` flag and the human-facing `state` for a skill
    /// instance in a single transaction. The state should be either `"loaded"`
    /// (on) or `"disabled"` (off).
    pub fn set_skill_toggle(
        &self,
        id: &str,
        enabled: bool,
        state: &str,
    ) -> Result<(), CatalogError> {
        self.conn.execute(
            "UPDATE skill_instance SET enabled = ?1, state = ?2 WHERE id = ?3",
            params![i64::from(enabled), state, id],
        )?;
        Ok(())
    }

    pub fn delete_skill_instance(&self, id: &str) -> Result<bool, CatalogError> {
        Ok(self
            .conn
            .execute("DELETE FROM skill_instance WHERE id = ?1", params![id])?
            > 0)
    }

    pub fn create_skill_event(&self, draft: SkillEventDraft<'_>) -> Result<(), CatalogError> {
        self.conn.execute(
            "INSERT INTO skill_event (instance_id, kind, payload, occurred_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                draft.instance_id,
                draft.kind,
                draft.payload,
                draft.occurred_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn set_finding_triage(
        &self,
        triage_key: &str,
        status: &str,
        note: Option<&str>,
        updated_at: i64,
    ) -> Result<Option<FindingTriageRecord>, CatalogError> {
        let Some(triage_context) = self.current_finding_triage_context(triage_key)? else {
            return Ok(None);
        };
        self.conn.execute(
            "INSERT INTO finding_triage (triage_key, triage_context, status, note, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(triage_key) DO UPDATE SET
                triage_context = excluded.triage_context,
                status = excluded.status,
                note = excluded.note,
                updated_at = excluded.updated_at",
            params![
                triage_key,
                triage_context.as_str(),
                status,
                note,
                updated_at
            ],
        )?;
        Ok(Some(FindingTriageRecord {
            triage_key: triage_key.to_string(),
            triage_context,
            status: status.to_string(),
            note: note.map(str::to_string),
            updated_at,
        }))
    }

    pub fn clear_finding_triage(&self, triage_key: &str) -> Result<bool, CatalogError> {
        Ok(self.conn.execute(
            "DELETE FROM finding_triage WHERE triage_key = ?1",
            params![triage_key],
        )? > 0)
    }

    pub fn set_rule_severity_override(
        &self,
        rule_id: &str,
        agent: Option<&str>,
        scope: Option<&str>,
        severity: &str,
        updated_at: i64,
    ) -> Result<RuleTuningRecord, CatalogError> {
        self.upsert_rule_tuning(RuleTuningUpdate {
            rule_id,
            agent,
            scope,
            severity_override: Some(severity),
            suppression_reason: None,
            suppression_note: None,
            updated_at,
        })?;
        self.get_rule_tuning(rule_id, agent, scope)
    }

    pub fn clear_rule_severity_override(
        &self,
        rule_id: &str,
        agent: Option<&str>,
        scope: Option<&str>,
        updated_at: i64,
    ) -> Result<bool, CatalogError> {
        let key = rule_tuning_key(agent, scope);
        if self.conn.execute(
            "DELETE FROM rule_tuning
             WHERE rule_id = ?1 AND agent = ?2 AND scope = ?3
               AND severity_override IS NOT NULL
               AND suppression_reason IS NULL",
            params![rule_id, key.0, key.1],
        )? > 0
        {
            return Ok(true);
        }
        Ok(self.conn.execute(
            "UPDATE rule_tuning
             SET severity_override = NULL, updated_at = ?4
             WHERE rule_id = ?1 AND agent = ?2 AND scope = ?3
               AND severity_override IS NOT NULL",
            params![rule_id, key.0, key.1, updated_at],
        )? > 0)
    }

    pub fn set_rule_suppression(
        &self,
        rule_id: &str,
        agent: Option<&str>,
        scope: Option<&str>,
        reason: &str,
        note: Option<&str>,
        updated_at: i64,
    ) -> Result<RuleTuningRecord, CatalogError> {
        self.upsert_rule_tuning(RuleTuningUpdate {
            rule_id,
            agent,
            scope,
            severity_override: None,
            suppression_reason: Some(reason),
            suppression_note: note,
            updated_at,
        })?;
        self.get_rule_tuning(rule_id, agent, scope)
    }

    pub fn clear_rule_suppression(
        &self,
        rule_id: &str,
        agent: Option<&str>,
        scope: Option<&str>,
        updated_at: i64,
    ) -> Result<bool, CatalogError> {
        let key = rule_tuning_key(agent, scope);
        if self.conn.execute(
            "DELETE FROM rule_tuning
             WHERE rule_id = ?1 AND agent = ?2 AND scope = ?3
               AND severity_override IS NULL
               AND suppression_reason IS NOT NULL",
            params![rule_id, key.0, key.1],
        )? > 0
        {
            return Ok(true);
        }
        Ok(self.conn.execute(
            "UPDATE rule_tuning
             SET suppression_reason = NULL, suppression_note = NULL, updated_at = ?4
             WHERE rule_id = ?1 AND agent = ?2 AND scope = ?3
               AND suppression_reason IS NOT NULL",
            params![rule_id, key.0, key.1, updated_at],
        )? > 0)
    }

    fn upsert_rule_tuning(&self, update: RuleTuningUpdate<'_>) -> Result<(), CatalogError> {
        let key = rule_tuning_key(update.agent, update.scope);
        self.conn.execute(
            "INSERT INTO rule_tuning (
                rule_id, agent, scope, severity_override, suppression_reason, suppression_note, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(rule_id, agent, scope) DO UPDATE SET
                severity_override = COALESCE(excluded.severity_override, rule_tuning.severity_override),
                suppression_reason = COALESCE(excluded.suppression_reason, rule_tuning.suppression_reason),
                suppression_note = CASE
                    WHEN excluded.suppression_reason IS NOT NULL THEN excluded.suppression_note
                    ELSE rule_tuning.suppression_note
                END,
                updated_at = excluded.updated_at",
            params![
                update.rule_id,
                key.0,
                key.1,
                update.severity_override,
                update.suppression_reason,
                update.suppression_note,
                update.updated_at
            ],
        )?;
        Ok(())
    }

    /// Record a pre-write snapshot of a config file. Caller supplies a unique
    /// id (e.g. `"snap-<nanos>"`). The `draft` bundles the snapshot fields so
    /// the call site stays readable.
    pub fn create_config_snapshot(
        &self,
        draft: ConfigSnapshotDraft<'_>,
    ) -> Result<(), CatalogError> {
        self.conn.execute(
            "INSERT INTO config_snapshot (
                id, agent, scope, project_root, target, content, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                draft.id,
                draft.agent,
                draft.scope,
                draft.project_root,
                draft.target,
                draft.content,
                draft.reason,
                draft.created_at_ms,
            ],
        )?;
        Ok(())
    }
}

#[cfg(unix)]
fn configure_anchored_connection(conn: &Connection) -> Result<(), CatalogError> {
    // Keep SQLite side files inside the descriptor-anchored VFS. WAL shared
    // memory can bypass xOpen through xShmMap, so this catalog deliberately
    // uses the rollback journal and in-memory temporary storage.
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(())
}

#[cfg(unix)]
fn unix_owner_identity(owner: &std::fs::File) -> Result<UnixOwnerIdentity, CatalogError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = owner.metadata().map_err(|error| {
        CatalogError::MutationOwner(format!(
            "reading the catalog owner identity failed: {error}"
        ))
    })?;
    if !metadata.is_dir() {
        return Err(CatalogError::MutationOwner(
            "the catalog owner must be a directory".to_string(),
        ));
    }
    Ok(UnixOwnerIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn legacy_path_storage(_path: &Path) -> Result<CatalogStorageKind, CatalogError> {
    Ok(CatalogStorageKind::LegacyPathBacked)
}

#[cfg(unix)]
fn normalize_sqlite_root_alias(path: &Path) -> PathBuf {
    use std::{fs, os::unix::fs::MetadataExt, path::Component};

    if !path.is_absolute() {
        return path.to_path_buf();
    }
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return path.to_path_buf();
    }
    let Some(Component::Normal(first)) = components.next() else {
        return path.to_path_buf();
    };
    let root_entry = Path::new("/").join(first);
    let Ok(metadata) = fs::symlink_metadata(&root_entry) else {
        return path.to_path_buf();
    };
    if !metadata.file_type().is_symlink() || metadata.uid() != 0 {
        return path.to_path_buf();
    }
    let Ok(mut normalized) = fs::canonicalize(root_entry) else {
        return path.to_path_buf();
    };
    for component in components {
        match component {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return path.to_path_buf()
            }
        }
    }
    normalized
}

fn agent_id_from_wire(value: String) -> Option<AgentId> {
    match value.as_str() {
        "tool-global" => Some(AgentId::ToolGlobal),
        "claude-code" => Some(AgentId::ClaudeCode),
        "codex" => Some(AgentId::Codex),
        "pi" => Some(AgentId::Pi),
        "hermes" => Some(AgentId::Hermes),
        "openclaw" => Some(AgentId::Openclaw),
        "opencode" => Some(AgentId::Opencode),
        _ => None,
    }
}

#[cfg(not(unix))]
fn normalize_sqlite_root_alias(path: &Path) -> PathBuf {
    path.to_path_buf()
}

impl CatalogImmediateTransaction<'_> {
    pub fn commit(self) -> Result<(), CatalogError> {
        if self.fail_commit {
            return Err(CatalogError::InjectedCommitFailure);
        }
        if self.fail_commit_outcome {
            return Err(CatalogError::InjectedCommitOutcomeUnknown);
        }
        self.transaction.commit().map_err(Into::into)
    }

    /// Commit with an explicit distinction between a transaction that is
    /// proven uncommitted and one whose SQLite commit result is uncertain.
    pub fn commit_classified(self) -> Result<(), CatalogCommitError> {
        let Self {
            transaction,
            fail_commit,
            fail_commit_outcome,
            fail_rollback,
        } = self;
        if fail_commit {
            if fail_rollback {
                return Err(CatalogCommitError::OutcomeUnknown(
                    CatalogError::InjectedRollbackFailure,
                ));
            }
            return match transaction.rollback() {
                Ok(()) => Err(CatalogCommitError::NotCommitted(
                    CatalogError::InjectedCommitFailure,
                )),
                Err(error) => Err(CatalogCommitError::OutcomeUnknown(CatalogError::Sqlite(
                    error,
                ))),
            };
        }
        if fail_commit_outcome {
            return Err(CatalogCommitError::OutcomeUnknown(
                CatalogError::InjectedCommitOutcomeUnknown,
            ));
        }
        transaction
            .commit()
            .map_err(|error| CatalogCommitError::OutcomeUnknown(CatalogError::Sqlite(error)))
    }

    /// Explicit rollback for callers that must classify rollback failure rather
    /// than relying on the transaction guard's best-effort drop behavior.
    pub fn rollback(self) -> Result<(), CatalogError> {
        if self.fail_rollback {
            return Err(CatalogError::InjectedRollbackFailure);
        }
        self.transaction.rollback().map_err(Into::into)
    }
}

fn legacy_scoped_scan_roots(scanned_roots: &[PathBuf]) -> Vec<(Scope, PathBuf)> {
    scanned_roots
        .iter()
        .flat_map(|root| {
            [Scope::AgentGlobal, Scope::AgentProject]
                .into_iter()
                .map(|scope| (scope, root.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use skills_copilot_adapters::ClaudeCodeAdapter;
    use skills_copilot_core::{
        AgentAdapter, AgentId, NetworkAccess, PermissionRequest, Scope, SkillState,
    };

    use super::*;

    fn catalog_test_instance(
        agent: AgentId,
        scope: Scope,
        path: &str,
        name: &str,
        state: SkillState,
    ) -> SkillInstance {
        SkillInstance {
            id: format!("{}:{path}", agent.as_str()),
            agent,
            scope,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.to_ascii_lowercase(),
            name: name.to_string(),
            display_name: name.to_string(),
            description: "catalog test fixture".to_string(),
            version: None,
            enabled: matches!(state, SkillState::Loaded | SkillState::Disabled),
            state,
            frontmatter_raw: format!("name: {name}\ndescription: catalog test fixture"),
            body: "body".to_string(),
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: String::new(),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        }
    }

    #[test]
    fn initializes_and_upserts_skill_records() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let skill = ClaudeCodeAdapter
            .parse(&fixture_path(
                "fixtures/claude-code/personal/valid-summarize/SKILL.md",
            ))
            .expect("skill parses");

        catalog
            .upsert_skill_instance(&skill)
            .expect("skill upserts");
        let records = catalog.list_skill_records().expect("records list");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "summarize-changes");
    }

    #[test]
    fn codex_plugin_cache_records_expose_read_only_package_provenance() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let instance = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/plugins/cache/openai-bundled/browser/1.10.0/playbooks/control/SKILL.md",
            "browser-control",
            SkillState::Loaded,
        );
        let id = instance.id.clone();
        catalog
            .upsert_skill_instance(&instance)
            .expect("plugin skill upserts");

        let record = catalog
            .get_skill_record(&id)
            .expect("record query succeeds")
            .expect("record exists");
        assert_eq!(record.publisher.as_deref(), Some("openai-bundled"));
        assert_eq!(record.package_name.as_deref(), Some("browser"));
        assert_eq!(record.package_version.as_deref(), Some("1.10.0"));
        assert_eq!(record.source_kind.as_deref(), Some("chatgpt-plugin-cache"));
        assert_eq!(
            record.read_only_reason.as_deref(),
            Some("Installed Codex plugin files are read-only")
        );

        let detail = catalog
            .get_skill_detail(&id)
            .expect("detail query succeeds")
            .expect("detail exists");
        assert_eq!(detail.publisher, record.publisher);
        assert_eq!(detail.package_name, record.package_name);
        assert_eq!(detail.package_version, record.package_version);
        assert_eq!(detail.source_kind, record.source_kind);
        assert_eq!(detail.read_only_reason, record.read_only_reason);
    }

    #[test]
    fn codex_runtime_records_expose_read_only_runtime_provenance() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let instance = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/.agent-copilot-runtime/abc/SKILL.md",
            "agent-pet-companion:agent-pet-studio",
            SkillState::Loaded,
        );
        let id = instance.id.clone();
        catalog
            .upsert_skill_instance(&instance)
            .expect("plugin skill upserts");

        let record = catalog
            .get_skill_record(&id)
            .expect("record query succeeds")
            .expect("record exists");
        assert_eq!(record.publisher, None);
        assert_eq!(record.package_name, None);
        assert_eq!(record.package_version, None);
        assert_eq!(record.source_kind.as_deref(), Some("codex-runtime"));
        assert_eq!(
            record.read_only_reason.as_deref(),
            Some("Reported by the current Codex runtime; source cache paths are not scanned or persisted")
        );
    }

    #[test]
    fn current_skill_projections_include_codex_plugin_files_and_ignore_legacy_runtime_rows() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let old_cache = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/plugins/cache/personal/agent-pet-companion/0.1.0/skills/agent-pet-studio/SKILL.md",
            "agent-pet-studio",
            SkillState::Missing,
        );
        let current_cache = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/plugins/cache/personal/agent-pet-companion/0.2.0/skills/agent-pet-studio/SKILL.md",
            "agent-pet-studio",
            SkillState::Loaded,
        );
        let runtime = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/.agent-copilot-runtime/abc/SKILL.md",
            "agent-pet-companion:agent-pet-studio",
            SkillState::Loaded,
        );
        let old_cache_id = old_cache.id.clone();
        let current_cache_id = current_cache.id.clone();

        catalog
            .upsert_skill_instances(&[old_cache, runtime, current_cache])
            .expect("plugin rows upsert");
        let records = catalog.list_skill_records().expect("records list");
        let instances = catalog
            .list_skill_instances_for_project_context(None)
            .expect("instances list");

        assert_eq!(records.len(), 2);
        assert_eq!(
            records
                .iter()
                .map(|record| record.id.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([old_cache_id.as_str(), current_cache_id.as_str()])
        );
        assert!(records
            .iter()
            .all(|record| { record.source_kind.as_deref() == Some("chatgpt-plugin-cache") }));
        assert_eq!(instances.len(), 2);
        assert!(instances
            .iter()
            .all(|instance| { instance.path.to_string_lossy().contains("/plugins/cache/") }));
        let raw_count = catalog
            .conn
            .query_row("SELECT COUNT(*) FROM skill_instance", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("raw row count");
        assert_eq!(
            raw_count, 3,
            "legacy runtime history may remain persisted until migration"
        );
    }

    #[test]
    fn list_skill_records_keeps_same_agent_same_name_different_paths() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let native = catalog_test_instance(
            AgentId::Opencode,
            Scope::AgentGlobal,
            "/tmp/home/.config/opencode/skills/shared-review/SKILL.md",
            "shared-review",
            SkillState::Loaded,
        );
        let duplicate_missing = catalog_test_instance(
            AgentId::Opencode,
            Scope::AgentGlobal,
            "/tmp/home/.agents/skills/shared-review/SKILL.md",
            "shared-review",
            SkillState::Missing,
        );

        catalog
            .upsert_skill_instances(&[duplicate_missing, native])
            .expect("upsert duplicate records");
        let records = catalog.list_skill_records().expect("records list");

        assert_eq!(records.len(), 2);
        assert!(records
            .iter()
            .any(|record| record.name == "shared-review" && record.state == "loaded"));
        assert!(records
            .iter()
            .any(|record| record.name == "shared-review" && record.state == "missing"));
    }

    #[test]
    fn list_skill_records_filters_pi_historical_markdown_noise() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let real = catalog_test_instance(
            AgentId::Pi,
            Scope::AgentGlobal,
            "/tmp/home/.pi/agent/skills/global-pdf/SKILL.md",
            "global-pdf",
            SkillState::Loaded,
        );
        let root_markdown = catalog_test_instance(
            AgentId::Pi,
            Scope::AgentGlobal,
            "/tmp/home/.pi/agent/skills/root-note.md",
            "root-note",
            SkillState::Missing,
        );
        let root_skill_md = catalog_test_instance(
            AgentId::Pi,
            Scope::AgentGlobal,
            "/tmp/home/.pi/agent/skills/SKILL.md",
            "root-noise",
            SkillState::Missing,
        );
        let reference_skill_md = catalog_test_instance(
            AgentId::Pi,
            Scope::AgentGlobal,
            "/tmp/home/.pi/agent/skills/global-pdf/references/SKILL.md",
            "reference-noise",
            SkillState::Missing,
        );

        catalog
            .upsert_skill_instances(&[real, root_markdown, root_skill_md, reference_skill_md])
            .expect("upsert Pi records");
        let records = catalog.list_skill_records().expect("records list");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "global-pdf");
    }

    #[test]
    fn skill_instances_roundtrip_permissions_from_catalog_rows() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let mut skill = ClaudeCodeAdapter
            .parse(&fixture_path(
                "fixtures/claude-code/personal/valid-summarize/SKILL.md",
            ))
            .expect("skill parses");
        skill.scope = Scope::AgentGlobal;
        skill.permissions = PermissionRequest {
            tools: vec!["Bash(git status:*)".to_string(), "Read".to_string()],
            files: vec!["/tmp/report.md".to_string()],
            network: NetworkAccess::ReadOnly,
            network_declared: true,
            exec: true,
            exec_declared: true,
            requires_human: false,
            requires_human_declared: true,
        };

        catalog
            .upsert_skill_instance(&skill)
            .expect("skill upserts");
        let instances = catalog
            .list_skill_instances_for_project_context(None)
            .expect("instances list");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].permissions, skill.permissions);
        assert_eq!(parse_permissions_json("{"), PermissionRequest::default());
        assert_eq!(
            parse_permissions_json(
                r#"{"tools":[],"files":[],"network":"internet","exec":false,"requires_human":true}"#
            ),
            PermissionRequest {
                network: NetworkAccess::Unknown("internet".to_string()),
                network_declared: true,
                exec: false,
                exec_declared: true,
                requires_human: true,
                requires_human_declared: true,
                ..PermissionRequest::default()
            }
        );
        assert_eq!(
            network_access_key(&NetworkAccess::Unknown("raw".to_string())),
            "raw"
        );
    }

    #[test]
    fn mark_missing_except_moves_unseen_records_to_missing_state() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let path_a = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");
        let path_b = fixture_path("fixtures/claude-code/project/valid-review/SKILL.md");
        let mut inst_a = ClaudeCodeAdapter.parse(&path_a).expect("skill a parses");
        inst_a.scope = Scope::AgentGlobal;
        inst_a.path = path_a.clone();
        let mut inst_b = ClaudeCodeAdapter.parse(&path_b).expect("skill b parses");
        inst_b.scope = Scope::AgentProject;
        inst_b.path = path_b.clone();

        catalog.upsert_skill_instance(&inst_a).expect("upsert a");
        catalog.upsert_skill_instance(&inst_b).expect("upsert b");
        assert_eq!(catalog.list_skill_records().expect("list").len(), 2);

        let scanned_roots = vec![
            path_a.parent().expect("a parent").to_path_buf(),
            path_b.parent().expect("b parent").to_path_buf(),
        ];
        let seen = vec![("agent-global".to_string(), path_a.clone())];
        let marked = catalog
            .mark_missing_except("claude-code", &scanned_roots, &seen)
            .expect("sweep succeeds");
        assert_eq!(marked, 1);

        let records = catalog.list_skill_records().expect("records after sweep");
        let loaded = records
            .iter()
            .find(|r| r.path == path_a)
            .expect("seen record still present");
        let missing = records
            .iter()
            .find(|r| r.path == path_b)
            .expect("unseen record retained");
        assert_eq!(loaded.state, "loaded");
        assert_eq!(missing.state, "missing");

        let events = catalog
            .list_skill_events(&inst_b.id, None)
            .expect("missing events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "missing");
        assert_eq!(
            events[0].payload,
            serde_json::json!({ "reason": "not_seen_in_complete_scan" })
        );
        assert!(
            !events[0]
                .payload
                .to_string()
                .contains(&path_b.to_string_lossy().to_string()),
            "missing event payload must not persist the local absolute path"
        );

        let marked_again = catalog
            .mark_missing_except("claude-code", &scanned_roots, &seen)
            .expect("repeated sweep succeeds");
        assert_eq!(
            marked_again, 0,
            "already-missing rows are not transitioned again"
        );
        assert_eq!(
            catalog
                .list_skill_events(&inst_b.id, None)
                .expect("events after repeated sweep")
                .len(),
            1,
            "repeated complete scans do not duplicate missing events"
        );
    }

    #[test]
    fn mark_missing_except_leaves_records_outside_scanned_roots_alone() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let scanned_root = fixture_path("fixtures/claude-code/personal");
        let inside_path = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");
        let outside_path = fixture_path("fixtures/claude-code/project/valid-review/SKILL.md");

        let mut inside = ClaudeCodeAdapter
            .parse(&inside_path)
            .expect("inside parses");
        inside.scope = Scope::AgentGlobal;
        inside.path = inside_path.clone();
        let mut outside = ClaudeCodeAdapter
            .parse(&outside_path)
            .expect("outside parses");
        outside.scope = Scope::AgentProject;
        outside.path = outside_path.clone();

        catalog
            .upsert_skill_instance(&inside)
            .expect("upsert inside");
        catalog
            .upsert_skill_instance(&outside)
            .expect("upsert outside");

        let seen = vec![("agent-global".to_string(), inside_path.clone())];
        let marked = catalog
            .mark_missing_except("claude-code", std::slice::from_ref(&scanned_root), &seen)
            .expect("sweep succeeds");
        assert_eq!(marked, 0, "outside record is not under scanned_root");

        let records = catalog.list_skill_records().expect("records");
        let outside_record = records
            .iter()
            .find(|r| r.path == outside_path)
            .expect("outside record still present");
        assert_eq!(
            outside_record.state, "loaded",
            "records outside any scanned_root are not swept"
        );
    }

    #[test]
    fn exact_agent_sweep_retires_removed_roots_but_preserves_other_projects() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let current_project = PathBuf::from("/tmp/current-project");
        let other_project = PathBuf::from("/tmp/other-project");
        let mut retired_global = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/retired-plugin/version-1/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        retired_global.id = "retired-global".to_string();
        let mut retired_current_project = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentProject,
            "/tmp/current-project/.removed-skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        retired_current_project.id = "retired-current-project".to_string();
        retired_current_project.project_root = Some(current_project.clone());
        let mut other_project_record = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentProject,
            "/tmp/other-project/.agents/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        other_project_record.id = "other-project".to_string();
        other_project_record.project_root = Some(other_project);
        catalog
            .upsert_skill_instances(&[
                retired_global,
                retired_current_project,
                other_project_record,
            ])
            .expect("seed retired-root rows");

        let marked = catalog
            .mark_missing_except_agent_for_project_context("codex", Some(&current_project), &[])
            .expect("exact agent sweep succeeds");
        let records = catalog.list_skill_records().expect("records after sweep");

        assert_eq!(marked, 2);
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "retired-global")
                .expect("global row")
                .state,
            "missing"
        );
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "retired-current-project")
                .expect("current project row")
                .state,
            "missing"
        );
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "other-project")
                .expect("other project row")
                .state,
            "loaded"
        );
    }

    #[test]
    fn scoped_missing_sweep_does_not_cross_scope_on_same_canonical_root() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let path = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md")
            .canonicalize()
            .expect("canonical skill path");
        let root = path.parent().expect("skill parent").to_path_buf();
        let mut global = ClaudeCodeAdapter.parse(&path).expect("global parses");
        global.id = "same-root-global".to_string();
        global.scope = Scope::AgentGlobal;
        global.path = path.clone();
        let mut project = global.clone();
        project.id = "same-root-project".to_string();
        project.scope = Scope::AgentProject;
        project.project_root = Some(root.clone());

        catalog
            .upsert_skill_instances(&[global, project])
            .expect("seed same-root rows");
        let marked = catalog
            .mark_missing_except_scoped("claude-code", &[(Scope::AgentGlobal, root)], &[])
            .expect("scoped sweep succeeds");
        let records = catalog.list_skill_records().expect("records after sweep");

        assert_eq!(marked, 1);
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "same-root-global")
                .expect("global row")
                .state,
            "missing"
        );
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "same-root-project")
                .expect("project row")
                .state,
            "loaded"
        );
    }

    #[test]
    fn missing_event_insert_failure_rolls_back_state_transition() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let path = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md")
            .canonicalize()
            .expect("canonical skill path");
        let root = path.parent().expect("skill parent").to_path_buf();
        let mut instance = ClaudeCodeAdapter.parse(&path).expect("skill parses");
        instance.id = "rollback-on-event-failure".to_string();
        instance.scope = Scope::AgentGlobal;
        instance.path = path;
        catalog
            .upsert_skill_instance(&instance)
            .expect("seed instance");
        catalog
            .conn
            .execute_batch(
                "CREATE TRIGGER fail_missing_event
                 BEFORE INSERT ON skill_event
                 WHEN NEW.kind = 'missing'
                 BEGIN
                   SELECT RAISE(ABORT, 'injected missing event failure');
                 END;",
            )
            .expect("install failure trigger");

        let result =
            catalog.mark_missing_except_scoped("claude-code", &[(Scope::AgentGlobal, root)], &[]);
        let record = catalog
            .get_skill_record(&instance.id)
            .expect("record lookup")
            .expect("record remains");

        assert!(matches!(result, Err(CatalogError::Sqlite(_))));
        assert_eq!(record.state, "loaded");
        assert!(catalog
            .list_skill_events(&instance.id, None)
            .expect("events after rollback")
            .is_empty());
    }

    #[test]
    fn tool_global_records_roundtrip_and_are_not_swept_by_adapter_missing_cleanup() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let staging_path = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");
        let mut tool_global = ClaudeCodeAdapter
            .parse(&staging_path)
            .expect("tool-global skill parses");
        tool_global.id = "tool-global-instance".to_string();
        tool_global.agent = AgentId::ToolGlobal;
        tool_global.scope = Scope::ToolGlobal;
        tool_global.project_root = None;
        tool_global.path = staging_path.clone();
        tool_global.display_path = staging_path.clone();
        tool_global.definition_id = "shared-definition".to_string();

        catalog
            .upsert_skill_instance(&tool_global)
            .expect("tool-global upserts");
        let records = catalog.list_skill_records().expect("records list");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].agent, "tool-global");
        assert_eq!(records[0].scope, "tool-global");

        let detail = catalog
            .get_skill_detail("tool-global-instance")
            .expect("detail lookup succeeds")
            .expect("detail exists");
        assert_eq!(detail.agent, "tool-global");
        assert_eq!(detail.scope, "tool-global");

        let marked = catalog
            .mark_missing_except(
                "tool-global",
                &[staging_path.parent().expect("parent").to_path_buf()],
                &[],
            )
            .expect("sweep succeeds");
        assert_eq!(marked, 0, "tool-global rows are not adapter-owned");
        let after = catalog
            .get_skill_record("tool-global-instance")
            .expect("record lookup succeeds")
            .expect("record exists");
        assert_eq!(after.state, "loaded");
    }

    #[test]
    fn tool_global_and_agent_global_same_name_remain_distinct_catalog_rows() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let path = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");
        let mut agent_global = ClaudeCodeAdapter.parse(&path).expect("agent skill parses");
        agent_global.id = "agent-global-same-name".to_string();
        agent_global.agent = AgentId::ClaudeCode;
        agent_global.scope = Scope::AgentGlobal;
        agent_global.path = path.clone();
        agent_global.display_path = path.clone();
        agent_global.definition_id = "shared-definition".to_string();

        let mut tool_global = agent_global.clone();
        tool_global.id = "tool-global-same-name".to_string();
        tool_global.agent = AgentId::ToolGlobal;
        tool_global.scope = Scope::ToolGlobal;
        tool_global.path = path
            .parent()
            .expect("parent")
            .join("tool-global-copy")
            .join("SKILL.md");
        tool_global.display_path = tool_global.path.clone();

        catalog
            .upsert_skill_instances(&[agent_global, tool_global])
            .expect("both rows upsert");
        let records = catalog.list_skill_records().expect("records list");

        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|record| {
            record.id == "agent-global-same-name" && record.agent == "claude-code"
        }));
        assert!(records.iter().any(|record| {
            record.id == "tool-global-same-name" && record.agent == "tool-global"
        }));
        assert_eq!(
            records
                .iter()
                .map(|record| record.definition_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            1,
            "same-name records share a definition for conflict presentation"
        );
    }

    #[test]
    fn project_context_sweep_skips_other_project_records_under_scanned_roots() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let scanned_root = fixture_path("fixtures/claude-code/project")
            .canonicalize()
            .expect("canonical scanned root");
        let current_project = scanned_root.join("project-a");
        let other_project = scanned_root.join("project-b");
        let current_path = fixture_path("fixtures/claude-code/project/valid-review/SKILL.md")
            .canonicalize()
            .expect("canonical current skill");
        let other_path = fixture_path("fixtures/claude-code/project/content-drift-a/SKILL.md")
            .canonicalize()
            .expect("canonical other skill");

        let mut current = ClaudeCodeAdapter
            .parse(&current_path)
            .expect("current parses");
        current.id = "current-project-record".to_string();
        current.scope = Scope::AgentProject;
        current.project_root = Some(current_project.clone());
        current.path = current_path.clone();
        let mut other = ClaudeCodeAdapter.parse(&other_path).expect("other parses");
        other.id = "other-project-record".to_string();
        other.scope = Scope::AgentProject;
        other.project_root = Some(other_project);
        other.path = other_path.clone();

        catalog
            .upsert_skill_instance(&current)
            .expect("upsert current");
        catalog.upsert_skill_instance(&other).expect("upsert other");

        let marked = catalog
            .mark_missing_except_for_project_context(
                "claude-code",
                Some(&current_project),
                std::slice::from_ref(&scanned_root),
                &[],
            )
            .expect("sweep succeeds");

        assert_eq!(marked, 1, "only the current project record is swept");
        let records = catalog.list_skill_records().expect("records");
        let current_record = records
            .iter()
            .find(|record| record.path == current_path)
            .expect("current record");
        let other_record = records
            .iter()
            .find(|record| record.path == other_path)
            .expect("other record");
        assert_eq!(current_record.state, "missing");
        assert_eq!(
            other_record.state, "loaded",
            "other project record under the scanned root is left alone"
        );
    }

    #[test]
    fn refreshes_rule_findings_and_conflict_groups() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        catalog
            .refresh_rule_findings(&[RuleFindingDraft {
                id: "finding-1".to_string(),
                instance_id: Some("inst-1".to_string()),
                definition_id: Some("def-1".to_string()),
                rule_id: "name.collision".to_string(),
                severity: "info".to_string(),
                message: "duplicate name".to_string(),
                suggestion: Some("review duplicates".to_string()),
                created_at: 42,
            }])
            .expect("findings refresh");
        let findings = catalog.list_rule_findings().expect("findings list");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "name.collision");
        assert_eq!(findings[0].triage_status, "open");
        assert!(!findings[0].triage_key.is_empty());

        catalog
            .refresh_definitions_and_conflicts(
                &[SkillDefinitionDraft {
                    id: "def-1".to_string(),
                    canonical_name: "demo".to_string(),
                    description: "demo skill".to_string(),
                    active_instance: Some("inst-1".to_string()),
                    has_multiple_instances: true,
                    has_conflict: true,
                }],
                &[ConflictGroupDraft {
                    id: "def-1:name-collision".to_string(),
                    definition_id: "def-1".to_string(),
                    reason: "name-collision".to_string(),
                    winner_id: None,
                    instance_ids: vec!["inst-1".to_string(), "inst-2".to_string()],
                }],
            )
            .expect("conflicts refresh");
        let conflicts = catalog.list_conflict_groups().expect("conflicts list");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].instance_ids.len(), 2);
    }

    #[test]
    fn refresh_rule_findings_rolls_back_on_insert_failure() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        let original = RuleFindingDraft {
            id: "finding-original".to_string(),
            instance_id: Some("inst-1".to_string()),
            definition_id: Some("def-1".to_string()),
            rule_id: "name.collision".to_string(),
            severity: "info".to_string(),
            message: "original finding".to_string(),
            suggestion: Some("keep original".to_string()),
            created_at: 42,
        };
        catalog
            .refresh_rule_findings(std::slice::from_ref(&original))
            .expect("seed finding");

        let duplicate = RuleFindingDraft {
            id: "finding-duplicate".to_string(),
            message: "replacement finding".to_string(),
            ..original
        };
        let result = catalog.refresh_rule_findings(&[duplicate.clone(), duplicate]);
        assert!(result.is_err(), "duplicate IDs should fail the refresh");

        let findings = catalog
            .list_rule_findings()
            .expect("findings list after failed refresh");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "finding-original");
        assert_eq!(findings[0].message, "original finding");
    }

    #[test]
    fn refresh_definitions_and_conflicts_rolls_back_on_insert_failure() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        catalog
            .refresh_definitions_and_conflicts(
                &[SkillDefinitionDraft {
                    id: "def-original".to_string(),
                    canonical_name: "original".to_string(),
                    description: "original definition".to_string(),
                    active_instance: Some("inst-1".to_string()),
                    has_multiple_instances: true,
                    has_conflict: true,
                }],
                &[ConflictGroupDraft {
                    id: "conflict-original".to_string(),
                    definition_id: "def-original".to_string(),
                    reason: "name-collision".to_string(),
                    winner_id: None,
                    instance_ids: vec!["inst-1".to_string(), "inst-2".to_string()],
                }],
            )
            .expect("seed definitions");

        let duplicate = SkillDefinitionDraft {
            id: "def-duplicate".to_string(),
            canonical_name: "duplicate".to_string(),
            description: "duplicate definition".to_string(),
            active_instance: None,
            has_multiple_instances: false,
            has_conflict: false,
        };
        let result =
            catalog.refresh_definitions_and_conflicts(&[duplicate.clone(), duplicate], &[]);
        assert!(result.is_err(), "duplicate IDs should fail the refresh");

        let conflicts = catalog
            .list_conflict_groups()
            .expect("conflicts list after failed refresh");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].id, "conflict-original");
        assert_eq!(conflicts[0].instance_ids, vec!["inst-1", "inst-2"]);
    }

    #[test]
    fn list_rule_findings_orders_by_severity_rank_then_rule_and_instance() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");

        catalog
            .refresh_rule_findings(&[
                RuleFindingDraft {
                    id: "finding-info".to_string(),
                    instance_id: Some("inst-1".to_string()),
                    definition_id: None,
                    rule_id: "aaa.info".to_string(),
                    severity: "info".to_string(),
                    message: "info".to_string(),
                    suggestion: None,
                    created_at: 1,
                },
                RuleFindingDraft {
                    id: "finding-warning".to_string(),
                    instance_id: Some("inst-2".to_string()),
                    definition_id: None,
                    rule_id: "bbb.warn".to_string(),
                    severity: "warning".to_string(),
                    message: "warning".to_string(),
                    suggestion: None,
                    created_at: 1,
                },
                RuleFindingDraft {
                    id: "finding-error-b".to_string(),
                    instance_id: Some("inst-2".to_string()),
                    definition_id: None,
                    rule_id: "zzz.error".to_string(),
                    severity: "error".to_string(),
                    message: "error b".to_string(),
                    suggestion: None,
                    created_at: 1,
                },
                RuleFindingDraft {
                    id: "finding-warn".to_string(),
                    instance_id: Some("inst-1".to_string()),
                    definition_id: None,
                    rule_id: "bbb.warn".to_string(),
                    severity: "warn".to_string(),
                    message: "warn".to_string(),
                    suggestion: None,
                    created_at: 1,
                },
                RuleFindingDraft {
                    id: "finding-error-a".to_string(),
                    instance_id: Some("inst-1".to_string()),
                    definition_id: None,
                    rule_id: "aaa.error".to_string(),
                    severity: "error".to_string(),
                    message: "error a".to_string(),
                    suggestion: None,
                    created_at: 1,
                },
            ])
            .expect("findings refresh");

        let ids = catalog
            .list_rule_findings()
            .expect("findings list")
            .into_iter()
            .map(|finding| finding.id)
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "finding-error-a",
                "finding-error-b",
                "finding-warn",
                "finding-warning",
                "finding-info",
            ]
        );
    }

    #[test]
    fn rule_tuning_applies_effective_severity_and_suppression_locally() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let skill = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        catalog
            .upsert_skill_instance(&skill)
            .expect("skill upserts");
        catalog
            .refresh_rule_findings(&[RuleFindingDraft {
                id: "finding-1".to_string(),
                instance_id: Some(skill.id.clone()),
                definition_id: Some(skill.definition_id.clone()),
                rule_id: "body.too-long".to_string(),
                severity: "warn".to_string(),
                message: "long body".to_string(),
                suggestion: Some("split references".to_string()),
                created_at: 1,
            }])
            .expect("findings refresh");

        catalog
            .set_rule_severity_override("body.too-long", Some("codex"), None, "info", 10)
            .expect("severity override");
        catalog
            .set_rule_suppression(
                "body.too-long",
                Some("codex"),
                None,
                "accepted local policy",
                Some("fixture note"),
                11,
            )
            .expect("suppression");

        let finding = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");
        assert_eq!(finding.severity, "warn");
        assert_eq!(finding.effective_severity, "info");
        assert_eq!(finding.severity_override.as_deref(), Some("info"));
        assert!(finding.suppressed);
        assert_eq!(
            finding.suppression_reason.as_deref(),
            Some("accepted local policy")
        );
        assert_eq!(finding.suppression_note.as_deref(), Some("fixture note"));
        assert_eq!(finding.rule_tuning_updated_at, Some(11));

        assert!(catalog
            .clear_rule_suppression("body.too-long", Some("codex"), None, 12)
            .expect("clear suppression"));
        let unsuppressed = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");
        assert!(!unsuppressed.suppressed);
        assert_eq!(unsuppressed.effective_severity, "info");
        assert!(catalog
            .clear_rule_severity_override("body.too-long", Some("codex"), None, 13)
            .expect("clear severity"));
        assert!(catalog.list_rule_tuning().expect("tuning list").is_empty());
    }

    #[test]
    fn finding_triage_persists_for_same_finding_identity() {
        let path = std::env::temp_dir().join(format!(
            "skills-copilot-triage-persist-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        {
            let catalog = Catalog::open(&path).expect("catalog opens");
            catalog.init().expect("schema initializes");
            let skill = catalog_test_instance(
                AgentId::ClaudeCode,
                Scope::AgentGlobal,
                "/tmp/home/.claude/skills/review/SKILL.md",
                "review",
                SkillState::Loaded,
            );
            catalog
                .upsert_skill_instance(&skill)
                .expect("skill upserts");
            catalog
                .refresh_rule_findings(&[RuleFindingDraft {
                    id: "finding-1".to_string(),
                    instance_id: Some(skill.id.clone()),
                    definition_id: Some(skill.definition_id.clone()),
                    rule_id: "body.too-long".to_string(),
                    severity: "warn".to_string(),
                    message: "long body".to_string(),
                    suggestion: Some("split references".to_string()),
                    created_at: 1,
                }])
                .expect("findings refresh");
            let finding = catalog
                .list_rule_findings()
                .expect("findings list")
                .pop()
                .expect("finding exists");
            catalog
                .set_finding_triage(&finding.triage_key, "reviewed", Some("checked"), 10)
                .expect("set triage")
                .expect("current finding key");
        }
        {
            let catalog = Catalog::open(&path).expect("catalog reopens");
            catalog.init().expect("schema initializes again");
            let finding = catalog
                .list_rule_findings()
                .expect("findings list")
                .pop()
                .expect("finding exists");
            assert_eq!(finding.triage_status, "reviewed");
            assert_eq!(finding.triage_note.as_deref(), Some("checked"));
            assert_eq!(finding.triage_updated_at, Some(10));
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn read_only_open_migrates_legacy_config_snapshot_schema_before_querying() {
        let path = std::env::temp_dir().join(format!(
            "skills-copilot-read-migration-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        {
            let conn = Connection::open(&path).expect("legacy catalog opens");
            conn.execute_batch(
                "CREATE TABLE config_snapshot (
                    id TEXT PRIMARY KEY,
                    agent TEXT NOT NULL,
                    scope TEXT NOT NULL,
                    target TEXT NOT NULL,
                    content TEXT NOT NULL,
                    reason TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                 );
                 INSERT INTO config_snapshot (
                    id, agent, scope, target, content, reason, created_at
                 ) VALUES (
                    'legacy-snapshot', 'claude-code', 'agent-global',
                    '/tmp/settings.json', '{}', 'pre-toggle', 1
                 );",
            )
            .expect("legacy schema seeds");
        }

        let catalog = Catalog::open_read_only_after_migration(&path)
            .expect("legacy catalog migrates before read");
        let snapshots = catalog
            .list_all_config_snapshots(None)
            .expect("migrated snapshots list");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "legacy-snapshot");
        assert_eq!(snapshots[0].project_root, None);
        assert!(
            catalog
                .conn
                .execute("DELETE FROM config_snapshot", [])
                .is_err(),
            "returned connection must remain read-only"
        );
        drop(catalog);

        let conn = Connection::open(&path).expect("migrated catalog reopens");
        assert!(schema::is_current(&conn).expect("schema version reads"));
        drop(conn);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn schema_migration_removes_legacy_runtime_and_missing_pi_document_history() {
        let path = std::env::temp_dir().join(format!(
            "skills-copilot-source-history-migration-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        let retained_id;
        {
            let catalog = Catalog::open(&path).expect("catalog opens");
            catalog.init().expect("current schema initializes");
            let runtime = catalog_test_instance(
                AgentId::Codex,
                Scope::AgentGlobal,
                "/tmp/home/.codex/.agent-copilot-runtime/runtime/SKILL.md",
                "runtime-only",
                SkillState::Loaded,
            );
            let pi_reference = catalog_test_instance(
                AgentId::Pi,
                Scope::AgentGlobal,
                "/tmp/home/.pi/agent/skills/review/references/details.md",
                "details",
                SkillState::Missing,
            );
            let retained = catalog_test_instance(
                AgentId::Pi,
                Scope::AgentGlobal,
                "/tmp/home/.pi/agent/skills/review/SKILL.md",
                "review",
                SkillState::Loaded,
            );
            retained_id = retained.id.clone();
            catalog
                .upsert_skill_instances(&[runtime.clone(), pi_reference.clone(), retained])
                .expect("legacy rows seed");
            for (id, kind) in [
                (&runtime.id, "runtime-event"),
                (&pi_reference.id, "pi-event"),
                (&retained_id, "retained-event"),
            ] {
                catalog
                    .conn
                    .execute(
                        "INSERT INTO skill_event (instance_id, kind, payload, occurred_at) VALUES (?1, ?2, '{}', 1)",
                        params![id, kind],
                    )
                    .expect("event seeds");
            }
            for (id, finding_id, triage_key) in [
                (&runtime.id, "runtime-finding", "runtime-triage"),
                (&pi_reference.id, "pi-finding", "pi-triage"),
                (&retained_id, "retained-finding", "retained-triage"),
            ] {
                catalog
                    .conn
                    .execute(
                        "INSERT INTO rule_finding (
                            id, triage_key, triage_context, instance_id, definition_id,
                            rule_id, severity, message, suggestion, created_at
                         ) VALUES (?1, ?2, 'context', ?3, NULL, 'fixture', 'warn', 'fixture', NULL, 1)",
                        params![finding_id, triage_key, id],
                    )
                    .expect("finding seeds");
                catalog
                    .conn
                    .execute(
                        "INSERT INTO finding_triage (triage_key, triage_context, status, note, updated_at)
                         VALUES (?1, 'context', 'reviewed', NULL, 1)",
                        params![triage_key],
                    )
                    .expect("triage seeds");
            }
            catalog
                .conn
                .pragma_update(None, "user_version", 6)
                .expect("simulate v6 catalog");
        }

        let catalog = Catalog::open(&path).expect("catalog reopens");
        catalog.init().expect("history cleanup migration runs");
        let remaining_ids = catalog
            .conn
            .prepare("SELECT id FROM skill_instance ORDER BY id")
            .expect("instance query prepares")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("instance rows query")
            .collect::<Result<Vec<_>, _>>()
            .expect("instance rows decode");
        assert_eq!(remaining_ids, vec![retained_id.clone()]);
        assert_eq!(
            catalog
                .conn
                .query_row("SELECT COUNT(*) FROM skill_event", [], |row| row
                    .get::<_, i64>(0))
                .expect("event count"),
            1
        );
        assert_eq!(
            catalog
                .conn
                .query_row("SELECT COUNT(*) FROM rule_finding", [], |row| row
                    .get::<_, i64>(0))
                .expect("finding count"),
            1
        );
        assert_eq!(
            catalog
                .conn
                .query_row("SELECT COUNT(*) FROM finding_triage", [], |row| row
                    .get::<_, i64>(0))
                .expect("triage count"),
            1
        );
        assert!(schema::is_current(&catalog.conn).expect("schema is current"));
        drop(catalog);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn finding_triage_reopens_when_instance_fingerprint_changes() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let mut skill = catalog_test_instance(
            AgentId::ClaudeCode,
            Scope::AgentGlobal,
            "/tmp/home/.claude/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        skill.fingerprint = "fingerprint-a".to_string();
        catalog
            .upsert_skill_instance(&skill)
            .expect("skill upserts");
        let finding = RuleFindingDraft {
            id: "finding-1".to_string(),
            instance_id: Some(skill.id.clone()),
            definition_id: Some(skill.definition_id.clone()),
            rule_id: "fingerprint.changed".to_string(),
            severity: "info".to_string(),
            message: "Skill content fingerprint changed since the previous scan.".to_string(),
            suggestion: Some(
                "Review the skill details before relying on this version.".to_string(),
            ),
            created_at: 1,
        };
        catalog
            .refresh_rule_findings(std::slice::from_ref(&finding))
            .expect("findings refresh");
        let first = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");
        catalog
            .set_finding_triage(&first.triage_key, "ignored", None, 11)
            .expect("set triage");

        skill.fingerprint = "fingerprint-b".to_string();
        catalog
            .upsert_skill_instance(&skill)
            .expect("skill upserts with new fingerprint");
        catalog
            .refresh_rule_findings(&[finding])
            .expect("findings refresh after fingerprint change");
        let reopened = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");

        assert_ne!(reopened.triage_key, first.triage_key);
        assert_eq!(reopened.triage_status, "open");
        assert_eq!(catalog.list_finding_triage().expect("triage list").len(), 1);
    }

    #[test]
    fn finding_triage_reopens_when_definition_instance_set_changes() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        let first = catalog_test_instance(
            AgentId::ClaudeCode,
            Scope::AgentGlobal,
            "/tmp/home/.claude/skills/review-a/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        let mut second = catalog_test_instance(
            AgentId::ClaudeCode,
            Scope::AgentProject,
            "/tmp/project/.claude/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        second.definition_id = first.definition_id.clone();
        catalog
            .upsert_skill_instances(&[first.clone(), second.clone()])
            .expect("skills upsert");
        let finding = RuleFindingDraft {
            id: "finding-1".to_string(),
            instance_id: None,
            definition_id: Some(first.definition_id.clone()),
            rule_id: "name.collision".to_string(),
            severity: "warn".to_string(),
            message: "runtime sees skill name in multiple locations".to_string(),
            suggestion: Some("compare copies".to_string()),
            created_at: 1,
        };
        catalog
            .refresh_rule_findings(std::slice::from_ref(&finding))
            .expect("findings refresh");
        let original = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");
        catalog
            .set_finding_triage(&original.triage_key, "reviewed", None, 12)
            .expect("set triage");

        let mut third = catalog_test_instance(
            AgentId::ClaudeCode,
            Scope::AgentProject,
            "/tmp/other/.claude/skills/review/SKILL.md",
            "review",
            SkillState::Loaded,
        );
        third.definition_id = first.definition_id.clone();
        catalog.upsert_skill_instance(&third).expect("third upsert");
        catalog
            .refresh_rule_findings(&[finding])
            .expect("findings refresh after member change");
        let reopened = catalog
            .list_rule_findings()
            .expect("findings list")
            .pop()
            .expect("finding exists");

        assert_ne!(reopened.triage_key, original.triage_key);
        assert_eq!(reopened.triage_status, "open");
    }

    #[test]
    fn strict_read_only_open_rejects_outdated_schema_without_migration() {
        let path = std::env::temp_dir().join(format!(
            "skills-copilot-catalog-strict-read-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        let catalog = Catalog::open(&path).expect("catalog opens");
        catalog.init().expect("schema initializes");
        catalog
            .conn
            .pragma_update(None, "user_version", 1_i64)
            .expect("mark catalog outdated");
        drop(catalog);
        let before = std::fs::read(&path).expect("read outdated catalog");

        let result = Catalog::open_read_only_current(&path);

        assert!(matches!(result, Err(CatalogError::SchemaOutdated)));
        assert_eq!(
            std::fs::read(&path).expect("read catalog after strict open"),
            before
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_to_product_projection_schema_does_not_invent_coverage() {
        let path = std::env::temp_dir().join(format!(
            "skills-copilot-product-projection-migration-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        let catalog = Catalog::open(&path).expect("catalog opens");
        catalog.init().expect("schema initializes");
        let skill = catalog_test_instance(
            AgentId::Codex,
            Scope::AgentGlobal,
            "/tmp/home/.codex/skills/existing/SKILL.md",
            "existing",
            SkillState::Loaded,
        );
        catalog
            .upsert_skill_instance(&skill)
            .expect("seed pre-migration skill");
        catalog
            .conn
            .execute_batch(
                "DROP TABLE catalog_skill_projection;
                 DROP TABLE catalog_scan_coverage;",
            )
            .expect("remove product projection tables");
        catalog
            .conn
            .pragma_update(None, "user_version", 8_i64)
            .expect("mark catalog as schema v8");
        drop(catalog);

        let migrated =
            Catalog::open_read_only_after_migration(&path).expect("migrate catalog for read");
        assert_eq!(
            migrated
                .list_skill_records()
                .expect("pre-existing skill survives")
                .len(),
            1
        );
        assert!(
            migrated
                .list_catalog_scan_coverages("context-fixture")
                .expect("coverage rows")
                .is_empty(),
            "migration must not reinterpret historical presence as complete scan evidence"
        );
        assert!(
            migrated
                .list_catalog_skill_projections("context-fixture")
                .expect("projection rows")
                .is_empty(),
            "migration must not invent logical source or effectiveness metadata"
        );
        drop(migrated);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    #[cfg(unix)]
    fn catalog_open_rejects_a_symlink_without_mutating_its_target() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "skills-copilot-catalog-symlink-open-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir_all(&root).expect("create catalog test root");
        let target = root.join("victim.sqlite");
        let link = root.join("catalog.sqlite");
        let catalog = Catalog::open(&target).expect("create target catalog");
        catalog.init().expect("initialize target catalog");
        drop(catalog);
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644))
            .expect("set observable target mode");
        let bytes = std::fs::read(&target).expect("target bytes before rejection");
        symlink(&target, &link).expect("create catalog symlink");

        let writable = Catalog::open(&link);
        let read_only = Catalog::open_read_only_current(&link);

        assert!(writable.is_err());
        assert!(read_only.is_err());
        assert_eq!(
            std::fs::read(&target).expect("target bytes after rejection"),
            bytes
        );
        assert_eq!(
            std::fs::metadata(&target)
                .expect("target metadata")
                .permissions()
                .mode()
                & 0o777,
            0o644
        );
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn classified_precommit_failure_proves_transaction_not_committed() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        catalog.inject_next_commit_failure_for_test();
        let transaction = catalog
            .begin_immediate_transaction()
            .expect("begin transaction");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "classified-precommit",
                agent: "claude-code",
                scope: "agent-global",
                project_root: None,
                target: "/tmp/settings.json",
                content: "{}\n",
                reason: "test",
                created_at_ms: 1,
            })
            .expect("create snapshot");

        let result = transaction.commit_classified();

        assert!(matches!(
            result,
            Err(CatalogCommitError::NotCommitted(
                CatalogError::InjectedCommitFailure
            ))
        ));
        assert!(catalog
            .get_config_snapshot("classified-precommit")
            .expect("read snapshot")
            .is_none());
    }

    #[test]
    fn explicit_rollback_failure_is_reported() {
        let catalog = Catalog::in_memory().expect("catalog opens");
        catalog.init().expect("schema initializes");
        catalog.inject_next_rollback_failure_for_test();
        let transaction = catalog
            .begin_immediate_transaction()
            .expect("begin transaction");

        assert!(matches!(
            transaction.rollback(),
            Err(CatalogError::InjectedRollbackFailure)
        ));
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_and_journal_stay_on_the_opened_owner_after_path_replacement() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-catalog-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let owner_path = root.join("app-data");
        let moved_owner = root.join("accepted-owner");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner_path).expect("create owner");
        std::fs::create_dir(&victim).expect("create victim");
        std::fs::write(victim.join("sentinel"), "unchanged").expect("seed victim");
        let owner = std::fs::File::open(&owner_path).expect("open accepted owner");
        let late_owner = owner.try_clone().expect("clone accepted owner");
        let catalog = Catalog::open_anchored(owner).expect("open anchored catalog");
        catalog.init().expect("initialize anchored schema");

        std::fs::rename(&owner_path, &moved_owner).expect("move accepted owner");
        symlink(&victim, &owner_path).expect("replace display path");

        let transaction = catalog
            .begin_immediate_transaction()
            .expect("begin anchored transaction");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "anchored-snapshot",
                agent: "claude-code",
                scope: "agent-global",
                project_root: None,
                target: "/tmp/settings.json",
                content: "{}\n",
                reason: "anchored test",
                created_at_ms: 1,
            })
            .expect("write anchored row");
        assert!(
            moved_owner.join("catalog.sqlite-journal").exists(),
            "rollback journal must be created beside the accepted catalog inode"
        );
        assert!(
            !victim.join("catalog.sqlite-journal").exists(),
            "rollback journal must not follow the replaced display path"
        );
        transaction.commit().expect("commit anchored transaction");
        drop(catalog);

        assert!(moved_owner.join("catalog.sqlite").exists());
        assert!(!victim.join("catalog.sqlite").exists());
        assert_eq!(
            std::fs::read_to_string(victim.join("sentinel")).expect("victim sentinel"),
            "unchanged"
        );
        let read_only = Catalog::open_read_only_current_anchored(late_owner)
            .expect("read anchored current catalog");
        assert!(read_only
            .get_config_snapshot("anchored-snapshot")
            .expect("read anchored snapshot")
            .is_some());
        drop(read_only);

        let _ = std::fs::remove_file(owner_path);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn mutation_owner_binding_rejects_a_rebound_owner_for_anchored_and_legacy_catalogs() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-catalog-owner-binding-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let owner_path = root.join("app-data");
        let moved_owner = root.join("accepted-owner");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner_path).expect("create owner");
        std::fs::create_dir(&victim).expect("create victim");
        std::fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");

        let accepted = std::fs::File::open(&owner_path).expect("open accepted owner");
        let catalog = Catalog::open_anchored(
            accepted
                .try_clone()
                .expect("clone accepted owner for catalog"),
        )
        .expect("open anchored catalog");
        assert!(catalog.ensure_mutation_owner(&accepted).is_ok());

        std::fs::rename(&owner_path, &moved_owner).expect("move accepted owner");
        symlink(&victim, &owner_path).expect("replace owner path");
        let rebound = std::fs::File::open(&owner_path).expect("open rebound owner");
        assert!(matches!(
            catalog.ensure_mutation_owner(&rebound),
            Err(CatalogError::MutationOwner(_))
        ));
        assert_eq!(
            std::fs::read(victim.join("sentinel")).expect("victim sentinel"),
            b"unchanged"
        );
        assert!(!victim.join("catalog.sqlite").exists());
        drop(catalog);

        let legacy =
            Catalog::open(&moved_owner.join("legacy.sqlite")).expect("open legacy catalog");
        assert!(legacy.ensure_mutation_owner(&accepted).is_ok());
        assert!(matches!(
            legacy.ensure_mutation_owner(&rebound),
            Err(CatalogError::MutationOwner(_))
        ));

        let in_memory = Catalog::in_memory().expect("open in-memory catalog");
        assert!(in_memory.ensure_mutation_owner(&rebound).is_ok());

        let _ = std::fs::remove_file(owner_path);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_rejects_main_and_journal_symlinks_without_touching_victims() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-catalog-links-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let main_owner = root.join("main-owner");
        let main_victim = root.join("main-victim.sqlite");
        std::fs::create_dir_all(&main_owner).expect("create main owner");
        std::fs::write(&main_victim, b"victim-main-bytes").expect("seed main victim");
        std::fs::set_permissions(&main_victim, std::fs::Permissions::from_mode(0o640))
            .expect("set main victim mode");
        symlink(&main_victim, main_owner.join("catalog.sqlite")).expect("link main catalog victim");
        let main_before = std::fs::read(&main_victim).expect("read main victim");
        let main_mode = std::fs::metadata(&main_victim)
            .expect("main victim metadata")
            .permissions()
            .mode()
            & 0o777;
        let main_result =
            Catalog::open_anchored(std::fs::File::open(&main_owner).expect("open main owner"));
        assert!(
            main_result.is_err(),
            "main catalog symlink must fail closed"
        );
        let read_result = Catalog::open_read_only_current_anchored_if_exists(
            std::fs::File::open(&main_owner).expect("reopen main owner"),
        );
        assert!(
            read_result.is_err(),
            "read-only catalog symlink must fail closed"
        );
        assert_eq!(
            std::fs::read(&main_victim).expect("main victim after rejection"),
            main_before
        );
        assert_eq!(
            std::fs::metadata(&main_victim)
                .expect("main victim metadata after rejection")
                .permissions()
                .mode()
                & 0o777,
            main_mode
        );

        let journal_owner = root.join("journal-owner");
        let journal_victim = root.join("journal-victim");
        std::fs::create_dir(&journal_owner).expect("create journal owner");
        std::fs::write(&journal_victim, b"victim-journal-bytes").expect("seed journal victim");
        std::fs::set_permissions(&journal_victim, std::fs::Permissions::from_mode(0o640))
            .expect("set journal victim mode");
        let catalog = Catalog::open_anchored(
            std::fs::File::open(&journal_owner).expect("open journal owner"),
        )
        .expect("open safe main catalog");
        catalog.init().expect("initialize safe main catalog");
        symlink(
            &journal_victim,
            journal_owner.join("catalog.sqlite-journal"),
        )
        .expect("link journal victim");
        let journal_before = std::fs::read(&journal_victim).expect("read journal victim");
        let journal_mode = std::fs::metadata(&journal_victim)
            .expect("journal victim metadata")
            .permissions()
            .mode()
            & 0o777;
        assert!(
            catalog.begin_immediate_transaction().is_err(),
            "pre-existing journal symlink must prevent a transaction"
        );
        assert_eq!(
            std::fs::read(&journal_victim).expect("journal victim after rejection"),
            journal_before
        );
        assert_eq!(
            std::fs::metadata(&journal_victim)
                .expect("journal victim metadata after rejection")
                .permissions()
                .mode()
                & 0o777,
            journal_mode
        );
        assert!(
            journal_owner.join("catalog.sqlite-journal").is_symlink(),
            "fail-closed handling must not delete or replace the suspicious link"
        );
        drop(catalog);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_rejects_main_and_journal_hardlinks_without_touching_victims() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-catalog-hardlinks-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let main_owner = root.join("main-owner");
        let main_victim = root.join("main-victim.sqlite");
        std::fs::create_dir_all(&main_owner).expect("create main owner");
        std::fs::write(&main_victim, b"victim-main-bytes").expect("seed main victim");
        std::fs::set_permissions(&main_victim, std::fs::Permissions::from_mode(0o640))
            .expect("set main victim mode");
        std::fs::hard_link(&main_victim, main_owner.join("catalog.sqlite"))
            .expect("hardlink main catalog victim");
        let main_before = std::fs::read(&main_victim).expect("read main victim");
        let main_metadata = std::fs::metadata(&main_victim).expect("main victim metadata");
        assert_eq!(main_metadata.nlink(), 2);
        let main_result =
            Catalog::open_anchored(std::fs::File::open(&main_owner).expect("open main owner"));
        assert!(
            main_result.is_err(),
            "main catalog hardlink must fail closed"
        );
        assert_eq!(
            std::fs::read(&main_victim).expect("main victim after rejection"),
            main_before
        );
        assert_eq!(
            std::fs::metadata(&main_victim)
                .expect("main victim metadata after rejection")
                .permissions()
                .mode()
                & 0o777,
            main_metadata.permissions().mode() & 0o777
        );

        let journal_owner = root.join("journal-owner");
        let journal_victim = root.join("journal-victim");
        std::fs::create_dir(&journal_owner).expect("create journal owner");
        std::fs::write(&journal_victim, b"victim-journal-bytes").expect("seed journal victim");
        std::fs::set_permissions(&journal_victim, std::fs::Permissions::from_mode(0o640))
            .expect("set journal victim mode");
        let catalog = Catalog::open_anchored(
            std::fs::File::open(&journal_owner).expect("open journal owner"),
        )
        .expect("open safe main catalog");
        catalog.init().expect("initialize safe main catalog");
        std::fs::hard_link(
            &journal_victim,
            journal_owner.join("catalog.sqlite-journal"),
        )
        .expect("hardlink journal victim");
        let journal_before = std::fs::read(&journal_victim).expect("read journal victim");
        let journal_metadata = std::fs::metadata(&journal_victim).expect("journal victim metadata");
        assert_eq!(journal_metadata.nlink(), 2);

        assert!(
            catalog.begin_immediate_transaction().is_err(),
            "pre-existing journal hardlink must prevent a transaction"
        );
        assert_eq!(
            std::fs::read(&journal_victim).expect("journal victim after rejection"),
            journal_before
        );
        assert_eq!(
            std::fs::metadata(&journal_victim)
                .expect("journal victim metadata after rejection")
                .permissions()
                .mode()
                & 0o777,
            journal_metadata.permissions().mode() & 0o777
        );
        drop(catalog);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_preserves_multi_connection_posix_locks() {
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-catalog-locks-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir(&root).expect("create owner");
        let first = Catalog::open_anchored(
            std::fs::File::open(&root).expect("open first owner descriptor"),
        )
        .expect("open first catalog");
        first.init().expect("initialize catalog");
        let second = Catalog::open_anchored(
            std::fs::File::open(&root).expect("open second owner descriptor"),
        )
        .expect("open second catalog");
        let first_transaction = first
            .begin_immediate_transaction()
            .expect("first connection reserves the writer lock");

        let transient = Catalog::open_read_only_current_anchored(
            std::fs::File::open(&root).expect("open transient owner descriptor"),
        )
        .expect("open transient read-only catalog");
        drop(transient);
        assert!(
            second.begin_immediate_transaction().is_err(),
            "opening and closing another descriptor must not release the first writer lock"
        );

        first_transaction
            .rollback()
            .expect("release first writer lock");
        second
            .begin_immediate_transaction()
            .expect("second writer acquires released lock")
            .commit()
            .expect("commit second transaction");
        drop(second);
        drop(first);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn path_and_anchored_catalog_modes_cannot_mix_for_one_owner_child() {
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-catalog-open-mode-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir(&root).expect("create owner");
        let path = root.join("catalog.sqlite");

        let path_open = Catalog::open(&path).expect("open path catalog");
        path_open.init().expect("initialize path catalog");
        assert!(
            Catalog::open_anchored(
                std::fs::File::open(&root).expect("open anchored owner descriptor")
            )
            .is_err(),
            "anchored open must be rejected while a path-open connection owns the target"
        );
        drop(path_open);

        let anchored =
            Catalog::open_anchored(std::fs::File::open(&root).expect("open owner descriptor"))
                .expect("open anchored catalog after path connection closes");
        assert!(
            Catalog::open_read_only(&path).is_err(),
            "path open must be rejected while an anchored connection owns the target"
        );
        drop(anchored);

        Catalog::open_read_only_current(&path)
            .expect("path mode can reopen after anchored connection closes");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn path_catalog_preserves_multi_connection_posix_locks() {
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-path-catalog-locks-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir(&root).expect("create owner");
        let path = root.join("catalog.sqlite");
        let first = Catalog::open(&path).expect("open first path catalog");
        first.init().expect("initialize path catalog");
        let second = Catalog::open(&path).expect("open second path catalog");
        let first_transaction = first
            .begin_immediate_transaction()
            .expect("first path connection reserves the writer lock");

        let transient =
            Catalog::open_read_only_current(&path).expect("open transient read-only path catalog");
        drop(transient);
        assert!(
            second.begin_immediate_transaction().is_err(),
            "closing a path-authority descriptor must not release another writer lock"
        );

        first_transaction
            .rollback()
            .expect("release first path writer lock");
        second
            .begin_immediate_transaction()
            .expect("second path writer acquires released lock")
            .commit()
            .expect("commit second path transaction");
        drop(second);
        drop(first);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn path_catalog_preserves_parent_vfs_wal_support() {
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-path-catalog-wal-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir(&root).expect("create owner");
        let path = root.join("catalog.sqlite");
        let first = Catalog::open(&path).expect("open path catalog");
        first.init().expect("initialize path catalog");
        let journal_mode = first
            .conn
            .query_row("PRAGMA journal_mode = WAL", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("enable WAL through the path compatibility VFS");
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

        first
            .conn
            .execute_batch(
                "CREATE TABLE path_wal_probe(value INTEGER NOT NULL);
                 INSERT INTO path_wal_probe(value) VALUES (42);",
            )
            .expect("write through WAL");
        let second = Catalog::open_read_only(&path).expect("open WAL reader");
        assert_eq!(
            second
                .conn
                .query_row("SELECT value FROM path_wal_probe", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("read WAL row"),
            42
        );
        drop(second);
        let journal_mode = first
            .conn
            .query_row("PRAGMA journal_mode = DELETE", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("restore rollback journal mode");
        assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
        drop(first);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn path_catalog_open_stays_on_one_parent_descriptor_across_rebind() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-path-catalog-rebind-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let owner_path = root.join("app-data");
        let accepted_owner = root.join("accepted-owner");
        let victim = root.join("victim");
        let catalog_path = owner_path.join("catalog.sqlite");
        std::fs::create_dir_all(&owner_path).expect("create accepted owner");
        std::fs::create_dir(&victim).expect("create victim");
        std::fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");

        let catalog = Catalog::open_path_anchored_with_hook(
            &catalog_path,
            OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            || {
                std::fs::rename(&owner_path, &accepted_owner).expect("move accepted owner");
                symlink(&victim, &owner_path).expect("rebind display owner to victim");
            },
        )
        .expect("path-authority catalog stays on accepted owner");
        catalog.init().expect("initialize accepted catalog");

        let accepted = std::fs::File::open(&accepted_owner).expect("open accepted owner");
        let rebound = std::fs::File::open(&owner_path).expect("open rebound victim owner");
        assert!(catalog.ensure_mutation_owner(&accepted).is_ok());
        assert!(matches!(
            catalog.ensure_mutation_owner(&rebound),
            Err(CatalogError::MutationOwner(_))
        ));
        assert!(
            Catalog::open_anchored(
                std::fs::File::open(&accepted_owner).expect("reopen accepted owner")
            )
            .is_err(),
            "the path-authority lease must remain bound to the accepted owner"
        );
        assert!(accepted_owner.join("catalog.sqlite").exists());
        assert!(!victim.join("catalog.sqlite").exists());
        assert_eq!(
            std::fs::read(victim.join("sentinel")).expect("read victim sentinel"),
            b"unchanged"
        );
        drop(catalog);

        let _ = std::fs::remove_file(owner_path);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn path_catalog_opens_beneath_a_shared_temp_parent() {
        let path = PathBuf::from("/tmp").join(format!(
            "ac-path-catalog-{}-{}.sqlite",
            std::process::id(),
            current_time_for_test()
        ));
        let catalog = Catalog::open(&path).expect("open path catalog under shared temp parent");
        catalog.init().expect("initialize path catalog");
        assert!(path.exists());
        drop(catalog);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_rejects_special_main_and_journal_nodes() {
        use std::os::unix::{fs::FileTypeExt, net::UnixListener};

        let root = PathBuf::from("/tmp").join(format!(
            "ac-vfs-special-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let main_owner = root.join("main-owner");
        std::fs::create_dir_all(&main_owner).expect("create main owner");
        let main_path = main_owner.join("catalog.sqlite");
        let main_socket = UnixListener::bind(&main_path).expect("bind main socket");
        assert!(
            Catalog::open_anchored(
                std::fs::File::open(&main_owner).expect("open main owner descriptor")
            )
            .is_err(),
            "a socket at the main database name must fail closed"
        );
        assert!(std::fs::symlink_metadata(&main_path)
            .expect("main socket metadata")
            .file_type()
            .is_socket());
        drop(main_socket);

        let journal_owner = root.join("journal-owner");
        std::fs::create_dir(&journal_owner).expect("create journal owner");
        let catalog = Catalog::open_anchored(
            std::fs::File::open(&journal_owner).expect("open journal owner descriptor"),
        )
        .expect("open safe main catalog");
        catalog.init().expect("initialize main catalog");
        let journal_path = journal_owner.join("catalog.sqlite-journal");
        let journal_socket = UnixListener::bind(&journal_path).expect("bind journal socket");
        assert!(
            catalog.begin_immediate_transaction().is_err(),
            "a socket at the rollback journal name must fail closed"
        );
        assert!(std::fs::symlink_metadata(&journal_path)
            .expect("journal socket metadata")
            .file_type()
            .is_socket());
        drop(journal_socket);
        drop(catalog);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_vfs_unregisters_after_its_catalog_closes() {
        use std::ffi::CString;

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-vfs-drop-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        std::fs::create_dir(&root).expect("create owner");
        let catalog =
            Catalog::open_anchored(std::fs::File::open(&root).expect("open owner descriptor"))
                .expect("open anchored catalog");
        let name = catalog
            ._anchored_vfs
            .as_ref()
            .expect("anchored VFS lease")
            .name()
            .to_string();
        let name = CString::new(name).expect("VFS name");
        assert!(
            !unsafe { rusqlite::ffi::sqlite3_vfs_find(name.as_ptr()) }.is_null(),
            "registered VFS must remain visible while the catalog is alive"
        );

        drop(catalog);

        assert!(
            unsafe { rusqlite::ffi::sqlite3_vfs_find(name.as_ptr()) }.is_null(),
            "catalog drop must unregister the now-idle VFS"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn anchored_catalog_handles_rollback_temp_and_read_only_without_path_escape() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "agent-copilot-anchored-catalog-behavior-{}-{}",
            std::process::id(),
            current_time_for_test()
        ));
        let owner = root.join("owner");
        let escaped = root.join("escaped.sqlite");
        std::fs::create_dir_all(&owner).expect("create owner");
        let catalog =
            Catalog::open_anchored(std::fs::File::open(&owner).expect("open owner descriptor"))
                .expect("open catalog");
        catalog.init().expect("initialize catalog");
        assert_eq!(
            std::fs::metadata(owner.join("catalog.sqlite"))
                .expect("main catalog metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        let relative_escape = catalog
            .conn
            .execute("ATTACH DATABASE '../escaped.sqlite' AS escaped", []);
        assert!(
            relative_escape.is_err(),
            "parent-relative ATTACH must fail closed"
        );
        let absolute_escape = catalog.conn.execute(
            "ATTACH DATABASE ?1 AS escaped",
            [escaped.to_string_lossy().as_ref()],
        );
        assert!(absolute_escape.is_err(), "absolute ATTACH must fail closed");
        assert!(!escaped.exists(), "failed ATTACH must not create a file");

        catalog
            .conn
            .execute_batch(
                "PRAGMA temp_store=FILE;
                 CREATE TEMP TABLE anchored_temp(value TEXT);
                 INSERT INTO anchored_temp(value) VALUES ('temporary');",
            )
            .expect("descriptor-safe unnamed temp database works");
        assert_eq!(
            catalog
                .conn
                .query_row("SELECT value FROM anchored_temp", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("read temp row"),
            "temporary"
        );
        assert!(
            std::fs::read_dir(&owner).expect("read owner").all(|entry| {
                !entry
                    .expect("owner entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".agent-copilot-sqlite-temp-")
            }),
            "DELETEONCLOSE temp children must be unlinked"
        );

        let rollback = catalog
            .begin_immediate_transaction()
            .expect("begin rollback transaction");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "rolled-back",
                agent: "claude-code",
                scope: "agent-global",
                project_root: None,
                target: "/tmp/settings.json",
                content: "{}\n",
                reason: "rollback test",
                created_at_ms: 1,
            })
            .expect("write rollback row");
        assert!(owner.join("catalog.sqlite-journal").exists());
        assert_eq!(
            std::fs::metadata(owner.join("catalog.sqlite-journal"))
                .expect("journal metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        rollback.rollback().expect("rollback transaction");
        assert!(!owner.join("catalog.sqlite-journal").exists());
        assert!(catalog
            .get_config_snapshot("rolled-back")
            .expect("read rolled-back row")
            .is_none());

        let commit = catalog
            .begin_immediate_transaction()
            .expect("begin commit transaction");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "committed",
                agent: "claude-code",
                scope: "agent-global",
                project_root: None,
                target: "/tmp/settings.json",
                content: "{}\n",
                reason: "commit test",
                created_at_ms: 2,
            })
            .expect("write committed row");
        commit.commit().expect("commit transaction");
        assert!(!owner.join("catalog.sqlite-journal").exists());

        let wal_mode = catalog
            .conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0));
        if let Ok(mode) = wal_mode {
            assert_ne!(mode.to_ascii_lowercase(), "wal");
        }
        assert!(!owner.join("catalog.sqlite-wal").exists());
        assert!(!owner.join("catalog.sqlite-shm").exists());

        let read_only = Catalog::open_read_only_current_anchored(
            std::fs::File::open(&owner).expect("open read-only owner descriptor"),
        )
        .expect("open read-only catalog");
        assert!(read_only
            .get_config_snapshot("committed")
            .expect("read committed row")
            .is_some());
        assert!(
            read_only
                .conn
                .execute("CREATE TABLE forbidden(value TEXT)", [])
                .is_err(),
            "read-only connection must reject writes"
        );
        drop(read_only);
        drop(catalog);

        let _ = std::fs::remove_dir_all(root);
    }

    fn current_time_for_test() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
