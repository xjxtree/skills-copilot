CREATE TEMP TABLE legacy_skill_rows_to_remove (
    id TEXT PRIMARY KEY
);

INSERT INTO legacy_skill_rows_to_remove (id)
SELECT id
FROM skill_instance
WHERE (
        agent = 'codex'
        AND instr(replace(path, '\', '/'), '/.agent-copilot-runtime/') > 0
    )
    OR (
        agent = 'pi'
        AND state = 'missing'
        AND (
            instr(lower(replace(path, '\', '/')), '/references/') > 0
            OR (
                lower(path) LIKE '%.md'
                AND lower(replace(path, '\', '/')) NOT LIKE '%/skill.md'
            )
        )
    );

CREATE TEMP TABLE legacy_conflict_groups_to_remove (
    id TEXT PRIMARY KEY
);

INSERT OR IGNORE INTO legacy_conflict_groups_to_remove (id)
SELECT group_id
FROM conflict_group_member
WHERE instance_id IN (SELECT id FROM legacy_skill_rows_to_remove);

INSERT OR IGNORE INTO legacy_conflict_groups_to_remove (id)
SELECT id
FROM conflict_group
WHERE winner_id IN (SELECT id FROM legacy_skill_rows_to_remove);

CREATE TEMP TABLE legacy_finding_triage_to_remove (
    triage_key TEXT PRIMARY KEY
);

INSERT OR IGNORE INTO legacy_finding_triage_to_remove (triage_key)
SELECT triage_key
FROM rule_finding
WHERE instance_id IN (SELECT id FROM legacy_skill_rows_to_remove)
  AND triage_key != '';

DELETE FROM conflict_group_member
WHERE group_id IN (SELECT id FROM legacy_conflict_groups_to_remove)
   OR instance_id IN (SELECT id FROM legacy_skill_rows_to_remove);

DELETE FROM conflict_group
WHERE id IN (SELECT id FROM legacy_conflict_groups_to_remove);

DELETE FROM skill_event
WHERE instance_id IN (SELECT id FROM legacy_skill_rows_to_remove);

DELETE FROM rule_finding
WHERE instance_id IN (SELECT id FROM legacy_skill_rows_to_remove);

DELETE FROM finding_triage
WHERE triage_key IN (SELECT triage_key FROM legacy_finding_triage_to_remove)
  AND NOT EXISTS (
      SELECT 1
      FROM rule_finding
      WHERE rule_finding.triage_key = finding_triage.triage_key
  );

UPDATE skill_definition
SET active_instance = NULL
WHERE active_instance IN (SELECT id FROM legacy_skill_rows_to_remove);

DELETE FROM skill_instance
WHERE id IN (SELECT id FROM legacy_skill_rows_to_remove);

DELETE FROM skill_definition
WHERE NOT EXISTS (
    SELECT 1
    FROM skill_instance
    WHERE skill_instance.definition_id = skill_definition.id
);

DROP TABLE legacy_finding_triage_to_remove;
DROP TABLE legacy_conflict_groups_to_remove;
DROP TABLE legacy_skill_rows_to_remove;
