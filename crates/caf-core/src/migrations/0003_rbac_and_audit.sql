-- 0003_rbac_and_audit.sql (Framework migration 0003)

CREATE TABLE IF NOT EXISTS command_logs (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    host_id TEXT,
    details TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_command_logs_ts ON command_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_command_logs_event ON command_logs(event_type);

CREATE TABLE IF NOT EXISTS roles (
    key TEXT PRIMARY KEY,
    label_en TEXT NOT NULL,
    label_id TEXT NOT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    permissions TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    rev INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    owner_profile_id TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('shared', 'private_summary', 'private'))
);

CREATE INDEX IF NOT EXISTS idx_roles_owner ON roles(owner_profile_id);
CREATE INDEX IF NOT EXISTS idx_roles_updated ON roles(updated_at);
