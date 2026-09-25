/* ==========================================================================
 * Metadata foundation
 * A read-only description of a single path. This is what the UI needs to show
 * a folder or file without the webview touching the filesystem itself.
 * ========================================================================== */

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::errors::AppResult;

/// Read-only metadata for one path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMetadata {
    /// The path as inspected, already normalized and absolute.
    pub path: String,
    /// Final component for display; a root such as `C:\` keeps its own form.
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    /// Whether the entry itself is a symlink (the metadata follows links).
    pub is_symlink: bool,
    /// Size in bytes. Meaningful for files; directories report the value the
    /// platform returns for the directory entry itself.
    pub size_bytes: u64,
    /// Last modification time in milliseconds since the UNIX epoch. `None`
    /// when the platform cannot report it (for example a pre-1970 timestamp).
    pub modified_ms: Option<u64>,
    pub readonly: bool,
}

/// Inspects a path, following symlinks for the reported kind and size.
pub fn describe(path: &Path) -> AppResult<EntryMetadata> {
    let metadata = std::fs::metadata(path)?;

    // `metadata` follows links, so a separate lstat tells us whether the entry
    // the user picked is itself a symlink.
    let is_symlink = std::fs::symlink_metadata(path)
        .map(|entry| entry.file_type().is_symlink())
        .unwrap_or(false);

    Ok(EntryMetadata {
        path: path.display().to_string(),
        name: display_name(path),
        is_dir: metadata.is_dir(),
        is_file: metadata.is_file(),
        is_symlink,
        size_bytes: metadata.len(),
        modified_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_millis() as u64),
        readonly: metadata.permissions().readonly(),
    })
}

/// `file_name` is `None` for filesystem roots, which would otherwise produce an
/// empty label.
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    #[test]
    fn describes_a_file() {
        let dir = unique_temp_dir("metadata-file");
        let file = dir.join("payload.bin");
        std::fs::write(&file, vec![0u8; 2048]).expect("file should be writable");

        let described = describe(&file).expect("file exists");

        assert_eq!(described.name, "payload.bin");
        assert_eq!(described.path, file.display().to_string());
        assert!(described.is_file);
        assert!(!described.is_dir);
        assert!(!described.is_symlink);
        assert_eq!(described.size_bytes, 2048);
        assert!(
            described.modified_ms.is_some(),
            "a freshly written file has a modification time"
        );
        clean_up(&dir);
    }

    #[test]
    fn describes_a_directory() {
        let dir = unique_temp_dir("metadata-dir");

        let described = describe(&dir).expect("directory exists");

        assert!(described.is_dir);
        assert!(!described.is_file);
        assert!(described.name.starts_with("crossport-test-metadata-dir"));
        clean_up(&dir);
    }

    #[test]
    fn describes_a_filesystem_root_with_a_name() {
        let root = if cfg!(windows) { "C:\\" } else { "/" };

        let described = describe(Path::new(root)).expect("the root exists");

        assert!(described.is_dir);
        assert!(!described.name.is_empty());
    }

    #[test]
    fn reports_a_missing_path_as_path_not_found() {
        let dir = unique_temp_dir("metadata-missing");

        let error = describe(&dir.join("gone")).expect_err("path does not exist");

        assert_eq!(error.code(), "path_not_found");
        clean_up(&dir);
    }

    #[test]
    fn serializes_with_camel_case_keys() {
        let dir = unique_temp_dir("metadata-json");

        let json = serde_json::to_value(describe(&dir).expect("directory exists"))
            .expect("metadata serializes");

        assert!(json.get("sizeBytes").is_some());
        assert!(json.get("modifiedMs").is_some());
        assert!(json.get("isDir").is_some());
        assert!(json.get("isSymlink").is_some());
        assert!(
            json.get("size_bytes").is_none(),
            "the IPC contract is camelCase"
        );
        clean_up(&dir);
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
