CREATE TABLE metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;
INSERT INTO metadata(key, value) VALUES
  ('schema_version', '1'),
  ('revision', '0');
CREATE TABLE service_health (
  service_id TEXT PRIMARY KEY,
  observed_at_unix_ms INTEGER NOT NULL,
  valid_for_ms INTEGER NOT NULL,
  dimensions_json TEXT NOT NULL,
  attributes_json TEXT NOT NULL
) STRICT;
CREATE TABLE diagnostic_events (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id TEXT NOT NULL UNIQUE,
  service_id TEXT NOT NULL,
  occurred_at_unix_ms INTEGER NOT NULL,
  severity TEXT NOT NULL,
  name TEXT NOT NULL,
  outcome TEXT NOT NULL,
  reason_code TEXT,
  attributes_json TEXT NOT NULL
) STRICT;
CREATE INDEX diagnostic_events_recent ON diagnostic_events(occurred_at_unix_ms DESC);
CREATE TABLE metric_samples (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  service_id TEXT NOT NULL,
  observed_at_unix_ms INTEGER NOT NULL,
  name TEXT NOT NULL,
  value REAL NOT NULL,
  unit TEXT NOT NULL,
  attributes_json TEXT NOT NULL
) STRICT;
CREATE INDEX metric_samples_recent ON metric_samples(observed_at_unix_ms DESC);
