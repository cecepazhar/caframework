//! Entity registry and field schema definitions.

use crate::error::{CafError, DataIoError, DbError};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EntityFieldType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Tags,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EntityField {
    pub name: String,
    pub field_type: EntityFieldType,
    pub required: bool,
    pub is_natural_key: bool,
    pub description_en: String,
    pub description_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EntityDefinition {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub table_name: String,
    pub formats: Vec<String>,
    pub natural_keys: Vec<String>,
    pub fields: Vec<EntityField>,
}

/// Dynamic adapter enabling generic export and transactional import for an entity.
pub trait EntitySyncAdapter: Send + Sync {
    /// Returns the table name.
    fn table_name(&self) -> &'static str;

    /// Streams all accessible records for export row by row: `on_row(&[(&str, &str)])`.
    /// Returns `(total_rows, private_reads_count)`.
    #[allow(clippy::type_complexity)]
    fn stream_export_records(
        &self,
        conn: &rusqlite::Connection,
        is_super: bool,
        actor_profile_id: &str,
        on_row: &mut dyn FnMut(&[(&str, &str)]) -> Result<(), CafError>,
    ) -> Result<(usize, usize), CafError>;

    /// Fetches all accessible records for export as a map of field name -> string value.
    fn fetch_export_records(
        &self,
        conn: &rusqlite::Connection,
        is_super: bool,
        actor_profile_id: &str,
    ) -> Result<Vec<HashMap<String, String>>, CafError> {
        let mut list = Vec::new();
        self.stream_export_records(conn, is_super, actor_profile_id, &mut |fields| {
            let mut map = HashMap::with_capacity(fields.len());
            for (k, v) in fields {
                map.insert((*k).to_string(), (*v).to_string());
            }
            list.push(map);
            Ok(())
        })?;
        Ok(list)
    }

    /// Resolves an existing record ID by explicit ID or by natural key values.
    fn resolve_existing_id(
        &self,
        tx: &rusqlite::Transaction,
        id: Option<&str>,
        natural_keys: &HashMap<String, String>,
    ) -> Result<Option<String>, CafError>;

    /// Inserts a new record, updates change_log, and returns the newly generated ID.
    fn insert_record(
        &self,
        tx: &rusqlite::Transaction,
        fields: &HashMap<String, String>,
        actor_profile_id: &str,
        device_id: &str,
    ) -> Result<String, CafError>;

    /// Updates an existing record and appends an update entry to change_log.
    fn update_record(
        &self,
        tx: &rusqlite::Transaction,
        id: &str,
        fields: &HashMap<String, String>,
        actor_profile_id: &str,
        device_id: &str,
    ) -> Result<(), CafError>;
}

pub type EntityRegistryMap = HashMap<String, (EntityDefinition, Arc<dyn EntitySyncAdapter>)>;

/// Global registry of registered entities.
static ENTITY_REGISTRY: Lazy<RwLock<EntityRegistryMap>> = Lazy::new(|| {
    let registry = RwLock::new(HashMap::new());
    register_builtin_notes(&registry);
    registry
});

fn register_builtin_notes(registry: &RwLock<EntityRegistryMap>) {
    let def = EntityDefinition {
        key: "notes".into(),
        label_en: "Notes".into(),
        label_id: "Catatan".into(),
        table_name: "notes".into(),
        formats: vec!["csv".into(), "xlsx".into(), "json".into()],
        natural_keys: vec!["title".into()],
        fields: vec![
            EntityField {
                name: "id".into(),
                field_type: EntityFieldType::String,
                required: false,
                is_natural_key: false,
                description_en: "Unique identifier (UUID)".into(),
                description_id: "Pengenal unik (UUID)".into(),
            },
            EntityField {
                name: "title".into(),
                field_type: EntityFieldType::String,
                required: true,
                is_natural_key: true,
                description_en: "Note title".into(),
                description_id: "Judul catatan".into(),
            },
            EntityField {
                name: "content".into(),
                field_type: EntityFieldType::String,
                required: false,
                is_natural_key: false,
                description_en: "Note content".into(),
                description_id: "Isi catatan".into(),
            },
            EntityField {
                name: "tags".into(),
                field_type: EntityFieldType::Tags,
                required: false,
                is_natural_key: false,
                description_en: "Comma-separated tags".into(),
                description_id: "Label / tag terpisah koma".into(),
            },
            EntityField {
                name: "visibility".into(),
                field_type: EntityFieldType::String,
                required: false,
                is_natural_key: false,
                description_en: "Visibility (shared, private_summary, private)".into(),
                description_id: "Visibilitas (shared, private_summary, private)".into(),
            },
        ],
    };

    struct NotesAdapter;

    impl EntitySyncAdapter for NotesAdapter {
        fn table_name(&self) -> &'static str {
            "notes"
        }

        #[allow(clippy::type_complexity)]
        fn stream_export_records(
            &self,
            conn: &rusqlite::Connection,
            is_super: bool,
            actor_profile_id: &str,
            on_row: &mut dyn FnMut(&[(&str, &str)]) -> Result<(), CafError>,
        ) -> Result<(usize, usize), CafError> {
            let sql = if is_super {
                "SELECT id, title, content, tags, visibility, owner_profile_id, created_at, updated_at FROM notes WHERE deleted_at IS NULL ORDER BY rowid ASC"
            } else {
                "SELECT id, title, content, tags, visibility, owner_profile_id, created_at, updated_at FROM notes WHERE deleted_at IS NULL AND (owner_profile_id = ?1 OR visibility = 'shared') ORDER BY rowid ASC"
            };

            let mut stmt = conn
                .prepare(sql)
                .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

            let mut rows = if is_super {
                stmt.query([])
            } else {
                stmt.query(rusqlite::params![actor_profile_id])
            }
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

            let mut total_count = 0usize;
            let mut private_reads_count = 0usize;

            while let Some(r) = rows
                .next()
                .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
            {
                total_count += 1;
                let id = r
                    .get_ref(0)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                    .as_str()
                    .unwrap_or("");
                let title = r
                    .get_ref(1)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                    .as_str()
                    .unwrap_or("");
                let content = match r
                    .get_ref(2)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => std::str::from_utf8(s).unwrap_or(""),
                    _ => "",
                };
                let tags_raw = match r
                    .get_ref(3)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => std::str::from_utf8(s).unwrap_or(""),
                    _ => "",
                };
                let tags_buf;
                let tags = if tags_raw == "[]" || tags_raw.is_empty() {
                    ""
                } else if tags_raw.starts_with('[') {
                    tags_buf = serde_json::from_str::<Vec<String>>(tags_raw)
                        .unwrap_or_default()
                        .join(", ");
                    tags_buf.as_str()
                } else {
                    tags_raw
                };
                let visibility = match r
                    .get_ref(4)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => {
                        std::str::from_utf8(s).unwrap_or("shared")
                    }
                    _ => "shared",
                };
                let owner = match r
                    .get_ref(5)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => std::str::from_utf8(s).unwrap_or(""),
                    _ => "",
                };
                let created_at = match r
                    .get_ref(6)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => std::str::from_utf8(s).unwrap_or(""),
                    _ => "",
                };
                let updated_at = match r
                    .get_ref(7)
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    rusqlite::types::ValueRef::Text(s) => std::str::from_utf8(s).unwrap_or(""),
                    _ => "",
                };

                if is_super && owner != actor_profile_id && visibility != "shared" {
                    private_reads_count += 1;
                }

                let fields = [
                    ("id", id),
                    ("title", title),
                    ("content", content),
                    ("tags", tags),
                    ("visibility", visibility),
                    ("owner_profile_id", owner),
                    ("created_at", created_at),
                    ("updated_at", updated_at),
                ];

                on_row(&fields)?;
            }

            Ok((total_count, private_reads_count))
        }

        fn resolve_existing_id(
            &self,
            tx: &rusqlite::Transaction,
            id: Option<&str>,
            natural_keys: &HashMap<String, String>,
        ) -> Result<Option<String>, CafError> {
            if let Some(id_val) = id
                && !id_val.is_empty()
            {
                let mut stmt = tx
                    .prepare("SELECT id FROM notes WHERE id = ?1 AND deleted_at IS NULL LIMIT 1")
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
                let mut rows = stmt
                    .query(rusqlite::params![id_val])
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
                if let Some(row) = rows
                    .next()
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    return Ok(Some(
                        row.get(0)
                            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?,
                    ));
                }
            }

            if let Some(title) = natural_keys.get("title")
                && !title.is_empty()
            {
                let mut stmt = tx
                    .prepare("SELECT id FROM notes WHERE title = ?1 AND deleted_at IS NULL LIMIT 1")
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
                let mut rows = stmt
                    .query(rusqlite::params![title])
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
                if let Some(row) = rows
                    .next()
                    .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?
                {
                    return Ok(Some(
                        row.get(0)
                            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?,
                    ));
                }
            }

            Ok(None)
        }

        fn insert_record(
            &self,
            tx: &rusqlite::Transaction,
            fields: &HashMap<String, String>,
            actor_profile_id: &str,
            device_id: &str,
        ) -> Result<String, CafError> {
            let table = self.table_name();
            let id = fields
                .get("id")
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
            let title = fields.get("title").cloned().unwrap_or_default();
            let content = fields.get("content").cloned().unwrap_or_default();
            let tags_raw = fields.get("tags").cloned().unwrap_or_default();
            let tags_vec: Vec<String> = tags_raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let tags_json = serde_json::to_string(&tags_vec).unwrap_or_else(|_| "[]".into());
            let visibility = fields
                .get("visibility")
                .cloned()
                .unwrap_or_else(|| "shared".into());
            let now = chrono::Utc::now().to_rfc3339();

            let sql = format!(
                "INSERT INTO {table} (id, title, content, tags, rev, created_at, updated_at, origin_device_id, owner_profile_id, visibility) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            );
            tx.execute(
                &sql,
                rusqlite::params![
                    id,
                    title,
                    content,
                    tags_json,
                    1,
                    now,
                    now,
                    device_id,
                    actor_profile_id,
                    visibility
                ],
            )
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

            crate::sync::insert(tx, table, &id, actor_profile_id, device_id)?;
            Ok(id)
        }

        fn update_record(
            &self,
            tx: &rusqlite::Transaction,
            id: &str,
            fields: &HashMap<String, String>,
            actor_profile_id: &str,
            device_id: &str,
        ) -> Result<(), CafError> {
            let table = self.table_name();
            let query_rev = format!("SELECT rev FROM {table} WHERE id = ?1 AND deleted_at IS NULL");
            let mut stmt = tx
                .prepare(&query_rev)
                .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
            let rev: i64 = stmt
                .query_row(rusqlite::params![id], |r| r.get(0))
                .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

            let new_rev = rev + 1;
            let now = chrono::Utc::now().to_rfc3339();

            let title = fields.get("title").cloned().unwrap_or_default();
            let content = fields.get("content").cloned().unwrap_or_default();
            let tags_raw = fields.get("tags").cloned().unwrap_or_default();
            let tags_vec: Vec<String> = tags_raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let tags_json = serde_json::to_string(&tags_vec).unwrap_or_else(|_| "[]".into());
            let visibility = fields
                .get("visibility")
                .cloned()
                .unwrap_or_else(|| "shared".into());

            let sql = format!(
                "UPDATE {table} SET title = ?1, content = ?2, tags = ?3, rev = ?4, updated_at = ?5, visibility = ?6 WHERE id = ?7"
            );
            tx.execute(
                &sql,
                rusqlite::params![title, content, tags_json, new_rev, now, visibility, id],
            )
            .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

            crate::sync::update(tx, table, id, new_rev, actor_profile_id, device_id)?;
            Ok(())
        }
    }

    registry
        .write()
        .insert("notes".into(), (def, Arc::new(NotesAdapter)));
}

/// Retrieves an entity definition and its sync adapter by key.
pub fn get_entity_and_adapter(
    key: &str,
) -> Result<(EntityDefinition, Arc<dyn EntitySyncAdapter>), CafError> {
    let guard = ENTITY_REGISTRY.read();
    guard
        .get(key)
        .cloned()
        .ok_or(CafError::DataIo(DataIoError::UnknownEntity))
}

/// Retrieves an entity definition by key.
pub fn get_entity(key: &str) -> Result<EntityDefinition, CafError> {
    let (def, _) = get_entity_and_adapter(key)?;
    Ok(def)
}

/// Lists all registered entities.
pub fn list_entities() -> Vec<EntityDefinition> {
    let guard = ENTITY_REGISTRY.read();
    let mut list: Vec<EntityDefinition> = guard.values().map(|(def, _)| def.clone()).collect();
    list.sort_by(|a, b| a.key.cmp(&b.key));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_notes_registered() {
        let entities = list_entities();
        assert!(!entities.is_empty());
        let notes = entities.iter().find(|e| e.key == "notes");
        assert!(notes.is_some(), "Notes entity must be registered");
        let notes_def = notes.cloned().unwrap_or_else(|| panic!("missing notes"));
        assert_eq!(notes_def.table_name, "notes");
        assert!(notes_def.formats.contains(&"csv".to_string()));
        assert!(notes_def.formats.contains(&"xlsx".to_string()));
        assert!(notes_def.formats.contains(&"json".to_string()));
        assert_eq!(notes_def.natural_keys, vec!["title".to_string()]);
    }
}
