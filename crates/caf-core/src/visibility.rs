//! Visibility rules, data isolation, and super-role audit scopes.

use crate::error::{CafError, DbError};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Shared,
    PrivateSummary,
    Private,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::PrivateSummary => "private_summary",
            Self::Private => "private",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        <Self as std::str::FromStr>::from_str(s).ok()
    }
}

impl std::str::FromStr for Visibility {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shared" => Ok(Self::Shared),
            "private_summary" => Ok(Self::PrivateSummary),
            "private" => Ok(Self::Private),
            _ => Err(()),
        }
    }
}

pub const SCOPE_WHERE_SUPER: &str = "deleted_at IS NULL";
pub const SCOPE_WHERE_USER: &str =
    "deleted_at IS NULL AND (owner_profile_id = ?1 OR visibility = 'shared')";

/// Returns the static parameterised SQL filter fragment.
/// Guarantees 0 string interpolation of user-supplied values.
pub fn sql_scope(is_super: bool) -> &'static str {
    if is_super {
        SCOPE_WHERE_SUPER
    } else {
        SCOPE_WHERE_USER
    }
}

#[derive(Debug, Clone)]
pub struct VisibilityScope {
    pub sql_clause: &'static str,
    pub is_super: bool,
    pub actor_profile_id: String,
}

pub fn scope(actor_profile_id: &str, is_super: bool) -> VisibilityScope {
    if is_super {
        VisibilityScope {
            sql_clause: SCOPE_WHERE_SUPER,
            is_super: true,
            actor_profile_id: actor_profile_id.to_string(),
        }
    } else {
        VisibilityScope {
            sql_clause: SCOPE_WHERE_USER,
            is_super: false,
            actor_profile_id: actor_profile_id.to_string(),
        }
    }
}

pub fn can_modify(owner_profile_id: &str, actor_profile_id: &str, has_manage_perm: bool) -> bool {
    owner_profile_id == actor_profile_id || has_manage_perm
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerSummary {
    pub owner_profile_id: String,
    pub count: usize,
}

/// Helper returning aggregate counts per owner for non-owners (FR-5 private_summary).
pub fn query_owner_summaries(
    conn: &Connection,
    table: &'static str,
) -> Result<Vec<OwnerSummary>, CafError> {
    if table != "notes" && table != "profiles" {
        return Err(CafError::Db(DbError::Generic(
            "invalid table for summary".into(),
        )));
    }

    let sql = if table == "notes" {
        "SELECT owner_profile_id, COUNT(*) FROM notes WHERE deleted_at IS NULL GROUP BY owner_profile_id"
    } else {
        "SELECT owner_profile_id, COUNT(*) FROM profiles WHERE deleted_at IS NULL GROUP BY owner_profile_id"
    };

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let summaries = stmt
        .query_map([], |row| {
            Ok(OwnerSummary {
                owner_profile_id: row.get(0)?,
                count: row.get::<_, i64>(1)? as usize,
            })
        })
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_sql_scope_is_static() {
        assert_eq!(sql_scope(true), SCOPE_WHERE_SUPER);
        assert_eq!(sql_scope(false), SCOPE_WHERE_USER);
    }

    #[test]
    fn test_can_modify() {
        assert!(can_modify("user_1", "user_1", false));
        assert!(!can_modify("user_1", "user_2", false));
        assert!(can_modify("user_1", "user_2", true));
    }
}
