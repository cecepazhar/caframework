# ADR 0002: Encrypted Backup & Restore Format v2

## Status
Accepted

## Context
CAFramework requires an authenticated, portable, encrypted backup format supporting both:
1. Manual on-demand export protected by user passphrase.
2. Automated scheduled backups protected by the keyring's unwrapped backup key (`bk`).

Previous implementations lacked unencrypted metadata inspection, had hardcoded static salts, omitted dynamic schema discovery (ignoring new tables and tombstones), and restored tables in non-atomic multiple statements without rollback guarantees.

## Decision

### 1. Binary Container Layout
```
+--------------------------------------------------------------------------------+
| Magic Bytes: b"CAFBAK\0\0" (8 bytes)                                          |
+--------------------------------------------------------------------------------+
| Format Version: u16 big-endian (2 bytes, value = 2)                            |
+--------------------------------------------------------------------------------+
| Header Length: u32 big-endian (4 bytes, length of JSON string)                 |
+--------------------------------------------------------------------------------+
| Header JSON: UTF-8 string readable without secrets                            |
+--------------------------------------------------------------------------------+
| Nonce: 12 bytes random CSPRNG nonce                                           |
+--------------------------------------------------------------------------------+
| Ciphertext + Tag: AES-256-GCM encrypted payload with 16-byte authentication tag|
+--------------------------------------------------------------------------------+
```

### 2. Header Specification (Plaintext Readable)
The header contains metadata inspectable without knowledge of secrets or passwords:
```json
{
  "app_id": "com.caframework.app",
  "created_at": 1700000000,
  "schema_version": 3,
  "salt": "32-byte-hex-encoded-random-csprng-salt",
  "auth_mode": "passphrase" | "keyring"
}
```

### 3. Cryptographic Primitives
- **Authenticated Cipher**: AES-256-GCM.
- **Nonce Generation**: 12 bytes CSPRNG (`aes_gcm::aead::OsRng`).
- **Passphrase KDF**: Argon2id (`m_cost = 64 MB`, `t_cost = 3`, `p_cost = 4`, `output = 32 bytes`) with a random 32-byte salt generated per file.
- **Keyring Sub-key**: SHA-256 domain-separated derivation from `bk` (Backup Key from `Keyring::UnwrappedKeys`) and the per-file random 32-byte salt:
  `sub_key = SHA256(b"caframework-backup-keyring-v2" || salt || bk)`
- **Tamper Protection**: Any single bit flip in the header, salt, nonce, ciphertext, or auth tag fails decryption immediately before any database interaction.

### 4. Payload Discovery
All tables in `sqlite_master` (excluding SQLite internals and virtual/FTS shadow tables) are automatically discovered. Backups capture:
- All sync tables with tombstones (`deleted_at IS NOT NULL`), `rev`, `owner_profile_id`, and `origin_device_id`.
- `change_log` audit entries.
- `profiles` with PIN hashes and credentials.
- `app_kv` settings and feature flags.
- Newly added plugin or migration tables without requiring code changes.

### 5. Atomic Restoration
Restorations are executed inside a single SQLite transaction (`tx.transaction()`):
1. `PRAGMA foreign_keys = OFF` is set on the connection before starting the transaction.
2. `PRAGMA defer_foreign_keys = ON` is activated inside the transaction.
3. Target tables are wiped and repopulated with parameterized queries (`INSERT INTO ... VALUES (?, ...)`).
4. Full-text search indexes (`ai_habits_fts`) are rebuilt.
5. If any error occurs during restoration, the transaction is automatically rolled back, leaving pre-restore database state completely untouched.

## Consequences
- Guaranteed forward compatibility for future schema migrations.
- Strict isolation and data integrity across multi-profile vaults.
- Automated scheduled backups operate safely without manual user input.
