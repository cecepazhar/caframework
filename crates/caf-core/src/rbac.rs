//! RBAC permission matrix definitions, runtime role CRUD, and evaluation.

use crate::error::{CafError, DbError, ValidationError};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SUPER_ROLE_KEY: &str = "super_admin";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,
    pub action: String,
}

impl Permission {
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    pub fn matches(&self, required: &Permission) -> bool {
        if self.resource == "*" && self.action == "*" {
            return true;
        }
        if self.resource == required.resource
            && (self.action == "*" || self.action == required.action)
        {
            return true;
        }
        false
    }
}

pub fn matches_permission_str(granted: &str, required: &str) -> bool {
    if granted == "*" || granted == "*:*" {
        return true;
    }
    if granted == required {
        return true;
    }
    if let (Some((g_res, g_act)), Some((r_res, r_act))) =
        (granted.split_once(':'), required.split_once(':'))
    {
        return g_res == r_res && (g_act == "*" || g_act == r_act);
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRecord {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub is_builtin: bool,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleInput {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub permissions: Vec<String>,
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Ensures the default roles are seeded in the database.
pub fn seed_roles(conn: &Connection) -> Result<(), CafError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM roles WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        let now = now_epoch();
        let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());

        let defaults = [
            (SUPER_ROLE_KEY, "Super admin", "Super admin", 1, r#"["*"]"#),
            ("owner", "Owner", "Pemilik", 1, r#"["*"]"#),
            (
                "admin",
                "Admin",
                "Admin",
                1,
                r#"["notes:*","users:view","backup:export","data:export","scheduler:view"]"#,
            ),
            (
                "member",
                "Member",
                "Anggota",
                1,
                r#"["notes:view","notes:create","notes:update","notes:delete"]"#,
            ),
            ("viewer", "Viewer", "Pengamat", 1, r#"["notes:view"]"#),
        ];

        for (key, en, id, builtin, perms) in defaults {
            conn.execute(
                "INSERT OR REPLACE INTO roles (
                    key, label_en, label_id, is_builtin, permissions,
                    created_at, updated_at, deleted_at, rev, origin_device_id, owner_profile_id, visibility
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL, 1, ?7, 'system', 'shared')",
                params![key, en, id, builtin, perms, now, device_id],
            )
            .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to seed role {key}: {e}"))))?;
        }
    }

    Ok(())
}

pub fn list_roles() -> Result<Vec<RoleRecord>, CafError> {
    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT key, label_en, label_id, is_builtin, permissions
             FROM roles WHERE deleted_at IS NULL ORDER BY is_builtin DESC, key ASC",
        )
        .map_err(|e| {
            CafError::Db(DbError::Generic(format!(
                "Failed to prepare list_roles: {e}"
            )))
        })?;

    let roles = stmt
        .query_map([], |row| {
            let perms_json: String = row.get(4)?;
            let permissions: Vec<String> = serde_json::from_str(&perms_json).unwrap_or_default();
            Ok(RoleRecord {
                key: row.get(0)?,
                label_en: row.get(1)?,
                label_id: row.get(2)?,
                is_builtin: row.get::<_, i64>(3)? != 0,
                permissions,
            })
        })
        .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to query roles: {e}"))))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to read roles: {e}"))))?;

    Ok(roles)
}

pub fn get_role(key: &str) -> Result<Option<RoleRecord>, CafError> {
    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT key, label_en, label_id, is_builtin, permissions
             FROM roles WHERE key = ?1 AND deleted_at IS NULL",
        )
        .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to prepare get_role: {e}"))))?;

    let mut rows = stmt
        .query(params![key])
        .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to query get_role: {e}"))))?;

    if let Some(row) = rows
        .next()
        .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to read role row: {e}"))))?
    {
        let key_str: String = row
            .get(0)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        let label_en: String = row
            .get(1)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        let label_id: String = row
            .get(2)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        let builtin_int: i64 = row
            .get(3)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        let perms_json: String = row
            .get(4)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        let permissions: Vec<String> = serde_json::from_str(&perms_json).unwrap_or_default();
        Ok(Some(RoleRecord {
            key: key_str,
            label_en,
            label_id,
            is_builtin: builtin_int != 0,
            permissions,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_role_permissions(key: &str) -> Result<Vec<String>, CafError> {
    if let Some(r) = get_role(key)? {
        Ok(r.permissions)
    } else if key == SUPER_ROLE_KEY || key == "owner" {
        Ok(vec!["*".to_string()])
    } else {
        Ok(vec![])
    }
}

pub fn is_super_role(key: &str) -> bool {
    if key == SUPER_ROLE_KEY || key == "owner" {
        return true;
    }
    if let Ok(Some(r)) = get_role(key) {
        return r.permissions.iter().any(|p| p == "*" || p == "*:*");
    }
    false
}

pub fn has_permission(permissions: &[String], required: &str) -> bool {
    permissions
        .iter()
        .any(|p| matches_permission_str(p, required))
}

pub fn create_role(input: RoleInput, actor_profile_id: &str) -> Result<RoleRecord, CafError> {
    if input.key.trim().is_empty() {
        return Err(CafError::Validation(ValidationError::EmptyField));
    }

    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let existing = get_role(&input.key)?;
    if existing.is_some() {
        return Err(CafError::Validation(ValidationError::Generic(format!(
            "Role '{}' already exists",
            input.key
        ))));
    }

    let now = now_epoch();
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());
    let perms_json = serde_json::to_string(&input.permissions)
        .map_err(|e| CafError::Validation(ValidationError::Generic(e.to_string())))?;

    conn.execute(
        "INSERT INTO roles (
            key, label_en, label_id, is_builtin, permissions,
            created_at, updated_at, deleted_at, rev, origin_device_id, owner_profile_id, visibility
         ) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?5, NULL, 1, ?6, ?7, 'shared')",
        params![
            input.key,
            input.label_en,
            input.label_id,
            perms_json,
            now,
            device_id,
            actor_profile_id
        ],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to insert role: {e}"))))?;

    let _ = crate::audit::log_event(
        "ROLE_CHANGE",
        Some(actor_profile_id),
        &format!("create role: {}", input.key),
    );

    Ok(RoleRecord {
        key: input.key,
        label_en: input.label_en,
        label_id: input.label_id,
        is_builtin: false,
        permissions: input.permissions,
    })
}

pub fn update_role(
    key: &str,
    input: RoleInput,
    actor_profile_id: &str,
) -> Result<RoleRecord, CafError> {
    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let existing = get_role(key)?.ok_or(CafError::Db(DbError::RecordNotFound))?;

    // Invariant 2: Super role cannot be renamed
    if is_super_role(key) && key != input.key {
        return Err(CafError::Validation(ValidationError::Generic(
            "Super role cannot be renamed".to_string(),
        )));
    }

    // Invariant 1: Super role holds `*`
    if is_super_role(key) {
        let holds_star = input.permissions.iter().any(|p| p == "*" || p == "*:*");
        if !holds_star {
            return Err(CafError::Validation(ValidationError::Generic(
                "Super role must hold wildcard '*' permission".to_string(),
            )));
        }
    }

    let now = now_epoch();
    let perms_json = serde_json::to_string(&input.permissions)
        .map_err(|e| CafError::Validation(ValidationError::Generic(e.to_string())))?;

    conn.execute(
        "UPDATE roles SET label_en = ?1, label_id = ?2, permissions = ?3, updated_at = ?4
         WHERE key = ?5 AND deleted_at IS NULL",
        params![input.label_en, input.label_id, perms_json, now, key],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to update role: {e}"))))?;

    let _ = crate::audit::log_event(
        "ROLE_CHANGE",
        Some(actor_profile_id),
        &format!("update role: {}", key),
    );

    Ok(RoleRecord {
        key: key.to_string(),
        label_en: input.label_en,
        label_id: input.label_id,
        is_builtin: existing.is_builtin,
        permissions: input.permissions,
    })
}

pub fn delete_role(key: &str, actor_profile_id: &str) -> Result<(), CafError> {
    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let existing = get_role(key)?.ok_or(CafError::Db(DbError::RecordNotFound))?;

    // Invariant 2: Super role cannot be deleted
    if is_super_role(key) || existing.is_builtin {
        return Err(CafError::Validation(ValidationError::Generic(format!(
            "Role '{key}' is protected and cannot be deleted"
        ))));
    }

    // Invariant 3: Check if active profiles are assigned to this role
    let assigned_profiles: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM profiles WHERE role = ?1 AND deleted_at IS NULL",
            params![key],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if assigned_profiles > 0 {
        return Err(CafError::Validation(ValidationError::Generic(format!(
            "Cannot delete role '{key}': {assigned_profiles} active profiles are currently assigned to it"
        ))));
    }

    let now = now_epoch();
    conn.execute(
        "UPDATE roles SET deleted_at = ?1, updated_at = ?1 WHERE key = ?2",
        params![now, key],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("Failed to delete role: {e}"))))?;

    let _ = crate::audit::log_event(
        "ROLE_CHANGE",
        Some(actor_profile_id),
        &format!("delete role: {}", key),
    );

    Ok(())
}

/// Invariant 3 check for profile management: ensure at least one profile keeps the super role.
pub fn ensure_at_least_one_super_profile(
    target_profile_id: &str,
    new_role: Option<&str>,
    is_delete: bool,
) -> Result<(), CafError> {
    let conn = crate::db::open()?;
    seed_roles(&conn)?;

    let current_role: Option<String> = conn
        .query_row(
            "SELECT role FROM profiles WHERE id = ?1 AND deleted_at IS NULL",
            params![target_profile_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(role) = current_role
        && is_super_role(&role)
    {
        let changing_away = if is_delete {
            true
        } else if let Some(nr) = new_role {
            !is_super_role(nr)
        } else {
            false
        };

        if changing_away {
            let remaining: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM profiles
                     WHERE id != ?1 AND deleted_at IS NULL
                       AND (role = 'super_admin' OR role = 'owner')",
                    params![target_profile_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            if remaining == 0 {
                return Err(CafError::Validation(ValidationError::Generic(
                    "At least one profile must keep the super role".to_string(),
                )));
            }
        }
    }

    Ok(())
}
