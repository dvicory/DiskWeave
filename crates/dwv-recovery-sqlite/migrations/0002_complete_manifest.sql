ALTER TABLE recovery_state ADD COLUMN manifest_json TEXT;
ALTER TABLE recovery_state ADD COLUMN manifest_digest TEXT;

INSERT OR IGNORE INTO schema_migrations(version) VALUES (2);
PRAGMA user_version=2;
