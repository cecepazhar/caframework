-- 0002_ai_routing_and_memory.sql (Framework migration 0002)

CREATE TABLE IF NOT EXISTS ai_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key_encrypted TEXT,
    default_model TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    custom_headers_json TEXT DEFAULT '{}',
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS ai_user_personas (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL,
    custom_rules_json TEXT NOT NULL DEFAULT '[]',
    environment_constraints TEXT,
    is_global_default INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS ai_routing_matrix (
    task_type TEXT PRIMARY KEY,
    primary_provider_id TEXT NOT NULL,
    primary_model TEXT NOT NULL,
    fallback_provider_id TEXT,
    fallback_model TEXT,
    temperature REAL NOT NULL DEFAULT 0.2,
    max_tokens INTEGER NOT NULL DEFAULT 2048,
    system_persona_id TEXT,
    FOREIGN KEY(primary_provider_id) REFERENCES ai_providers(id) ON DELETE RESTRICT,
    FOREIGN KEY(fallback_provider_id) REFERENCES ai_providers(id) ON DELETE SET NULL,
    FOREIGN KEY(system_persona_id) REFERENCES ai_user_personas(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS ai_habits (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    key_tag TEXT NOT NULL,
    fact_content TEXT NOT NULL,
    source_context TEXT,
    confidence_score REAL NOT NULL DEFAULT 1.0,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_accessed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS ai_custom_skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    triggers_json TEXT NOT NULL,
    preferred_model_id TEXT,
    system_instructions TEXT NOT NULL,
    allowed_tools_json TEXT NOT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS ai_habits_fts USING fts5(
    id UNINDEXED,
    category,
    key_tag,
    fact_content,
    tokenize = 'porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS trg_ai_habits_ai AFTER INSERT ON ai_habits BEGIN
    INSERT INTO ai_habits_fts(id, category, key_tag, fact_content)
    VALUES (new.id, new.category, new.key_tag, new.fact_content);
END;

CREATE TRIGGER IF NOT EXISTS trg_ai_habits_ad AFTER DELETE ON ai_habits BEGIN
    INSERT INTO ai_habits_fts(ai_habits_fts, id, category, key_tag, fact_content)
    VALUES('delete', old.id, old.category, old.key_tag, old.fact_content);
END;

CREATE TRIGGER IF NOT EXISTS trg_ai_habits_au AFTER UPDATE ON ai_habits BEGIN
    INSERT INTO ai_habits_fts(ai_habits_fts, id, category, key_tag, fact_content)
    VALUES('delete', old.id, old.category, old.key_tag, old.fact_content);
    INSERT INTO ai_habits_fts(id, category, key_tag, fact_content)
    VALUES (new.id, new.category, new.key_tag, new.fact_content);
END;

INSERT OR IGNORE INTO ai_providers (id, name, provider_type, base_url, default_model, is_active) VALUES
    ('byo-anthropic', 'Anthropic Claude', 'anthropic', 'https://api.anthropic.com/v1', 'claude-3-7-sonnet-20250219', 1),
    ('byo-openai', 'OpenAI', 'openai_compatible', 'https://api.openai.com/v1', 'gpt-4o', 1),
    ('local-ollama', 'Ollama Local', 'ollama', 'http://localhost:11434', 'llama3.2:latest', 1);

INSERT OR IGNORE INTO ai_user_personas (id, title, description, system_prompt, custom_rules_json, environment_constraints, is_global_default)
    VALUES (
        'default-general',
        'CAFramework AI Assistant',
        'Asisten AI cerdas untuk aplikasi ekosistem CA',
        'You are an AI assistant in CAFramework. Be concise, objective, and helpful.',
        '[]',
        'Linux/macOS/Windows, POSIX shell',
        1
    );

INSERT OR IGNORE INTO ai_routing_matrix (task_type, primary_provider_id, primary_model, fallback_provider_id, fallback_model, temperature, max_tokens, system_persona_id) VALUES
    ('chat', 'byo-anthropic', 'claude-3-7-sonnet-20250219', 'byo-openai', 'gpt-4o', 0.7, 4096, 'default-general'),
    ('error_diagnostic', 'byo-anthropic', 'claude-3-7-sonnet-20250219', 'byo-openai', 'gpt-4o-mini', 0.2, 2048, 'default-general'),
    ('prompt_studio', 'byo-anthropic', 'claude-3-7-sonnet-20250219', 'byo-openai', 'gpt-4o', 0.7, 4096, 'default-general'),
    ('command_autocomplete', 'byo-openai', 'gpt-4o-mini', 'local-ollama', 'llama3.2:latest', 0.1, 512, 'default-general'),
    ('security_review', 'byo-anthropic', 'claude-3-7-sonnet-20250219', 'byo-openai', 'gpt-4o', 0.1, 4096, 'default-general');
