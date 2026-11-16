//! Runtime data directory and device ID resolution.

use crate::error::{CafError, IoError};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static CUSTOM_DATA_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Explicitly configure the application data directory at runtime (e.g. from Tauri AppHandle on Android/iOS).
pub fn set_custom_data_dir(path: PathBuf) -> Result<(), CafError> {
    if let Ok(mut lock) = CUSTOM_DATA_DIR.write() {
        *lock = Some(path);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirSource {
    EnvOverride,
    PerOsDefault,
    Portable,
}

impl DataDirSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnvOverride => "env_override",
            Self::PerOsDefault => "per_os_default",
            Self::Portable => "portable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataDirInfo {
    pub path: PathBuf,
    pub source: DataDirSource,
}

fn portable_marker_present() -> Result<bool, CafError> {
    let exe = std::env::current_exe()
        .map_err(|e| CafError::Io(IoError::Generic(format!("current_exe failed: {e}"))))?;
    Ok(exe
        .parent()
        .map(|dir| dir.join("portable.txt").is_file())
        .unwrap_or(false))
}

pub fn resolve_data_dir() -> Result<DataDirInfo, CafError> {
    let env_override = std::env::var("CAF_DATA_DIR").ok();
    resolve_data_dir_with(env_override.as_deref())
}

pub fn data_dir() -> Result<PathBuf, CafError> {
    resolve_data_dir().map(|info| info.path)
}

fn resolve_data_dir_with(env_override: Option<&str>) -> Result<DataDirInfo, CafError> {
    if let Ok(lock) = CUSTOM_DATA_DIR.read()
        && let Some(ref dir) = *lock
    {
        return Ok(DataDirInfo {
            path: dir.clone(),
            source: DataDirSource::EnvOverride,
        });
    }

    if let Some(dir) = env_override {
        return Ok(DataDirInfo {
            path: PathBuf::from(dir),
            source: DataDirSource::EnvOverride,
        });
    }

    if portable_marker_present()? {
        return Ok(DataDirInfo {
            path: PathBuf::from("./caframework-data"),
            source: DataDirSource::Portable,
        });
    }

    if let Some(base) = directories::BaseDirs::new() {
        return Ok(DataDirInfo {
            path: base.data_dir().join("caframework"),
            source: DataDirSource::PerOsDefault,
        });
    }

    // Fallbacks for Android & embedded systems where BaseDirs::new() returns None
    if let Ok(files_dir) = std::env::var("FILES_DIR") {
        return Ok(DataDirInfo {
            path: PathBuf::from(files_dir).join("caframework"),
            source: DataDirSource::PerOsDefault,
        });
    }

    if let Ok(home) = std::env::var("HOME") {
        return Ok(DataDirInfo {
            path: PathBuf::from(home).join(".local/share/caframework"),
            source: DataDirSource::PerOsDefault,
        });
    }

    #[cfg(target_os = "android")]
    {
        return Ok(DataDirInfo {
            path: PathBuf::from("/data/data/com.fathforce.caframework/files/caframework"),
            source: DataDirSource::PerOsDefault,
        });
    }

    #[cfg(not(target_os = "android"))]
    Err(CafError::Io(IoError::Generic(
        "unable to determine OS data directory (HOME/APPDATA unreadable)".into(),
    )))
}

pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("caframework.db")
}

pub fn ensure_secure_dir(path: &Path) -> Result<(), CafError> {
    std::fs::create_dir_all(path)
        .map_err(|e| CafError::Io(IoError::Generic(format!("create dir failed: {e}"))))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

pub fn crash_dumps_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("crash-dumps")
}

pub fn device_id() -> Result<String, CafError> {
    // If vault is open, store and load from app_kv inside the vault (P04.5)
    if let Ok(conn) = crate::db::open() {
        let existing: Result<String, _> = conn.query_row(
            "SELECT value FROM app_kv WHERE key = 'origin_device_id'",
            [],
            |r| r.get(0),
        );
        if let Ok(id) = existing
            && !id.trim().is_empty()
        {
            return Ok(id);
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO app_kv (key, value) VALUES ('origin_device_id', ?1)",
            [&new_id],
        );
        return Ok(new_id);
    }

    let dir = data_dir()?;
    let id_file = dir.join("device_id.txt");
    if id_file.is_file() {
        let content = std::fs::read_to_string(&id_file).map_err(|e| {
            CafError::Io(IoError::Generic(format!("failed to read device id: {e}")))
        })?;
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    std::fs::create_dir_all(&dir)
        .map_err(|e| CafError::Io(IoError::Generic(format!("failed to create data dir: {e}"))))?;

    let new_id = uuid::Uuid::new_v4().to_string();
    std::fs::write(&id_file, &new_id)
        .map_err(|e| CafError::Io(IoError::Generic(format!("failed to write device id: {e}"))))?;

    Ok(new_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_takes_precedence() {
        let info = resolve_data_dir_with(Some("/tmp/caf_test_probe")).expect("resolve success");
        assert_eq!(info.source, DataDirSource::EnvOverride);
        assert_eq!(info.path, PathBuf::from("/tmp/caf_test_probe"));
    }

    #[test]
    fn db_path_is_data_dir_slash_caframework_db() {
        let dir = PathBuf::from("/tmp/x");
        assert_eq!(db_path(&dir), PathBuf::from("/tmp/x/caframework.db"));
    }
}
