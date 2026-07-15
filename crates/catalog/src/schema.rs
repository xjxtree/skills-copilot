use rusqlite::Connection;

use super::CatalogError;

const INITIAL_SCHEMA: &str = include_str!("migrations/0001_initial.sql");
const MIGRATION_0002: &str = include_str!("migrations/0002_add_display_path.sql");
const MIGRATION_0003: &str = include_str!("migrations/0003_add_rule_findings.sql");
const MIGRATION_0004: &str = include_str!("migrations/0004_add_finding_triage.sql");
const MIGRATION_0005: &str = include_str!("migrations/0005_add_rule_tuning.sql");
const MIGRATION_0006: &str = include_str!("migrations/0006_add_config_snapshot_project_root.sql");
const SCHEMA_VERSION: i64 = 6;

pub(crate) fn init_schema(conn: &Connection) -> Result<(), CatalogError> {
    if is_current(conn)? {
        return Ok(());
    }
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = init_schema_in_transaction(conn);
    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(error);
        }
    }
    Ok(())
}

fn init_schema_in_transaction(conn: &Connection) -> Result<(), CatalogError> {
    conn.execute_batch(INITIAL_SCHEMA)?;
    apply_column_migration_if_missing(conn, "skill_instance", "display_path", MIGRATION_0002)?;
    conn.execute_batch(MIGRATION_0003)?;
    ensure_rule_finding_triage_columns(conn)?;
    conn.execute_batch(MIGRATION_0004)?;
    conn.execute_batch(MIGRATION_0005)?;
    apply_column_migration_if_missing(conn, "config_snapshot", "project_root", MIGRATION_0006)?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_config_snapshot_project
         ON config_snapshot(scope, project_root, created_at);",
    )?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

pub(crate) fn is_current(conn: &Connection) -> Result<bool, CatalogError> {
    let version = conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?;
    if version < SCHEMA_VERSION {
        return Ok(false);
    }
    for (table, column) in [
        ("skill_instance", "display_path"),
        ("rule_finding", "triage_key"),
        ("rule_finding", "triage_context"),
        ("config_snapshot", "project_root"),
    ] {
        if !table_has_column(conn, table, column)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn apply_column_migration_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    migration_sql: &str,
) -> Result<(), CatalogError> {
    if !table_has_column(conn, table, column)? {
        conn.execute_batch(migration_sql)?;
    }
    Ok(())
}

fn ensure_rule_finding_triage_columns(conn: &Connection) -> Result<(), CatalogError> {
    if !table_has_column(conn, "rule_finding", "triage_key")? {
        conn.execute(
            "ALTER TABLE rule_finding ADD COLUMN triage_key TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !table_has_column(conn, "rule_finding", "triage_context")? {
        conn.execute(
            "ALTER TABLE rule_finding ADD COLUMN triage_context TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, CatalogError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
