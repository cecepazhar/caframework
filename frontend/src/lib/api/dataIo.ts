import {
  invoke_listEntities,
  invoke_getEntityTemplate,
  invoke_exportEntityData,
  invoke_parseImportFile,
  invoke_previewImportData,
  invoke_executeImport
} from './generated_bindings';

export type EntityFieldType =
  | 'string'
  | 'integer'
  | 'float'
  | 'boolean'
  | 'date_time'
  | 'tags';

export interface EntityField {
  name: string;
  field_type: EntityFieldType;
  required: boolean;
  is_natural_key: boolean;
  description_en: string;
  description_id: string;
}

export interface EntityDefinition {
  key: string;
  label_en: string;
  label_id: string;
  table_name: string;
  formats: string[];
  natural_keys: string[];
  fields: EntityField[];
}

export interface ExportResult {
  entity: string;
  format: string;
  row_count: number;
  file_path: string;
  file_size_bytes: number;
}

export type ImportMode = 'insert_only' | 'upsert_by_id' | 'upsert_by_natural_key';
export type ImportStrategy = 'all_or_nothing' | 'skip_invalid';

export interface ImportFileInfo {
  entity: string;
  format: string;
  columns: string[];
  total_rows: number;
  suggested_mapping: Record<string, string>;
}

export interface RowPreview {
  row_number: number;
  values: Record<string, string>;
  action: 'insert' | 'update' | 'invalid';
  error: string | null;
}

export interface ImportPreview {
  total_rows: number;
  to_insert: number;
  to_update: number;
  invalid_count: number;
  preview_rows: RowPreview[];
}

export interface FailedRow {
  row_number: number;
  reason: string;
  raw_data: Record<string, string>;
}

export interface ImportExecutionReport {
  entity: string;
  total_rows: number;
  inserted: number;
  updated: number;
  skipped: number;
  failed: FailedRow[];
  failed_rows_csv: string | null;
}

export async function listEntities(): Promise<EntityDefinition[]> {
  return (await invoke_listEntities()) as EntityDefinition[];
}

export async function getEntityTemplate(entityKey: string, format: string): Promise<Uint8Array> {
  const bytes = await invoke_getEntityTemplate(entityKey, format);
  return new Uint8Array(bytes);
}

export async function exportEntityData(entityKey: string, format: string): Promise<ExportResult> {
  return (await invoke_exportEntityData(entityKey, format)) as ExportResult;
}

export async function parseImportFile(
  entityKey: string,
  format: string,
  data: Uint8Array
): Promise<ImportFileInfo> {
  return (await invoke_parseImportFile(entityKey, format, Array.from(data))) as ImportFileInfo;
}

export async function previewImportData(
  entityKey: string,
  format: string,
  data: Uint8Array,
  mapping: Record<string, string>,
  mode: ImportMode
): Promise<ImportPreview> {
  return (await invoke_previewImportData(
    entityKey,
    format,
    Array.from(data),
    mapping,
    mode
  )) as ImportPreview;
}

export async function executeImport(
  entityKey: string,
  format: string,
  data: Uint8Array,
  mapping: Record<string, string>,
  mode: ImportMode,
  strategy: ImportStrategy
): Promise<ImportExecutionReport> {
  return (await invoke_executeImport(
    entityKey,
    format,
    Array.from(data),
    mapping,
    mode,
    strategy
  )) as ImportExecutionReport;
}
