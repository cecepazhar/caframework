//! Test suite for P05: Core-owned sessions, RBAC invariants, parameterised visibility,
//! super-role private reads (D-12), and permission matrix.

use caf_core::api::policy::{COMMAND_POLICIES, Policy, check_policy, guarded};
use caf_core::error::{AuthError, CafError, DbError};
use caf_core::notes::{self, NoteInput};
use caf_core::profiles::{self, ProfileInput};
use caf_core::rbac::{self, RoleInput};
use caf_core::session::{self, Session};
use caf_core::vault;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vault::lock();
        session::clear_current_session().ok();
        let dir = std::env::temp_dir().join(format!("caf_p05_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        vault::setup_vault("test-master-password-123").expect("setup vault");
        let conn = caf_core::db::open().expect("open db");
        rbac::seed_roles(&conn).expect("seed roles");
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        session::clear_current_session().ok();
        let _ = fs::remove_dir_all(&self.dir);
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DATA_DIR");
        }
    }
}

#[test]
fn test_p05_1_core_owned_sessions_lifecycle() {
    let _env = TestEnv::new("p05_1_sessions");

    // 1. Ensure clean slate (no session)
    session::clear_current_session().expect("clear session");
    assert!(session::get_current_session().is_err());
    assert!(session::session_info().is_err());

    // Protected command without session must fail with Auth Unauthorized
    let guarded_res = guarded("list_notes", notes::list_notes);
    assert!(
        matches!(guarded_res, Err(CafError::Auth(AuthError::Unauthorized))),
        "Expected Unauthorized, got {:?}",
        guarded_res
    );

    // 2. Create a test profile with PIN
    let profile = profiles::save_profile(ProfileInput {
        id: None,
        name: "Alice Owner".into(),
        role: "owner".into(),
        avatar: None,
        pin: Some("987654".into()),
    })
    .expect("create profile");

    // 3. Unlock with invalid PIN must fail
    let bad_unlock = session::profile_unlock(&profile.id, "000000");
    assert!(
        matches!(bad_unlock, Err(CafError::Auth(AuthError::InvalidPin))),
        "Expected InvalidPin, got {:?}",
        bad_unlock
    );
    assert!(session::get_current_session().is_err());

    // 4. Unlock with valid PIN establishes core session
    let info = session::profile_unlock(&profile.id, "987654").expect("unlock");
    assert_eq!(info.profile_id, profile.id);
    assert_eq!(info.display_name, "Alice Owner");
    assert_eq!(info.role_key, "owner");
    assert!(info.permissions.contains(&"*".to_string()));

    // Active session info matches
    let curr_info = session::session_info().expect("session info");
    assert_eq!(curr_info.profile_id, profile.id);

    // Protected command now succeeds
    let notes_res = guarded("list_notes", notes::list_notes);
    assert!(notes_res.is_ok());

    // 5. Lock profile removes the session
    session::profile_lock().expect("lock profile");
    assert!(session::get_current_session().is_err());
    assert!(session::session_info().is_err());

    // Protected command fails again
    let locked_call = guarded("list_notes", notes::list_notes);
    assert!(matches!(
        locked_call,
        Err(CafError::Auth(AuthError::Unauthorized))
    ));
}

#[test]
fn test_p05_2_guarded_wrapper_checks() {
    let _env = TestEnv::new("p05_2_guarded");

    // Public command works without session
    let public_res = guarded("window_minimize", || Ok(42));
    assert_eq!(public_res.expect("public"), 42);

    // Session command fails without session
    session::clear_current_session().expect("clear");
    let session_res = guarded("save_note", || Ok(()));
    assert!(matches!(
        session_res,
        Err(CafError::Auth(AuthError::Unauthorized))
    ));

    // Setup session as viewer
    session::set_current_session(Session::new("user_viewer", "viewer")).expect("set session");

    // Viewer trying Requires("security:manage") fails with PermissionDenied
    let sec_res = guarded("change_master_password", || Ok(()));
    assert!(matches!(
        sec_res,
        Err(CafError::Auth(AuthError::PermissionDenied))
    ));

    // Super role can execute Requires commands
    session::set_current_session(Session::new("user_super", "super_admin")).expect("set session");
    let super_sec = guarded("change_master_password", || Ok("allowed"));
    assert_eq!(super_sec.expect("allowed"), "allowed");
}

#[test]
fn test_p05_3_rbac_invariants_and_audit() {
    let _env = TestEnv::new("p05_3_rbac");

    // Bootstrap first super profile
    let super_prof = profiles::save_profile(ProfileInput {
        id: None,
        name: "Root Admin".into(),
        role: "super_admin".into(),
        avatar: None,
        pin: None,
    })
    .expect("bootstrap root");

    session::set_current_session(Session::new(&super_prof.id, "super_admin")).expect("set session");

    // Invariant 1: Super role must hold `*`
    let update_super_no_star = rbac::update_role(
        rbac::SUPER_ROLE_KEY,
        RoleInput {
            key: rbac::SUPER_ROLE_KEY.to_string(),
            label_en: "Super admin".to_string(),
            label_id: "Super admin".to_string(),
            permissions: vec!["notes:view".to_string()], // Missing wildcard!
        },
        &super_prof.id,
    );
    assert!(
        update_super_no_star.is_err(),
        "Updating super role without wildcard must fail"
    );

    // Invariant 2: Super role cannot be renamed or deleted
    let rename_super = rbac::update_role(
        rbac::SUPER_ROLE_KEY,
        RoleInput {
            key: "new_super_key".to_string(),
            label_en: "Super admin".to_string(),
            label_id: "Super admin".to_string(),
            permissions: vec!["*".to_string()],
        },
        &super_prof.id,
    );
    assert!(rename_super.is_err(), "Renaming super role must fail");

    let del_super = rbac::delete_role(rbac::SUPER_ROLE_KEY, &super_prof.id);
    assert!(del_super.is_err(), "Deleting super role must fail");

    // Invariant 3: At least one profile must keep the super role
    let del_last_super_prof = profiles::delete_profile(&super_prof.id);
    assert!(
        del_last_super_prof.is_err(),
        "Deleting the only super role profile must fail"
    );

    // Role CRUD for custom runtime role
    let created_role = rbac::create_role(
        RoleInput {
            key: "auditor".to_string(),
            label_en: "Auditor".to_string(),
            label_id: "Auditor".to_string(),
            permissions: vec!["audit:view".to_string()],
        },
        &super_prof.id,
    )
    .expect("create role");
    assert_eq!(created_role.key, "auditor");

    // Verify audit event was logged
    let logs = caf_core::audit::get_logs(None, Some("create role: auditor"), None).expect("logs");
    assert!(!logs.is_empty(), "Audit event for role creation must exist");

    // Clean delete custom role
    rbac::delete_role("auditor", &super_prof.id).expect("delete role");
}

#[test]
fn test_p05_4_parameterised_visibility_and_sql_injection() {
    let _env = TestEnv::new("p05_4_visibility");

    // Create owner profile and a note
    let owner = profiles::save_profile(ProfileInput {
        id: None,
        name: "Bob".into(),
        role: "member".into(),
        avatar: None,
        pin: None,
    })
    .expect("create bob");

    session::set_current_session(Session::new(&owner.id, "member")).expect("session bob");
    let secret_note = notes::save_note(NoteInput {
        id: None,
        title: "Bob Secret".into(),
        content: "Top secret content".into(),
        tags: vec![],
        visibility: Some("private".into()),
    })
    .expect("save bob note");

    // Attacker attempting SQL injection via actor profile id `' OR 1=1 --`
    let injection_id = "' OR 1=1 --";
    let leaked_notes =
        notes::list_notes_internal(injection_id, false).expect("query with injection");
    assert!(
        leaked_notes.is_empty(),
        "SQL injection must not return any private rows: {:?}",
        leaked_notes
    );

    // Non-owner attempting to read private note directly gets RecordNotFound
    session::set_current_session(Session::new("other_user", "member")).expect("session other");
    let fetch_res = notes::get_note(&secret_note.id);
    assert!(
        matches!(fetch_res, Err(CafError::Db(DbError::RecordNotFound))),
        "Invisible note must return RecordNotFound"
    );

    // Private summary returns aggregates only
    let summaries = notes::get_notes_summary().expect("summaries");
    let bob_summary = summaries.iter().find(|s| s.owner_profile_id == owner.id);
    assert!(bob_summary.is_some());
    assert_eq!(bob_summary.unwrap().count, 1);
}

#[test]
fn test_p05_5_super_role_private_reads_d12() {
    let _env = TestEnv::new("p05_5_d12");

    // Create user and a private note
    let charlie = profiles::save_profile(ProfileInput {
        id: None,
        name: "Charlie".into(),
        role: "member".into(),
        avatar: None,
        pin: None,
    })
    .expect("create charlie");

    session::set_current_session(Session::new(&charlie.id, "member")).expect("session charlie");
    let priv_note = notes::save_note(NoteInput {
        id: None,
        title: "Charlie Vault".into(),
        content: "Super confidential".into(),
        tags: vec![],
        visibility: Some("private".into()),
    })
    .expect("save private note");

    // Viewer/member/admin cannot read it
    session::set_current_session(Session::new("dave_viewer", "viewer")).expect("viewer");
    assert!(notes::list_notes().expect("viewer list").is_empty());
    assert!(notes::get_note(&priv_note.id).is_err());

    session::set_current_session(Session::new("eve_admin", "admin")).expect("admin");
    assert!(notes::list_notes().expect("admin list").is_empty());
    assert!(notes::get_note(&priv_note.id).is_err());

    // Super role CAN read it
    let super_user_id = "super_admin_user";
    session::set_current_session(Session::new(super_user_id, "super_admin")).expect("super");
    let super_list = notes::list_notes().expect("super list");
    assert_eq!(super_list.len(), 1);
    assert_eq!(super_list[0].id, priv_note.id);
    assert_eq!(super_list[0].owner_name.as_deref(), Some("Charlie"));
    assert!(super_list[0].is_private);

    // Verify exactly one PRIVATE_READ audit event per query was written
    let logs = caf_core::audit::get_logs(None, Some("PRIVATE_READ"), None).expect("audit logs");
    assert_eq!(logs.len(), 1);
    assert!(logs[0].details.contains("entity=notes"));
    assert!(logs[0].details.contains("count=1"));
}

#[test]
fn test_p05_6_permission_matrix_all_commands() {
    let _env = TestEnv::new("p05_6_matrix");

    // Test matrix across roles
    let roles = [
        ("no_session", None),
        ("viewer", Some("viewer")),
        ("member", Some("member")),
        ("admin", Some("admin")),
        ("super_admin", Some("super_admin")),
    ];

    for cp in COMMAND_POLICIES {
        for (role_name, role_opt) in &roles {
            if let Some(r) = role_opt {
                session::set_current_session(Session::new(format!("user_{}", role_name), *r))
                    .expect("session");
            } else {
                session::clear_current_session().expect("clear");
            }

            let check_res = check_policy(cp.name);

            match cp.policy {
                Policy::Public => {
                    assert!(
                        check_res.is_ok(),
                        "Command '{}' is Public, must pass for role '{}'",
                        cp.name,
                        role_name
                    );
                }
                Policy::Unlocked => {
                    assert!(
                        check_res.is_ok(),
                        "Command '{}' is Unlocked, must pass for role '{}' when vault is open",
                        cp.name,
                        role_name
                    );
                }
                Policy::Session => {
                    if role_opt.is_some() {
                        assert!(
                            check_res.is_ok(),
                            "Command '{}' is Session, must pass when session exists (role '{}')",
                            cp.name,
                            role_name
                        );
                    } else {
                        assert!(
                            matches!(check_res, Err(CafError::Auth(AuthError::Unauthorized))),
                            "Command '{}' is Session, must fail without session",
                            cp.name
                        );
                    }
                }
                Policy::Requires(perm) => {
                    if role_opt.is_none() {
                        assert!(
                            matches!(check_res, Err(CafError::Auth(AuthError::Unauthorized))),
                            "Command '{}' requires permission '{}', must fail without session",
                            cp.name,
                            perm
                        );
                    } else if *role_name == "super_admin" {
                        assert!(
                            check_res.is_ok(),
                            "Super role must pass command '{}' requiring '{}'",
                            cp.name,
                            perm
                        );
                    } else {
                        // Check if specific role holds this permission
                        let perms =
                            rbac::get_role_permissions(role_opt.unwrap()).unwrap_or_default();
                        let has_perm = rbac::has_permission(&perms, perm);
                        if has_perm {
                            assert!(
                                check_res.is_ok(),
                                "Role '{}' has permission '{}' for command '{}', should pass",
                                role_name,
                                perm,
                                cp.name
                            );
                        } else {
                            assert!(
                                matches!(
                                    check_res,
                                    Err(CafError::Auth(AuthError::PermissionDenied))
                                ),
                                "Role '{}' lacks permission '{}' for command '{}', should fail with PermissionDenied",
                                role_name,
                                perm,
                                cp.name
                            );
                        }
                    }
                }
            }
        }
    }
}
