-- 0001_baseline_schema.sql (Framework migration 0001)

CREATE TABLE IF NOT EXISTS app_kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'owner',
    avatar TEXT,
    pin_hash TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    rev INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    owner_profile_id TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('shared', 'private_summary', 'private'))
);

CREATE INDEX IF NOT EXISTS idx_profiles_owner ON profiles(owner_profile_id);
CREATE INDEX IF NOT EXISTS idx_profiles_updated ON profiles(updated_at);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    rev INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    owner_profile_id TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('shared', 'private_summary', 'private'))
);

CREATE INDEX IF NOT EXISTS idx_notes_owner ON notes(owner_profile_id);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at);

CREATE TABLE IF NOT EXISTS change_log (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op TEXT NOT NULL CHECK (op IN ('insert', 'update', 'delete')),
    rev INTEGER NOT NULL,
    at INTEGER NOT NULL,
    profile_id TEXT,
    device_id TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_change_log_entity ON change_log(entity, entity_id);
CREATE INDEX IF NOT EXISTS idx_change_log_at ON change_log(at);
