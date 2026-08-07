PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS recovery_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    schema_version INTEGER NOT NULL,
    generation INTEGER NOT NULL,
    topology_epoch INTEGER NOT NULL,
    manifest_payload TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);
PRAGMA user_version=1;
