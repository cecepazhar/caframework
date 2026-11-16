use crate::ai_context::ContextProvider;
use crate::error::CafError;
use crate::visibility::VisibilityScope;
use async_trait::async_trait;

pub struct NotesContextProvider;

#[async_trait]
impl ContextProvider for NotesContextProvider {
    fn name(&self) -> &'static str {
        "notes"
    }

    async fn get_context(&self, scope: &VisibilityScope) -> Result<String, CafError> {
        let conn = crate::db::open()?;
        let query = format!(
            "SELECT title, content FROM notes WHERE {}",
            scope.sql_clause
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;

        let map_fn = |row: &rusqlite::Row| {
            let title: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok(format!("Title: {}\nContent: {}\n", title, content))
        };

        let mut context = String::new();
        if scope.is_super {
            let rows = stmt
                .query_map([], map_fn)
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for row in rows {
                let entry =
                    row.map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
                context.push_str(&entry);
            }
        } else {
            let rows = stmt
                .query_map([&scope.actor_profile_id], map_fn)
                .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
            for row in rows {
                let entry =
                    row.map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
                context.push_str(&entry);
            }
        }

        if scope.is_super {
            crate::audit::log_event(
                "ai_context_read",
                None,
                "Super-role accessed all notes via AI context provider",
            )
            .map_err(|e| CafError::Db(crate::error::DbError::Generic(e.to_string())))?;
        }

        Ok(context)
    }
}
