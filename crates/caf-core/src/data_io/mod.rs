//! Data import and export module (FR-17, P10).

pub mod csv;
pub mod entity;
pub mod export;
pub mod formula;
pub mod import;
pub mod json;
pub mod templates;
pub mod xlsx;

pub use entity::{
    EntityDefinition, EntityField, EntityFieldType, EntitySyncAdapter, get_entity,
    get_entity_and_adapter, list_entities,
};
pub use export::{ExportResult, export_entity};
pub use formula::{neutralize_cell, unneutralize_cell};
pub use import::{
    DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_ROWS, FailedRow, ImportExecutionReport, ImportFileInfo,
    ImportMode, ImportPreview, ImportStrategy, RowPreview, execute_import, parse_import_file,
    preview_import,
};
pub use templates::generate_entity_template;
