use caf_core::backup::{self, BackupAuthMode, TableDump, inspect_backup_header};
use rusqlite::Connection;
use std::path::PathBuf;

fn setup_test_db(name: &str) -> (PathBuf, Connection) {
    let temp_dir =
        std::env::temp_dir().join(format!("caf_backup_test_{name}_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let conn = caf_core::db::open_encrypted(&temp_dir, key).expect("open encrypted test db");
    (temp_dir, conn)
}

#[test]
fn test_p07_1_file_format_and_header_inspection() {
    let (_dir, conn) = setup_test_db("format");

    // Add sample data
    conn.execute(
        "INSERT INTO profiles (id, name, avatar, role, origin_device_id, owner_profile_id, visibility, created_at, updated_at) \
         VALUES ('p1', 'Alice', 'avatar.png', 'admin', 'dev1', 'p1', 'shared', 100, 100)",
        [],
    ).unwrap();

    let payload = backup::dump_payload_from_conn(&conn).unwrap();
    let mut salt1 = [0u8; 32];
    salt1.fill(1);
    let mut salt2 = [0u8; 32];
    salt2.fill(2);

    let pass = "test-strong-passphrase-2026";
    let key1 = [10u8; 32];
    let key2 = [20u8; 32];

    let backup1 =
        backup::pack_and_encrypt_for_test(&payload, &salt1, BackupAuthMode::Passphrase, &key1)
            .unwrap();
    let backup2 =
        backup::pack_and_encrypt_for_test(&payload, &salt2, BackupAuthMode::Passphrase, &key2)
            .unwrap();

    // 1. Two backups of the same data have different salts and ciphertexts
    assert_ne!(
        backup1, backup2,
        "Ciphertexts must differ due to random salt & nonce"
    );
    let header1 = inspect_backup_header(&backup1).expect("inspect header 1");
    let header2 = inspect_backup_header(&backup2).expect("inspect header 2");
    assert_ne!(header1.salt, header2.salt, "Salts must differ");
    assert_eq!(header1.app_id, "com.caframework.app");
    assert_eq!(header1.schema_version, payload.schema_version);

    // 2. Tampering any byte fails restore with specific error
    let mut tampered = backup1.clone();
    let last_idx = tampered.len() - 1;
    tampered[last_idx] ^= 0xFF; // Tamper ciphertext auth tag

    let res = backup::test_decrypt_payload(&tampered, pass);
    assert!(res.is_err(), "Tampered backup must fail decryption");
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("authentication tag mismatch") || err_str.contains("corrupted"),
        "Error must specify auth tag or corruption failure: {err_str}"
    );

    // Tamper magic bytes
    let mut bad_magic = backup1.clone();
    bad_magic[0] = b'X';
    let res_magic = inspect_backup_header(&bad_magic);
    assert!(res_magic.is_err(), "Bad magic bytes must fail");
    assert!(res_magic.unwrap_err().to_string().contains("magic"));
}

#[test]
fn test_p07_2_complete_payload_and_dynamic_table_discovery() {
    let (_dir, conn) = setup_test_db("discovery");

    // 1. Insert data in baseline sync tables including tombstones
    conn.execute(
        "INSERT INTO notes (id, title, content, visibility, owner_profile_id, origin_device_id, created_at, updated_at, deleted_at, rev) \
         VALUES ('n1', 'Secret Note', 'Content', 'private', 'p1', 'dev1', 100, 200, 250, 4)",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO change_log (entity, entity_id, op, rev, at, profile_id, device_id) \
         VALUES ('notes', 'n1', 'delete', 4, 250, 'p1', 'dev1')",
        [],
    )
    .unwrap();

    // 2. Add a dynamic fixture table through a test migration (P07.2 acceptance test)
    conn.execute_batch(
        "CREATE TABLE test_custom_plugin_table ( \
            id TEXT PRIMARY KEY, \
            payload_data TEXT NOT NULL, \
            count INTEGER NOT NULL \
         ); \
         INSERT INTO test_custom_plugin_table (id, payload_data, count) VALUES ('c1', 'hello-plugin', 42);"
    ).unwrap();

    // 3. Dump payload
    let payload = backup::dump_payload_from_conn(&conn).unwrap();

    // 4. Prove the fixture table is in the backup without any code change
    let found_fixture = payload
        .tables
        .iter()
        .find(|t| t.name == "test_custom_plugin_table");
    assert!(
        found_fixture.is_some(),
        "Dynamic table must be discovered in backup payload"
    );
    let t = found_fixture.unwrap();
    assert_eq!(t.rows.len(), 1);
    assert_eq!(t.rows[0][0], "c1");
    assert_eq!(t.rows[0][1], "hello-plugin");
    assert_eq!(t.rows[0][2], 42);

    // Also prove tombstones and change_log are present
    let notes_table = payload.tables.iter().find(|t| t.name == "notes").unwrap();
    assert_eq!(notes_table.rows.len(), 1);
    let change_table = payload
        .tables
        .iter()
        .find(|t| t.name == "change_log")
        .unwrap();
    assert_eq!(change_table.rows.len(), 1);
}

#[test]
fn test_p07_3_atomic_restore_roundtrip_and_corruption_rollback() {
    let (_dir, mut conn) = setup_test_db("atomic");

    // Populate data
    conn.execute(
        "INSERT INTO profiles (id, name, avatar, role, origin_device_id, owner_profile_id, visibility, created_at, updated_at) \
         VALUES ('p1', 'Alice', 'alice.png', 'admin', 'dev1', 'p1', 'shared', 100, 100)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO notes (id, title, content, visibility, owner_profile_id, origin_device_id, created_at, updated_at, deleted_at, rev) \
         VALUES ('n1', 'Note 1', 'Content 1', 'shared', 'p1', 'dev1', 100, 100, 150, 2)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES ('theme', 'dark')",
        [],
    )
    .unwrap();

    let original_payload = backup::dump_payload_from_conn(&conn).unwrap();

    // Wipe database
    conn.execute_batch("DELETE FROM notes; DELETE FROM profiles; DELETE FROM app_kv;")
        .unwrap();
    let empty_notes_count: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(empty_notes_count, 0);

    // Restore from payload
    backup::restore_database_tables(
        &mut conn,
        &original_payload.tables,
        original_payload.schema_version,
    )
    .unwrap();

    // Verify row-by-row identical restoration
    let restored_payload = backup::dump_payload_from_conn(&conn).unwrap();
    assert_eq!(original_payload.tables.len(), restored_payload.tables.len());
    for orig in &original_payload.tables {
        let restored = restored_payload
            .tables
            .iter()
            .find(|t| t.name == orig.name)
            .unwrap();
        assert_eq!(
            orig.rows, restored.rows,
            "Table {} must match row-by-row including rev and tombstones",
            orig.name
        );
    }

    // Negative test: Corrupted data mid-restore rolls back completely
    let mut corrupt_tables = original_payload.tables.clone();
    // Inject a bad table with mismatching columns and values that violates constraint
    corrupt_tables.push(TableDump {
        name: "profiles".to_string(),
        columns: vec!["non_existent_col".to_string()],
        rows: vec![vec![serde_json::json!("junk")]],
    });

    let res = backup::restore_database_tables(
        &mut conn,
        &corrupt_tables,
        original_payload.schema_version,
    );
    assert!(res.is_err(), "Corrupted table restore must fail");

    // Verify original data is intact (rollback succeeded)
    let post_fail_notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        post_fail_notes, 1,
        "Database must remain intact after failed restore"
    );
}

#[test]
fn test_p07_4_backup_key_wrapped_by_keyring_and_password_change() {
    let temp_dir = std::env::temp_dir().join(format!("caf_keyring_bk_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let initial_pass = "initial-secure-password-123";
    let setup = caf_core::keyring::initialize_vault_keyring(initial_pass).unwrap();
    caf_core::keyring::save(&temp_dir, &setup.keyring).unwrap();

    let mut keyring = caf_core::keyring::load(&temp_dir).unwrap();
    let unwrapped_initial =
        caf_core::keyring::unwrap_with_password(&keyring, initial_pass).unwrap();
    let bk_initial = *unwrapped_initial.bk;

    // Encrypt backup using bk
    let mut salt = [0u8; 32];
    salt.fill(7);
    let payload = backup::BackupPayload {
        app_id: "com.caframework.app".to_string(),
        schema_version: 3,
        created_at: 1000,
        tables: vec![],
    };

    let key = backup::derive_key_from_bk_for_test(&bk_initial, &salt);
    let _backup_data =
        backup::pack_and_encrypt_for_test(&payload, &salt, BackupAuthMode::Keyring, &key).unwrap();

    // 1. Restore with master password
    let unwrapped_pw = caf_core::keyring::unwrap_with_password(&keyring, initial_pass).unwrap();
    assert_eq!(
        *unwrapped_pw.bk, bk_initial,
        "BK unwrapped via password must match"
    );

    // 2. Restore with recovery phrase
    let entropy = caf_core::keyring::mnemonic_to_entropy(&setup.mnemonic).unwrap();
    let unwrapped_rec = caf_core::keyring::unwrap_with_recovery(&keyring, &entropy).unwrap();
    assert_eq!(
        *unwrapped_rec.bk, bk_initial,
        "BK unwrapped via recovery must match"
    );

    // 3. Change master password
    let new_pass = "new-super-secure-password-456";
    caf_core::keyring::change_master_password(&mut keyring, initial_pass, new_pass).unwrap();
    caf_core::keyring::save(&temp_dir, &keyring).unwrap();

    // Prove bk remains identical after password change
    let unwrapped_after_change =
        caf_core::keyring::unwrap_with_password(&keyring, new_pass).unwrap();
    assert_eq!(
        *unwrapped_after_change.bk, bk_initial,
        "BK must be preserved across password changes"
    );

    // And recovery code still unwraps the same bk
    let unwrapped_rec_after = caf_core::keyring::unwrap_with_recovery(&keyring, &entropy).unwrap();
    assert_eq!(
        *unwrapped_rec_after.bk, bk_initial,
        "Recovery phrase must still unwrap BK after password change"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
