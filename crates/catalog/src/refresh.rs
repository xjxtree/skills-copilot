use super::*;

impl Catalog {
    fn run_refresh_transaction<F>(&self, operation: F) -> Result<(), CatalogError>
    where
        F: FnOnce() -> Result<(), CatalogError>,
    {
        if !self.conn.is_autocommit() {
            return operation();
        }
        self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        match operation() {
            Ok(()) => match self.conn.execute_batch("COMMIT") {
                Ok(()) => Ok(()),
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

    pub fn refresh_rule_findings(&self, findings: &[RuleFindingDraft]) -> Result<(), CatalogError> {
        self.run_refresh_transaction(|| {
            self.conn.execute("DELETE FROM rule_finding", [])?;
            for finding in findings {
                let triage_context = self.finding_triage_context(finding)?;
                let triage_key = finding_triage_key(finding, &triage_context);
                self.conn.execute(
                    "INSERT INTO rule_finding (
                    id, triage_key, triage_context, instance_id, definition_id,
                    rule_id, severity, message, suggestion, created_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        finding.id,
                        triage_key,
                        triage_context,
                        finding.instance_id.as_deref(),
                        finding.definition_id.as_deref(),
                        finding.rule_id,
                        finding.severity,
                        finding.message,
                        finding.suggestion.as_deref(),
                        finding.created_at,
                    ],
                )?;
            }
            Ok(())
        })
    }

    pub(crate) fn current_finding_triage_context(
        &self,
        triage_key: &str,
    ) -> Result<Option<String>, CatalogError> {
        Ok(self
            .conn
            .query_row(
                "SELECT triage_context FROM rule_finding WHERE triage_key = ?1 LIMIT 1",
                params![triage_key],
                |row| row.get(0),
            )
            .ok())
    }

    fn finding_triage_context(&self, finding: &RuleFindingDraft) -> Result<String, CatalogError> {
        let mut members = Vec::new();
        if let Some(definition_id) = finding.definition_id.as_deref() {
            let mut stmt = self.conn.prepare(
                "SELECT id, fingerprint FROM skill_instance
             WHERE definition_id = ?1
             ORDER BY id",
            )?;
            let rows = stmt.query_map(params![definition_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                members.push(row?);
            }
        } else if let Some(instance_id) = finding.instance_id.as_deref() {
            let mut stmt = self.conn.prepare(
                "SELECT id, fingerprint FROM skill_instance
             WHERE id = ?1
             ORDER BY id",
            )?;
            let rows = stmt.query_map(params![instance_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                members.push(row?);
            }
        }

        let raw = members
            .into_iter()
            .map(|(id, fingerprint)| format!("{id}\x1f{fingerprint}"))
            .collect::<Vec<_>>()
            .join("\x1e");
        Ok(stable_hash(&raw))
    }

    pub fn refresh_definitions_and_conflicts(
        &self,
        definitions: &[SkillDefinitionDraft],
        conflicts: &[ConflictGroupDraft],
    ) -> Result<(), CatalogError> {
        self.run_refresh_transaction(|| {
            self.conn.execute("DELETE FROM conflict_group_member", [])?;
            self.conn.execute("DELETE FROM conflict_group", [])?;
            self.conn.execute("DELETE FROM skill_definition", [])?;

            for definition in definitions {
                self.conn.execute(
                    "INSERT INTO skill_definition (
                    id, canonical_name, description, active_instance,
                    has_multiple_instances, has_conflict
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        definition.id,
                        definition.canonical_name,
                        definition.description,
                        definition.active_instance.as_deref(),
                        i64::from(definition.has_multiple_instances),
                        i64::from(definition.has_conflict),
                    ],
                )?;
            }

            for conflict in conflicts {
                self.conn.execute(
                    "INSERT INTO conflict_group (id, definition_id, reason, winner_id)
                 VALUES (?1, ?2, ?3, ?4)",
                    params![
                        conflict.id,
                        conflict.definition_id,
                        conflict.reason,
                        conflict.winner_id.as_deref(),
                    ],
                )?;
                for instance_id in &conflict.instance_ids {
                    self.conn.execute(
                        "INSERT INTO conflict_group_member (group_id, instance_id)
                     VALUES (?1, ?2)",
                        params![conflict.id, instance_id],
                    )?;
                }
            }
            Ok(())
        })
    }
}
