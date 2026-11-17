//! Context Provider Registry (FR-7, P12.3).
//! Products register context providers to inject domain context into AI prompts.
//! Providers read through the active session's permissions and visibility rules.

use crate::error::CafError;
use crate::session::Session;
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

pub trait ContextProvider: Send + Sync {
    /// Unique provider identifier (e.g. "notes", "tasks", "system").
    fn key(&self) -> &'static str;

    /// Generates summary context (high-level aggregate statistics).
    fn provide_summary(&self, session: &Session) -> Result<String, CafError>;

    /// Generates detailed context (scoped by session visibility and permissions).
    fn provide_detailed(&self, session: &Session) -> Result<String, CafError>;

    /// Generates full context (only permitted for local loopback models).
    fn provide_full(&self, session: &Session) -> Result<String, CafError>;
}

/// Built-in Notes Context Provider.
pub struct NotesContextProvider;

impl ContextProvider for NotesContextProvider {
    fn key(&self) -> &'static str {
        "notes"
    }

    fn provide_summary(&self, session: &Session) -> Result<String, CafError> {
        if !session.has_permission("notes:read") && !session.has_permission("*") {
            return Ok(String::new());
        }
        let conn = crate::db::open()?;
        let is_super = session.role == "super_admin";
        let count: i64 = if is_super {
            conn.query_row(
                "SELECT COUNT(*) FROM notes WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0)
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM notes WHERE deleted_at IS NULL AND (owner_profile_id = ?1 OR visibility = 'shared')",
                [&session.profile_id],
                |r| r.get(0),
            )
            .unwrap_or(0)
        };
        Ok(format!("Notes: {} accessible notes", count))
    }

    fn provide_detailed(&self, session: &Session) -> Result<String, CafError> {
        if !session.has_permission("notes:read") && !session.has_permission("*") {
            return Ok(String::new());
        }
        let conn = crate::db::open()?;
        let is_super = session.role == "super_admin";
        let mut notes_text = String::new();

        if is_super {
            let mut stmt = conn
                .prepare(
                    "SELECT title, content FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 10",
                )
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            let rows = stmt
                .query_map([], |r| {
                    let title: String = r.get(0)?;
                    let content: String = r.get(1)?;
                    Ok(format!("Note: {}\n{}", title, content))
                })
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for entry in rows.flatten() {
                notes_text.push_str(&entry);
                notes_text.push_str("\n\n");
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT title, content FROM notes WHERE deleted_at IS NULL AND (owner_profile_id = ?1 OR visibility = 'shared') ORDER BY updated_at DESC LIMIT 10",
                )
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            let rows = stmt
                .query_map([&session.profile_id], |r| {
                    let title: String = r.get(0)?;
                    let content: String = r.get(1)?;
                    Ok(format!("Note: {}\n{}", title, content))
                })
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for entry in rows.flatten() {
                notes_text.push_str(&entry);
                notes_text.push_str("\n\n");
            }
        }

        Ok(notes_text)
    }

    fn provide_full(&self, session: &Session) -> Result<String, CafError> {
        if !session.has_permission("notes:read") && !session.has_permission("*") {
            return Ok(String::new());
        }
        let conn = crate::db::open()?;
        let is_super = session.role == "super_admin";
        let mut notes_text = String::new();

        if is_super {
            let mut stmt = conn
                .prepare(
                    "SELECT title, content FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 20",
                )
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            let rows = stmt
                .query_map([], |r| {
                    let title: String = r.get(0)?;
                    let content: String = r.get(1)?;
                    Ok(format!("Note: {}\n{}", title, content))
                })
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for entry in rows.flatten() {
                notes_text.push_str(&entry);
                notes_text.push_str("\n\n");
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT title, content FROM notes WHERE deleted_at IS NULL AND (owner_profile_id = ?1 OR visibility = 'shared') ORDER BY updated_at DESC LIMIT 20",
                )
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            let rows = stmt
                .query_map([&session.profile_id], |r| {
                    let title: String = r.get(0)?;
                    let content: String = r.get(1)?;
                    Ok(format!("Note: {}\n{}", title, content))
                })
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for entry in rows.flatten() {
                notes_text.push_str(&entry);
                notes_text.push_str("\n\n");
            }
        }

        Ok(notes_text)
    }
}

static PROVIDERS: Lazy<RwLock<Vec<Arc<dyn ContextProvider>>>> =
    Lazy::new(|| RwLock::new(vec![Arc::new(NotesContextProvider)]));

pub fn register_context_provider(provider: Arc<dyn ContextProvider>) {
    let mut list = PROVIDERS.write().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|p| p.key() == provider.key()) {
        list.push(provider);
    }
}

pub fn get_registered_providers() -> Vec<Arc<dyn ContextProvider>> {
    PROVIDERS.read().unwrap_or_else(|e| e.into_inner()).clone()
}
