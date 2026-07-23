CREATE TABLE IF NOT EXISTS catalog_scan_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    revision TEXT NOT NULL
);

INSERT OR IGNORE INTO catalog_scan_state (singleton, generation, revision)
VALUES (
    1,
    0,
    'sha256:0000000000000000000000000000000000000000000000000000000000000000'
);
