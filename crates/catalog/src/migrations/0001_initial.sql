CREATE TABLE IF NOT EXISTS skill_instance (
    id              TEXT PRIMARY KEY,
    agent           TEXT NOT NULL,
    scope           TEXT NOT NULL,
    project_root    TEXT,
    path            TEXT NOT NULL,
    definition_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL,
    version         TEXT,
    state           TEXT NOT NULL,
    enabled         INTEGER NOT NULL,
    frontmatter     TEXT NOT NULL,
    frontmatter_raw TEXT NOT NULL,
    body            TEXT NOT NULL,
    scripts         TEXT NOT NULL,
    permissions     TEXT NOT NULL,
    fingerprint     TEXT NOT NULL,
    mtime           INTEGER NOT NULL,
    first_seen      INTEGER NOT NULL,
    last_seen       INTEGER NOT NULL,
    UNIQUE (agent, scope, path)
);

CREATE INDEX IF NOT EXISTS idx_instance_definition
    ON skill_instance(definition_id);

CREATE INDEX IF NOT EXISTS idx_instance_agent
    ON skill_instance(agent, scope);

CREATE TABLE IF NOT EXISTS skill_definition (
    id                     TEXT PRIMARY KEY,
    canonical_name         TEXT NOT NULL UNIQUE,
    description            TEXT NOT NULL,
    active_instance        TEXT,
    has_multiple_instances INTEGER NOT NULL,
    has_conflict           INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS conflict_group (
    id            TEXT PRIMARY KEY,
    definition_id TEXT NOT NULL,
    reason        TEXT NOT NULL,
    winner_id     TEXT,
    FOREIGN KEY (definition_id) REFERENCES skill_definition(id)
);

CREATE TABLE IF NOT EXISTS conflict_group_member (
    group_id    TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    PRIMARY KEY (group_id, instance_id)
);

CREATE TABLE IF NOT EXISTS skill_event (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    kind        TEXT NOT NULL,
    payload     TEXT NOT NULL,
    occurred_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_event_instance
    ON skill_event(instance_id, occurred_at);

CREATE TABLE IF NOT EXISTS rule_finding (
    id            TEXT PRIMARY KEY,
    instance_id   TEXT,
    definition_id TEXT,
    rule_id       TEXT NOT NULL,
    severity      TEXT NOT NULL,
    message       TEXT NOT NULL,
    suggestion    TEXT,
    created_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rule_finding_instance
    ON rule_finding(instance_id);

CREATE INDEX IF NOT EXISTS idx_rule_finding_definition
    ON rule_finding(definition_id);

CREATE TABLE IF NOT EXISTS config_snapshot (
    id         TEXT PRIMARY KEY,
    agent      TEXT NOT NULL,
    scope      TEXT NOT NULL,
    project_root TEXT,
    target     TEXT NOT NULL,
    content    TEXT NOT NULL,
    reason     TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
