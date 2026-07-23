CREATE TABLE IF NOT EXISTS catalog_scan_coverage (
    agent TEXT PRIMARY KEY,
    context_revision TEXT NOT NULL,
    catalog_scan_generation INTEGER NOT NULL CHECK (catalog_scan_generation > 0),
    catalog_scan_revision TEXT NOT NULL,
    coverage_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS catalog_skill_projection (
    instance_id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    context_revision TEXT NOT NULL,
    catalog_scan_generation INTEGER NOT NULL CHECK (catalog_scan_generation > 0),
    catalog_scan_revision TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_identity TEXT NOT NULL,
    runtime_identity TEXT NOT NULL,
    linked INTEGER NOT NULL CHECK (linked IN (0, 1)),
    precedence_proven INTEGER NOT NULL CHECK (precedence_proven IN (0, 1)),
    coverage_json TEXT NOT NULL,
    FOREIGN KEY(instance_id) REFERENCES skill_instance(id) ON DELETE CASCADE
);
