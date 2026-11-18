-- 0004_feature_flags.sql (Framework migration 0004)

CREATE TABLE IF NOT EXISTS feature_flags (
    key TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1,
    locked INTEGER NOT NULL DEFAULT 0,
    locked_reason TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    rev INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    owner_profile_id TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('shared', 'private_summary', 'private'))
);

CREATE INDEX IF NOT EXISTS idx_feature_flags_owner ON feature_flags(owner_profile_id);
CREATE INDEX IF NOT EXISTS idx_feature_flags_updated ON feature_flags(updated_at);
