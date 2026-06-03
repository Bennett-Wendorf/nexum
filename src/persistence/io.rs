//! Core file I/O utilities for the persistence layer.
//!
//! All functions return [`Result`] from the persistence layer, mapping
//! `std::io::Error` to [`PersistenceError::Io`] and `serde_json::Error`
//! to [`PersistenceError::JsonParse`], each carrying path context.

use std::fs;
use std::io;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::errors::{PersistenceError, Result};

/// Read the entire contents of a file as a UTF-8 string.
///
/// Returns [`PersistenceError::FileNotFound`] if the file does not exist,
/// or [`PersistenceError::Io`] for other I/O errors.
pub fn read_file(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    fs::read_to_string(&path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            PersistenceError::FileNotFound(path.clone())
        } else {
            PersistenceError::Io(path, e)
        }
    })
}

/// Write a string to a file, overwriting any existing content.
///
/// The parent directory is **not** created automatically.
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    let path = path.to_path_buf();
    fs::write(&path, content).map_err(|e| PersistenceError::Io(path, e))
}

/// Check whether a file exists at the given path.
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Check whether a directory exists at the given path.
pub fn directory_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// Write content to a file atomically.
///
/// The content is first written to a temporary file in the same directory
/// (`.tmp-{random:8}`), then renamed into place via `fs::rename`. If the
/// rename fails, the temp file is cleaned up.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let path = path.to_path_buf();
    let parent = match path.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            return Err(PersistenceError::PathResolution(
                "Cannot determine parent directory for atomic write".into(),
            ))
        }
    };

    let random: String = (0..8)
        .map(|_| b"abcdef0123456789"[fastrand::u8(0..16) as usize] as char)
        .collect();
    let temp_path = parent.join(format!(".tmp-{}", random));

    fs::write(&temp_path, content).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        PersistenceError::Io(temp_path.clone(), e)
    })?;

    fs::rename(&temp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        PersistenceError::AtomicWrite(path, e)
    })
}

/// Read a file and deserialize its JSON contents into a typed value.
///
/// Returns [`PersistenceError::JsonParse`] on deserialization failure.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let path = path.to_path_buf();
    let content = read_file(path.as_path())?;
    serde_json::from_str(&content).map_err(|e| PersistenceError::JsonParse(path, e))
}

/// Serialize a value to pretty-printed JSON (2-space indent) and write it to a file.
pub fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let path = path.to_path_buf();
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| PersistenceError::JsonSerialize(path.clone(), e))?;
    fs::write(&path, json).map_err(|e| PersistenceError::Io(path, e))
}

/// Serialize a value to JSON and write it atomically.
pub fn atomic_write_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| PersistenceError::JsonParseBare(e))?;
    atomic_write(path, &json)
}

/// Recursively create a directory and all of its parent components.
pub fn create_dir_all(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    fs::create_dir_all(&path).map_err(|e| PersistenceError::Io(path, e))
}

/// List all entries in a directory.
pub fn list_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let path = path.to_path_buf();
    let mut entries = fs::read_dir(&path)
        .map_err(|e| PersistenceError::Io(path.clone(), e))?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// Remove a file.
pub fn remove_file(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    fs::remove_file(&path).map_err(|e| PersistenceError::Io(path, e))
}

/// Recursively remove a directory and all of its contents.
pub fn remove_dir_all(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    fs::remove_dir_all(&path).map_err(|e| PersistenceError::Io(path, e))
}
