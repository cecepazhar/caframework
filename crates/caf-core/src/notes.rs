//! Notes module — sample vertical slice demonstrating sync-ready conventions and RBAC/visibility.

use crate::db;
use crate::error::{CafError, DbError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRecord {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub rev: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub origin_device_id: String,
    pub owner_profile_id: String,
    pub owner_name: Option<String>,
    pub visibility: String, // 'shared' | 'private_summary' | 'private'
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInput {
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub visibility: Option<String>,
}

pub fn list_notes() -> Result<Vec<NoteRecord>, CafError> {
    let session = crate::session::get_current_session()?;
    let is_super = session.has_permission("*") || crate::rbac::is_super_role(&session.role);
    list_notes_internal(&session.profile_id, is_super)
}

pub fn list_notes_internal(
    actor_profile_id: &str,
    is_super: bool,
) -> Result<Vec<NoteRecord>, CafError> {
    let conn = db::open()?;

    let sql = if is_super {
        "SELECT n.id, n.title, n.content, n.tags, n.rev, n.created_at, n.updated_at, n.deleted_at,
                n.origin_device_id, n.owner_profile_id, n.visibility, p.name
         FROM notes n
         LEFT JOIN profiles p ON n.owner_profile_id = p.id
         WHERE n.deleted_at IS NULL
         ORDER BY n.updated_at DESC"
    } else {
        "SELECT n.id, n.title, n.content, n.tags, n.rev, n.created_at, n.updated_at, n.deleted_at,
                n.origin_device_id, n.owner_profile_id, n.visibility, p.name
         FROM notes n
         LEFT JOIN profiles p ON n.owner_profile_id = p.id
         WHERE n.deleted_at IS NULL AND (n.owner_profile_id = ?1 OR n.visibility = 'shared')
         ORDER BY n.updated_at DESC"
    };

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<NoteRecord> {
        let tags_str: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        let owner_profile_id: String = row.get(9)?;
        let visibility: String = row.get(10)?;
        let is_private = (visibility == "private" || visibility == "private_summary")
            && owner_profile_id != actor_profile_id;

        Ok(NoteRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            tags,
            rev: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
            origin_device_id: row.get(8)?,
            owner_profile_id,
            owner_name: row.get(11)?,
            visibility,
            is_private,
        })
    };

    let notes: Vec<NoteRecord> = if is_super {
        let rows = stmt
            .query_map([], map_fn)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
    } else {
        let rows = stmt
            .query_map([actor_profile_id], map_fn)
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
    };

    // P05.5: Audit event on super role reading another profile's private rows
    if is_super {
        let private_reads_count = notes
            .iter()
            .filter(|n| n.owner_profile_id != actor_profile_id && n.visibility != "shared")
            .count();

        if private_reads_count > 0 {
            let _ = crate::audit::log_event(
                "PRIVATE_READ",
                Some(actor_profile_id),
                &format!("entity=notes, count={private_reads_count}"),
            );
        }
    }

    Ok(notes)
}

pub fn get_note(id: &str) -> Result<NoteRecord, CafError> {
    let session = crate::session::get_current_session()?;
    let is_super = session.has_permission("*") || crate::rbac::is_super_role(&session.role);

    let conn = db::open()?;
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.title, n.content, n.tags, n.rev, n.created_at, n.updated_at, n.deleted_at,
                    n.origin_device_id, n.owner_profile_id, n.visibility, p.name
             FROM notes n
             LEFT JOIN profiles p ON n.owner_profile_id = p.id
             WHERE n.id = ?1 AND n.deleted_at IS NULL",
        )
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let row = stmt.query_row([id], |row| {
        let tags_str: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        let owner_profile_id: String = row.get(9)?;
        let visibility: String = row.get(10)?;
        let is_private = (visibility == "private" || visibility == "private_summary")
            && owner_profile_id != session.profile_id;

        Ok(NoteRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            tags,
            rev: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
            origin_device_id: row.get(8)?,
            owner_profile_id,
            owner_name: row.get(11)?,
            visibility,
            is_private,
        })
    });

    match row {
        Ok(note) => {
            // Check visibility permission
            if !is_super
                && note.owner_profile_id != session.profile_id
                && note.visibility != "shared"
            {
                // Invisible rows return not found per P05.4
                return Err(CafError::Db(DbError::RecordNotFound));
            }

            if is_super
                && note.owner_profile_id != session.profile_id
                && note.visibility != "shared"
            {
                let _ = crate::audit::log_event(
                    "PRIVATE_READ",
                    Some(&session.profile_id),
                    &format!("entity=notes, count=1, id={id}"),
                );
            }

            Ok(note)
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(CafError::Db(DbError::RecordNotFound)),
        Err(e) => Err(CafError::Db(DbError::Generic(e.to_string()))),
    }
}

pub fn save_note(input: NoteInput) -> Result<NoteRecord, CafError> {
    let session = crate::session::get_current_session()?;
    let actor_profile_id = session.profile_id.clone();
    let is_super = session.has_permission("*") || crate::rbac::is_super_role(&session.role);

    let mut conn = db::open()?;
    let tx = conn
        .transaction()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let now = Utc::now().to_rfc3339();
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());
    let visibility = input.visibility.unwrap_or_else(|| "shared".into());
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".into());

    let (id, rev, created_at, action) = if let Some(existing_id) = input.id {
        // Update existing note
        let existing: (i64, String, String, String) = tx
            .query_row(
                "SELECT rev, created_at, owner_profile_id, visibility FROM notes WHERE id = ?1 AND deleted_at IS NULL",
                [&existing_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| CafError::Db(DbError::Generic(format!("Note not found: {e}"))))?;

        if !is_super && existing.2 != actor_profile_id {
            return Err(CafError::Auth(crate::error::AuthError::PermissionDenied));
        }

        let new_rev = existing.0 + 1;
        tx.execute(
            "UPDATE notes SET title = ?1, content = ?2, tags = ?3, rev = ?4, updated_at = ?5, visibility = ?6 WHERE id = ?7",
            rusqlite::params![input.title, input.content, tags_json, new_rev, now, visibility, existing_id],
        )
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

        (existing_id, new_rev, existing.1, "update")
    } else {
        // Create new note with UUIDv7
        let new_id = Uuid::now_v7().to_string();
        let rev = 1;
        tx.execute(
            "INSERT INTO notes (id, title, content, tags, rev, created_at, updated_at, origin_device_id, owner_profile_id, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                new_id,
                input.title,
                input.content,
                tags_json,
                rev,
                now,
                now,
                device_id,
                actor_profile_id,
                visibility
            ],
        )
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

        (new_id, rev, now.clone(), "create")
    };

    // Append to change_log in the exact same transaction via central sync helper
    if action == "create" {
        crate::sync::insert(&tx, "notes", &id, &actor_profile_id, &device_id)?;
    } else {
        crate::sync::update(&tx, "notes", &id, rev, &actor_profile_id, &device_id)?;
    }

    tx.commit()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    Ok(NoteRecord {
        id,
        title: input.title,
        content: input.content,
        tags: input.tags,
        rev,
        created_at,
        updated_at: now,
        deleted_at: None,
        origin_device_id: device_id,
        owner_profile_id: actor_profile_id,
        owner_name: Some(session.display_name),
        visibility,
        is_private: false,
    })
}

pub fn delete_note(id: &str) -> Result<(), CafError> {
    let session = crate::session::get_current_session()?;
    let actor_profile_id = session.profile_id.clone();
    let is_super = session.has_permission("*") || crate::rbac::is_super_role(&session.role);

    let mut conn = db::open()?;
    let tx = conn
        .transaction()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let now = Utc::now().to_rfc3339();
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());

    let (rev, owner_profile_id): (i64, String) = tx
        .query_row(
            "SELECT rev, owner_profile_id FROM notes WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| CafError::Db(DbError::Generic(format!("Note not found: {e}"))))?;

    if !is_super && owner_profile_id != actor_profile_id {
        return Err(CafError::Auth(crate::error::AuthError::PermissionDenied));
    }

    let new_rev = rev + 1;
    tx.execute(
        "UPDATE notes SET deleted_at = ?1, rev = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![now, new_rev, now, id],
    )
    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    crate::sync::soft_delete(&tx, "notes", id, new_rev, &actor_profile_id, &device_id)?;

    tx.commit()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    Ok(())
}

pub fn get_notes_summary() -> Result<Vec<crate::visibility::OwnerSummary>, CafError> {
    let conn = db::open()?;
    crate::visibility::query_owner_summaries(&conn, "notes")
}
