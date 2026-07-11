use super::*;

impl Catalog {
    pub fn instance_fingerprints(&self) -> Result<HashMap<String, String>, CatalogError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, fingerprint FROM skill_instance")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut fingerprints = HashMap::new();
        for row in rows {
            let (id, fingerprint) = row?;
            fingerprints.insert(id, fingerprint);
        }
        Ok(fingerprints)
    }

    pub fn list_skill_records(&self) -> Result<Vec<SkillRecord>, CatalogError> {
        self.list_skill_records_with_project_context(None)
    }

    pub fn list_skill_instances_for_project_context(
        &self,
        current_project_root: Option<&Path>,
    ) -> Result<Vec<SkillInstance>, CatalogError> {
        self.list_skill_instances_with_project_context(Some(current_project_root))
    }

    fn list_skill_instances_with_project_context(
        &self,
        project_context: Option<Option<&Path>>,
    ) -> Result<Vec<SkillInstance>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, project_root, path, COALESCE(display_path, path),
                definition_id, name, description, version, state, enabled,
                frontmatter_raw, body, permissions, fingerprint, mtime, first_seen, last_seen
         FROM skill_instance
         ORDER BY agent, scope, name",
        )?;
        let rows = stmt.query_map([], skill_instance_from_row)?;

        let mut instances = Vec::new();
        for row in rows {
            let instance = row?;
            if record_matches_project_context(
                instance.scope.as_str(),
                instance.project_root.as_deref().and_then(Path::to_str),
                project_context,
            ) && catalog_path_has_skill_shape(instance.agent.as_str(), &instance.path)
            {
                instances.push(instance);
            }
        }
        Ok(dedup_skill_instances(instances))
    }

    /// List skill records visible for the current project context.
    ///
    /// Agent-global and tool-global rows are always visible. Agent-project rows
    /// are visible only when their recorded `project_root` matches the active
    /// project context. In no-project mode, project rows remain in the catalog
    /// but are hidden from current UI/service views.
    pub fn list_skill_records_for_project_context(
        &self,
        current_project_root: Option<&Path>,
    ) -> Result<Vec<SkillRecord>, CatalogError> {
        self.list_skill_records_with_project_context(Some(current_project_root))
    }

    fn list_skill_records_with_project_context(
        &self,
        project_context: Option<Option<&Path>>,
    ) -> Result<Vec<SkillRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
        "SELECT id, agent, scope, project_root, path, COALESCE(display_path, path), definition_id, name, state, enabled FROM skill_instance ORDER BY agent, scope, name",
    )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                SkillRecord {
                    id: row.get(0)?,
                    agent: row.get(1)?,
                    scope: row.get(2)?,
                    path: PathBuf::from(row.get::<_, String>(4)?),
                    display_path: PathBuf::from(row.get::<_, String>(5)?),
                    definition_id: row.get(6)?,
                    name: row.get(7)?,
                    state: row.get(8)?,
                    enabled: row.get::<_, i64>(9)? != 0,
                },
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        let mut records = Vec::new();
        for row in rows {
            let (record, project_root) = row?;
            if record_matches_project_context(
                &record.scope,
                project_root.as_deref(),
                project_context,
            ) && catalog_path_has_skill_shape(&record.agent, &record.path)
            {
                records.push(record);
            }
        }
        Ok(dedup_skill_records(records))
    }

    pub fn get_skill_instance_meta(
        &self,
        id: &str,
    ) -> Result<Option<SkillInstanceMeta>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, project_root, path, name, enabled
         FROM skill_instance WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            let agent_str: String = row.get(1)?;
            let scope_str: String = row.get(2)?;
            let agent = parse_agent_id(&agent_str).ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!("unknown agent: {agent_str}"))
            })?;
            let scope = parse_scope(&scope_str).ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!("unknown scope: {scope_str}"))
            })?;
            Ok(SkillInstanceMeta {
                id: row.get(0)?,
                agent,
                scope,
                project_root: row.get::<_, Option<String>>(3)?.map(PathBuf::from),
                path: PathBuf::from(row.get::<_, String>(4)?),
                name: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_skill_record(&self, id: &str) -> Result<Option<SkillRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
        "SELECT id, agent, scope, path, COALESCE(display_path, path), definition_id, name, state, enabled
         FROM skill_instance WHERE id = ?1",
    )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(SkillRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                scope: row.get(2)?,
                path: PathBuf::from(row.get::<_, String>(3)?),
                display_path: PathBuf::from(row.get::<_, String>(4)?),
                definition_id: row.get(5)?,
                name: row.get(6)?,
                state: row.get(7)?,
                enabled: row.get::<_, i64>(8)? != 0,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_skill_detail(&self, id: &str) -> Result<Option<SkillDetailRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, path, COALESCE(display_path, path), definition_id,
                name, description, state, enabled, frontmatter_raw, body, permissions, fingerprint
         FROM skill_instance WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], skill_detail_record_from_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_skill_details_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<SkillDetailRecord>, CatalogError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT id, agent, scope, path, COALESCE(display_path, path), definition_id,
                name, description, state, enabled, frontmatter_raw, body, permissions, fingerprint
             FROM skill_instance WHERE id IN ({placeholders})"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(ids.iter()),
            skill_detail_record_from_row,
        )?;
        let mut details = Vec::new();
        for row in rows {
            details.push(row?);
        }
        Ok(details)
    }

    pub fn list_skill_events(
        &self,
        instance_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SkillEventRecord>, CatalogError> {
        if let Some(limit) = limit {
            let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
            let mut stmt = self.conn.prepare(
                "SELECT id, instance_id, kind, payload, occurred_at
             FROM skill_event
             WHERE instance_id = ?1
             ORDER BY occurred_at DESC, id DESC
             LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![instance_id, limit_i64], skill_event_from_row)?;
            let mut events = Vec::new();
            for row in rows {
                events.push(row?);
            }
            Ok(events)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, instance_id, kind, payload, occurred_at
             FROM skill_event
             WHERE instance_id = ?1
             ORDER BY occurred_at DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![instance_id], skill_event_from_row)?;
            let mut events = Vec::new();
            for row in rows {
                events.push(row?);
            }
            Ok(events)
        }
    }

    pub fn list_skill_event_page_snapshot(
        &self,
        instance_id: &str,
        before: Option<(i64, i64)>,
        limit: usize,
        validate_revision: impl FnOnce(&str) -> Result<(), CatalogError>,
    ) -> Result<CatalogPageSnapshot<SkillEventRecord>, CatalogError> {
        let transaction = self.conn.unchecked_transaction()?;
        let metadata = query_skill_event_revision_metadata(&transaction, instance_id)?;
        let source_revision = history_source_revision("skill.listEventsPage", &metadata)?;
        validate_revision(&source_revision)?;
        let records = query_skill_event_page(&transaction, instance_id, before, limit)?;
        transaction.commit()?;
        Ok(CatalogPageSnapshot {
            records,
            source_revision,
            total_count: metadata.len(),
        })
    }

    pub fn list_rule_findings(&self) -> Result<Vec<RuleFindingRecord>, CatalogError> {
        let has_findings =
            self.conn
                .query_row("SELECT EXISTS(SELECT 1 FROM rule_finding)", [], |row| {
                    row.get::<_, i64>(0)
                })?
                != 0;
        if !has_findings {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "WITH finding_targets AS (
            SELECT
                f.*,
                COALESCE(i.agent, di.agent, '') AS target_agent,
                COALESCE(i.scope, di.scope, '') AS target_scope
            FROM rule_finding f
            LEFT JOIN skill_instance i ON i.id = f.instance_id
            LEFT JOIN skill_instance di ON di.id = (
                SELECT si.id
                FROM skill_instance si
                WHERE si.definition_id = f.definition_id
                ORDER BY si.id
                LIMIT 1
            )
         )
         SELECT
            f.id, f.triage_key, f.triage_context, f.instance_id, f.definition_id,
            f.rule_id, f.severity,
            COALESCE((
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = f.target_scope
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = ''
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = ''
                  AND t.scope = ''
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            ), f.severity) AS effective_severity,
            COALESCE((
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = f.target_scope
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = ''
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.severity_override
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = ''
                  AND t.scope = ''
                  AND t.severity_override IS NOT NULL
                LIMIT 1
            )) AS severity_override,
            f.message, f.suggestion, f.created_at,
            COALESCE((
                SELECT t.suppression_reason
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = f.target_scope
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.suppression_reason
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = ''
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.suppression_reason
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = ''
                  AND t.scope = ''
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            )) AS suppression_reason,
            COALESCE((
                SELECT t.suppression_note
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = f.target_scope
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.suppression_note
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = ''
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            ), (
                SELECT t.suppression_note
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = ''
                  AND t.scope = ''
                  AND t.suppression_reason IS NOT NULL
                LIMIT 1
            )) AS suppression_note,
            COALESCE((
                SELECT t.updated_at
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = f.target_scope
                  AND (t.severity_override IS NOT NULL OR t.suppression_reason IS NOT NULL)
                LIMIT 1
            ), (
                SELECT t.updated_at
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = f.target_agent
                  AND t.scope = ''
                  AND (t.severity_override IS NOT NULL OR t.suppression_reason IS NOT NULL)
                LIMIT 1
            ), (
                SELECT t.updated_at
                FROM rule_tuning t
                WHERE t.rule_id = f.rule_id
                  AND t.agent = ''
                  AND t.scope = ''
                  AND (t.severity_override IS NOT NULL OR t.suppression_reason IS NOT NULL)
                LIMIT 1
            )) AS rule_tuning_updated_at,
            COALESCE(t.status, 'open') AS triage_status,
            t.note,
            t.updated_at
         FROM finding_targets f
         LEFT JOIN finding_triage t
            ON t.triage_key = f.triage_key
           AND t.triage_context = f.triage_context
         ORDER BY
            CASE effective_severity
                WHEN 'error' THEN 0
                WHEN 'warn' THEN 1
                WHEN 'warning' THEN 1
                WHEN 'info' THEN 2
                ELSE 3
            END,
            rule_id,
            instance_id",
        )?;
        let rows = stmt.query_map([], |row| {
            let suppression_reason: Option<String> = row.get(12)?;
            Ok(RuleFindingRecord {
                id: row.get(0)?,
                triage_key: row.get(1)?,
                triage_context: row.get(2)?,
                instance_id: row.get(3)?,
                definition_id: row.get(4)?,
                rule_id: row.get(5)?,
                severity: row.get(6)?,
                effective_severity: row.get(7)?,
                severity_override: row.get(8)?,
                message: row.get(9)?,
                suggestion: row.get(10)?,
                created_at: row.get(11)?,
                suppressed: suppression_reason.is_some(),
                suppression_reason,
                suppression_note: row.get(13)?,
                rule_tuning_updated_at: row.get(14)?,
                triage_status: row.get(15)?,
                triage_note: row.get(16)?,
                triage_updated_at: row.get(17)?,
            })
        })?;
        let mut findings = Vec::new();
        for row in rows {
            findings.push(row?);
        }
        Ok(findings)
    }

    pub fn list_finding_triage(&self) -> Result<Vec<FindingTriageRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT triage_key, triage_context, status, note, updated_at
         FROM finding_triage
         ORDER BY updated_at DESC, triage_key",
        )?;
        let rows = stmt.query_map([], finding_triage_from_row)?;
        let mut triage = Vec::new();
        for row in rows {
            triage.push(row?);
        }
        Ok(triage)
    }

    pub fn list_rule_tuning(&self) -> Result<Vec<RuleTuningRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
        "SELECT rule_id, agent, scope, severity_override, suppression_reason, suppression_note, updated_at
         FROM rule_tuning
         ORDER BY rule_id, agent, scope",
    )?;
        let rows = stmt.query_map([], rule_tuning_from_row)?;
        let mut tuning = Vec::new();
        for row in rows {
            tuning.push(row?);
        }
        Ok(tuning)
    }

    pub(crate) fn get_rule_tuning(
        &self,
        rule_id: &str,
        agent: Option<&str>,
        scope: Option<&str>,
    ) -> Result<RuleTuningRecord, CatalogError> {
        let key = rule_tuning_key(agent, scope);
        self.conn.query_row(
        "SELECT rule_id, agent, scope, severity_override, suppression_reason, suppression_note, updated_at
         FROM rule_tuning
         WHERE rule_id = ?1 AND agent = ?2 AND scope = ?3",
        params![rule_id, key.0, key.1],
        rule_tuning_from_row,
    ).map_err(Into::into)
    }

    pub fn list_conflict_groups(&self) -> Result<Vec<ConflictGroupRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, definition_id, reason, winner_id
         FROM conflict_group
         ORDER BY definition_id, reason",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut groups = Vec::new();
        for row in rows {
            let (id, definition_id, reason, winner_id) = row?;
            let mut member_stmt = self.conn.prepare(
                "SELECT instance_id
             FROM conflict_group_member
             WHERE group_id = ?1
             ORDER BY instance_id",
            )?;
            let member_rows = member_stmt.query_map(params![&id], |row| row.get(0))?;
            let mut instance_ids = Vec::new();
            for member in member_rows {
                instance_ids.push(member?);
            }
            groups.push(ConflictGroupRecord {
                id,
                definition_id,
                reason,
                winner_id,
                instance_ids,
            });
        }
        Ok(groups)
    }

    /// List snapshots for a given (agent, target) pair, newest first. The
    /// `LIST-SKILLS` UI can render this for the rollback view.
    pub fn list_config_snapshots(
        &self,
        agent: &str,
        target: &str,
    ) -> Result<Vec<ConfigSnapshotRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, target, content, reason, created_at
         FROM config_snapshot
         WHERE agent = ?1 AND target = ?2
           AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
         ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![agent, target], |row| {
            Ok(ConfigSnapshotRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                scope: row.get(2)?,
                target: row.get(3)?,
                content: row.get(4)?,
                reason: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row?);
        }
        Ok(snapshots)
    }

    pub fn list_agent_config_snapshots(
        &self,
        agent: &str,
        scope: Option<&str>,
    ) -> Result<Vec<ConfigSnapshotRecord>, CatalogError> {
        if let Some(scope) = scope {
            let mut stmt = self.conn.prepare(
                "SELECT id, agent, scope, target, content, reason, created_at
             FROM config_snapshot
             WHERE agent = ?1 AND scope = ?2
               AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
             ORDER BY created_at DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![agent, scope], |row| {
                Ok(ConfigSnapshotRecord {
                    id: row.get(0)?,
                    agent: row.get(1)?,
                    scope: row.get(2)?,
                    target: row.get(3)?,
                    content: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, agent, scope, target, content, reason, created_at
             FROM config_snapshot
             WHERE agent = ?1
               AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
             ORDER BY created_at DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![agent], |row| {
                Ok(ConfigSnapshotRecord {
                    id: row.get(0)?,
                    agent: row.get(1)?,
                    scope: row.get(2)?,
                    target: row.get(3)?,
                    content: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        }
    }

    pub fn list_agent_config_snapshot_page_snapshot(
        &self,
        agent: &str,
        scope: Option<&str>,
        before: Option<(i64, &str)>,
        limit: usize,
        validate_revision: impl FnOnce(&str) -> Result<(), CatalogError>,
    ) -> Result<CatalogPageSnapshot<ConfigSnapshotRecord>, CatalogError> {
        let transaction = self.conn.unchecked_transaction()?;
        let metadata = query_agent_config_snapshot_revision_metadata(&transaction, agent, scope)?;
        let source_revision = history_source_revision("snapshot.listAgentConfigPage", &metadata)?;
        validate_revision(&source_revision)?;
        let records = query_agent_config_snapshot_page(&transaction, agent, scope, before, limit)?;
        transaction.commit()?;
        Ok(CatalogPageSnapshot {
            records,
            source_revision,
            total_count: metadata.len(),
        })
    }

    pub fn list_all_config_snapshots(&self) -> Result<Vec<ConfigSnapshotRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, target, content, reason, created_at
         FROM config_snapshot
         WHERE reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
         ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ConfigSnapshotRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                scope: row.get(2)?,
                target: row.get(3)?,
                content: row.get(4)?,
                reason: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row?);
        }
        Ok(snapshots)
    }

    pub fn get_config_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<ConfigSnapshotRecord>, CatalogError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, scope, target, content, reason, created_at
         FROM config_snapshot
         WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(ConfigSnapshotRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                scope: row.get(2)?,
                target: row.get(3)?,
                content: row.get(4)?,
                reason: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

fn query_skill_event_page(
    connection: &Connection,
    instance_id: &str,
    before: Option<(i64, i64)>,
    limit: usize,
) -> Result<Vec<SkillEventRecord>, CatalogError> {
    let (before_occurred_at, before_id) = before.unzip();
    let limit_i64 = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
    let mut stmt = connection.prepare(
        "SELECT id, instance_id, kind, payload, occurred_at
         FROM skill_event
         WHERE instance_id = ?1
           AND (?2 IS NULL OR occurred_at < ?2 OR (occurred_at = ?2 AND id < ?3))
         ORDER BY occurred_at DESC, id DESC
         LIMIT ?4",
    )?;
    let rows = stmt.query_map(
        params![instance_id, before_occurred_at, before_id, limit_i64],
        skill_event_from_row,
    )?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

fn query_skill_event_revision_metadata(
    connection: &Connection,
    instance_id: &str,
) -> Result<Vec<(i64, i64)>, CatalogError> {
    let mut stmt = connection.prepare(
        "SELECT id, occurred_at
         FROM skill_event
         WHERE instance_id = ?1
         ORDER BY occurred_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![instance_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut metadata = Vec::new();
    for row in rows {
        metadata.push(row?);
    }
    Ok(metadata)
}

fn query_agent_config_snapshot_page(
    connection: &Connection,
    agent: &str,
    scope: Option<&str>,
    before: Option<(i64, &str)>,
    limit: usize,
) -> Result<Vec<ConfigSnapshotRecord>, CatalogError> {
    let (before_created_at, before_id) = before.unzip();
    let limit_i64 = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
    let mut stmt = connection.prepare(
        "SELECT id, agent, scope, target, content, reason, created_at
         FROM config_snapshot
         WHERE agent = ?1
           AND (?2 IS NULL OR scope = ?2)
           AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
           AND (?3 IS NULL OR created_at < ?3 OR (created_at = ?3 AND id < ?4))
         ORDER BY created_at DESC, id DESC
         LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![agent, scope, before_created_at, before_id, limit_i64],
        |row| {
            Ok(ConfigSnapshotRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                scope: row.get(2)?,
                target: row.get(3)?,
                content: row.get(4)?,
                reason: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )?;
    let mut snapshots = Vec::new();
    for row in rows {
        snapshots.push(row?);
    }
    Ok(snapshots)
}

fn query_agent_config_snapshot_revision_metadata(
    connection: &Connection,
    agent: &str,
    scope: Option<&str>,
) -> Result<Vec<(String, i64)>, CatalogError> {
    let mut stmt = connection.prepare(
        "SELECT id, created_at
         FROM config_snapshot
         WHERE agent = ?1
           AND (?2 IS NULL OR scope = ?2)
           AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
         ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![agent, scope], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut metadata = Vec::new();
    for row in rows {
        metadata.push(row?);
    }
    Ok(metadata)
}

fn history_source_revision<T: Serialize>(domain: &str, value: &T) -> Result<String, CatalogError> {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(serde_json::to_vec(value)?);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn skill_detail_record_from_row(row: &Row<'_>) -> rusqlite::Result<SkillDetailRecord> {
    let permissions_raw: String = row.get(12)?;
    Ok(SkillDetailRecord {
        id: row.get(0)?,
        agent: row.get(1)?,
        scope: row.get(2)?,
        path: PathBuf::from(row.get::<_, String>(3)?),
        display_path: PathBuf::from(row.get::<_, String>(4)?),
        definition_id: row.get(5)?,
        name: row.get(6)?,
        description: row.get(7)?,
        state: row.get(8)?,
        enabled: row.get::<_, i64>(9)? != 0,
        frontmatter_raw: row.get(10)?,
        body: row.get(11)?,
        permissions: serde_json::from_str(&permissions_raw)
            .unwrap_or_else(|_| serde_json::json!({})),
        fingerprint: row.get(13)?,
    })
}
