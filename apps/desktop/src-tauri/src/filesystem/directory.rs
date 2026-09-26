/* ==========================================================================
 * Directory listing
 * Read-only enumeration of a single directory for the browser surface. The
 * listing never recurses, never follows symbolic links or reparse points, and
 * never mutates anything: one directory, one entry per child, deterministic
 * order.
 *
 * Entry metadata comes from the directory entry itself (`DirEntry::metadata`
 * does not traverse symlinks), so listing a large directory stays a single
 * pass with no extra per-child stat calls.
 * ========================================================================== */

use std::path::Path;

use serde::Serialize;

use crate::errors::{AppError, AppResult};
use crate::filesystem::display_name;
use crate::filesystem::path::{must_be_directory, normalize};

/// Upper bound on the entries returned for one directory. A directory larger
/// than this is still listed — truncated and flagged, never an error.
pub const MAX_DIRECTORY_ENTRIES: usize = 10_000;

/// What a directory child is, as far as the platform can tell without
/// following links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Directory,
    File,
    /// Symbolic link, junction, or another reparse point. Deliberately not
    /// resolved: CrossPort reports what the entry is, not what it points at.
    Symlink,
    /// Special entry (device, socket, FIFO) or one whose metadata could not be
    /// read at all.
    Other,
}

impl EntryKind {
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            Self::Symlink
        } else if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_file() {
            Self::File
        } else {
            Self::Other
        }
    }

    /// Ordering rank: directories first, then files, then links and special
    /// entries. Keeps folders above files the way file managers do.
    fn rank(self) -> u8 {
        match self {
            Self::Directory => 0,
            Self::File => 1,
            Self::Symlink | Self::Other => 2,
        }
    }
}

/// One child of a listed directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    /// Absolute path, built by Rust. The frontend never joins paths itself.
    pub path: String,
    pub kind: EntryKind,
    /// Size in bytes for regular files. `None` for directories, links, and
    /// entries whose metadata is unavailable — a directory's own record size
    /// says nothing useful about its contents.
    pub size_bytes: Option<u64>,
    /// Last modification time in milliseconds since the UNIX epoch, when the
    /// platform reports one.
    pub modified_ms: Option<u64>,
    /// Read-only flag when it could be read.
    pub readonly: Option<bool>,
}

/// The result of listing one directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    /// Absolute, normalized directory that was listed.
    pub path: String,
    /// Display name of that directory; a filesystem root keeps its own form.
    pub name: String,
    /// Parent directory, or `None` at a filesystem root. Rust derives it so
    /// the frontend never has to manipulate paths.
    pub parent: Option<String>,
    pub entries: Vec<DirectoryEntry>,
    /// True when the directory holds more entries than [`MAX_DIRECTORY_ENTRIES`].
    pub truncated: bool,
}

/// Validates a frontend-supplied path and lists it.
///
/// The path is normalized first (must be absolute, no null bytes, `..` may not
/// escape its root) and must be an existing directory; a missing path is
/// `path_not_found` and a file is `path_not_directory`.
pub fn list_requested(input: &str) -> AppResult<DirectoryListing> {
    let directory = must_be_directory(normalize(input)?)?;
    list(&directory)
}

/// Lists a directory that has already been validated.
pub fn list(directory: &Path) -> AppResult<DirectoryListing> {
    list_with_limit(directory, MAX_DIRECTORY_ENTRIES)
}

/// Lists at most `limit` entries. Split out from [`list`] so the truncation
/// behaviour is testable without creating thousands of files.
fn list_with_limit(directory: &Path, limit: usize) -> AppResult<DirectoryListing> {
    let children = std::fs::read_dir(directory).map_err(|error| read_error(directory, &error))?;

    let mut entries = Vec::new();
    let mut truncated = false;

    for child in children {
        // A child that vanished or cannot be read while we walk the directory
        // is normal system behaviour; it is skipped instead of failing the
        // whole listing.
        let Ok(child) = child else {
            continue;
        };
        if entries.len() == limit {
            truncated = true;
            break;
        }
        entries.push(describe_entry(&child));
    }

    sort_entries(&mut entries);

    Ok(DirectoryListing {
        path: directory.display().to_string(),
        name: display_name(directory),
        parent: directory
            .parent()
            .map(|parent| parent.display().to_string()),
        entries,
        truncated,
    })
}

/// Describes one child. Uses the entry's own metadata, which does not follow
/// symlinks or reparse points.
fn describe_entry(entry: &std::fs::DirEntry) -> DirectoryEntry {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();

    let Ok(metadata) = entry.metadata() else {
        // The name is still useful even when the entry disappeared between
        // enumeration and inspection.
        return DirectoryEntry {
            name,
            path: path.display().to_string(),
            kind: EntryKind::Other,
            size_bytes: None,
            modified_ms: None,
            readonly: None,
        };
    };

    let kind = EntryKind::from_metadata(&metadata);
    DirectoryEntry {
        name,
        path: path.display().to_string(),
        kind,
        size_bytes: (kind == EntryKind::File).then(|| metadata.len()),
        modified_ms: modified_ms(&metadata),
        readonly: Some(metadata.permissions().readonly()),
    }
}

/// Modification time in milliseconds since the UNIX epoch. `None` when the
/// platform cannot report it or the timestamp predates the epoch.
fn modified_ms(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as u64)
}

/// Deterministic order: directories, then files, then links and special
/// entries; case-insensitive by name, with the exact name as a tie-breaker so
/// two names that differ only in case keep a stable relative order.
fn sort_entries(entries: &mut [DirectoryEntry]) {
    entries.sort_by(|left, right| {
        left.kind
            .rank()
            .cmp(&right.kind.rank())
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
}

/// Maps a `read_dir` failure onto the structured error categories the
/// frontend switches on. A directory that disappeared between validation and
/// listing is reported as missing, not as an opaque I/O failure.
fn read_error(directory: &Path, error: &std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::PathNotFound(directory.display().to_string()),
        std::io::ErrorKind::PermissionDenied => {
            AppError::PermissionDenied(directory.display().to_string())
        }
        _ => AppError::Io(format!("failed to read '{}': {error}", directory.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;
    use std::path::PathBuf;

    fn find<'a>(listing: &'a DirectoryListing, name: &str) -> &'a DirectoryEntry {
        listing
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("'{name}' should be listed: {:?}", listing.entries))
    }

    /// Creates a file symlink, returning `false` when the platform refuses
    /// (Windows without developer mode). Symlink tests skip themselves then.
    fn try_symlink_file(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, link);
            false
        }
    }

    #[test]
    fn lists_directories_before_files_and_sorts_names_case_insensitively() {
        let dir = unique_temp_dir("directory-order");
        for child in ["beta", "alpha"] {
            std::fs::create_dir_all(dir.join(child)).expect("subdir");
        }
        for child in ["Beta.txt", "alpha.txt"] {
            std::fs::write(dir.join(child), b"x").expect("file");
        }

        let listing = list(&dir).expect("directory is listed");

        let names: Vec<&str> = listing
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["alpha", "beta", "alpha.txt", "Beta.txt"]);
        assert!(listing.entries[0].kind == EntryKind::Directory);
        assert!(listing.entries[2].kind == EntryKind::File);

        clean_up(&dir);
    }

    #[test]
    fn reports_file_metadata_and_paths() {
        let dir = unique_temp_dir("directory-metadata");
        let file = dir.join("payload.bin");
        std::fs::write(&file, vec![0u8; 4096]).expect("file");

        let listing = list(&dir).expect("directory is listed");
        let entry = find(&listing, "payload.bin");

        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size_bytes, Some(4096));
        assert_eq!(entry.path, file.display().to_string());
        assert!(entry.modified_ms.is_some());
        assert_eq!(entry.readonly, Some(false));

        clean_up(&dir);
    }

    #[test]
    fn reports_directories_without_a_size() {
        let dir = unique_temp_dir("directory-kinds");
        std::fs::create_dir_all(dir.join("nested")).expect("subdir");

        let listing = list(&dir).expect("directory is listed");
        let entry = find(&listing, "nested");

        assert_eq!(entry.kind, EntryKind::Directory);
        assert_eq!(
            entry.size_bytes, None,
            "a directory record size is not its content size"
        );

        clean_up(&dir);
    }

    #[test]
    fn reports_the_listed_directory_and_its_parent() {
        let dir = unique_temp_dir("directory-header");
        let nested = dir.join("nested");
        std::fs::create_dir_all(&nested).expect("subdir");

        let listing = list(&nested).expect("directory is listed");

        assert_eq!(listing.path, nested.display().to_string());
        assert_eq!(listing.name, "nested");
        assert_eq!(
            listing.parent,
            Some(dir.display().to_string()),
            "Rust derives the parent so the frontend never has to"
        );

        clean_up(&dir);
    }

    #[test]
    fn reports_the_name_and_parent_of_a_filesystem_root() {
        let root = if cfg!(windows) { "C:\\" } else { "/" };

        let listing = list(Path::new(root)).expect("the root is a directory");

        assert!(!listing.name.is_empty());
        assert_eq!(
            listing.parent, None,
            "a filesystem root has no parent to navigate to"
        );
    }

    #[test]
    fn lists_an_empty_directory() {
        let dir = unique_temp_dir("directory-empty");

        let listing = list(&dir).expect("directory is listed");

        assert!(listing.entries.is_empty());
        assert!(!listing.truncated);

        clean_up(&dir);
    }

    #[test]
    fn truncates_oversized_directories_instead_of_failing() {
        let dir = unique_temp_dir("directory-limit");
        for index in 0..3 {
            std::fs::write(dir.join(format!("file-{index}.txt")), b"x").expect("file");
        }

        let listing = list_with_limit(&dir, 2).expect("directory is listed");

        assert_eq!(listing.entries.len(), 2);
        assert!(listing.truncated);

        clean_up(&dir);
    }

    #[test]
    fn does_not_flag_truncation_when_the_limit_is_exact() {
        let dir = unique_temp_dir("directory-limit-exact");
        for index in 0..2 {
            std::fs::write(dir.join(format!("file-{index}.txt")), b"x").expect("file");
        }

        let listing = list_with_limit(&dir, 2).expect("directory is listed");

        assert_eq!(listing.entries.len(), 2);
        assert!(!listing.truncated);

        clean_up(&dir);
    }

    #[test]
    fn rejects_relative_paths_before_touching_the_filesystem() {
        for input in ["foo", "foo/bar", "./foo", "../foo"] {
            let error = list_requested(input).expect_err("relative paths are invalid");
            assert_eq!(error.code(), "invalid_input", "input: {input}");
        }
    }

    #[test]
    fn rejects_null_bytes_and_empty_paths() {
        assert_eq!(
            list_requested("C:\\temp\\\0evil")
                .expect_err("null bytes are invalid")
                .code(),
            "invalid_input"
        );
        assert_eq!(
            list_requested("   ")
                .expect_err("blank paths are invalid")
                .code(),
            "invalid_input"
        );
    }

    #[test]
    fn rejects_paths_that_escape_their_root() {
        let escaping = if cfg!(windows) {
            "C:\\..\\..\\Windows"
        } else {
            "/../etc"
        };

        let error = list_requested(escaping).expect_err("traversal is rejected");

        assert_eq!(error.code(), "invalid_input");
    }

    #[test]
    fn reports_a_missing_directory_as_path_not_found() {
        let dir = unique_temp_dir("directory-missing");
        let missing = dir.join("gone");

        let error = list_requested(&missing.display().to_string())
            .expect_err("the directory does not exist");

        assert_eq!(error.code(), "path_not_found");
        clean_up(&dir);
    }

    #[test]
    fn reports_a_file_as_path_not_directory() {
        let dir = unique_temp_dir("directory-file");
        let file = dir.join("notes.txt");
        std::fs::write(&file, b"hello").expect("file");

        let error =
            list_requested(&file.display().to_string()).expect_err("files are not directories");

        assert_eq!(error.code(), "path_not_directory");
        clean_up(&dir);
    }

    #[test]
    fn maps_an_inaccessible_directory_to_permission_denied() {
        let error = read_error(
            Path::new("C:\\Locked"),
            &std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
        );

        assert_eq!(error.code(), "permission_denied");
    }

    #[test]
    fn maps_a_disappearing_directory_to_path_not_found() {
        let error = read_error(
            Path::new("C:\\Gone"),
            &std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        );

        assert_eq!(error.code(), "path_not_found");
        assert!(error.to_string().contains("C:\\Gone"));
    }

    #[test]
    fn keeps_unmapped_read_failures_under_the_io_code_with_context() {
        let error = read_error(
            Path::new("C:\\Broken"),
            &std::io::Error::new(std::io::ErrorKind::WouldBlock, "busy"),
        );

        assert_eq!(error.code(), "io");
        assert!(error.to_string().contains("C:\\Broken"));
    }

    #[test]
    fn does_not_follow_symbolic_links() {
        let dir = unique_temp_dir("directory-symlink");
        let target = dir.join("real.txt");
        std::fs::write(&target, b"payload").expect("file");
        let link = dir.join("link.txt");

        if !try_symlink_file(&target, &link) {
            // Windows without developer mode cannot create symlinks; the
            // behaviour is covered on platforms that can.
            clean_up(&dir);
            return;
        }

        let listing = list(&dir).expect("directory is listed");
        let entry = find(&listing, "link.txt");

        assert_eq!(
            entry.kind,
            EntryKind::Symlink,
            "links are reported as links, not resolved"
        );
        assert!(!entry.name.is_empty());
        assert!(entry.path.ends_with("link.txt"));

        // The link target is still listed as the regular file it is.
        assert_eq!(find(&listing, "real.txt").kind, EntryKind::File);

        clean_up(&dir);
    }

    #[test]
    fn lists_a_broken_symbolic_link_without_failing() {
        let dir = unique_temp_dir("directory-broken-link");
        let link = dir.join("dangling.txt");

        if !try_symlink_file(&dir.join("not-there.txt"), &link) {
            clean_up(&dir);
            return;
        }

        let listing = list(&dir).expect("a broken link does not fail the listing");
        let entry = find(&listing, "dangling.txt");

        assert_eq!(entry.kind, EntryKind::Symlink);
        assert_eq!(entry.size_bytes, None);

        clean_up(&dir);
    }

    #[test]
    fn serializes_exactly_the_frontend_contract() {
        let listing = DirectoryListing {
            path: "C:\\Users".to_string(),
            name: "Users".to_string(),
            parent: Some("C:\\".to_string()),
            entries: vec![DirectoryEntry {
                name: "notes.txt".to_string(),
                path: "C:\\Users\\notes.txt".to_string(),
                kind: EntryKind::File,
                size_bytes: Some(12),
                modified_ms: Some(1_700_000_000_000),
                readonly: Some(false),
            }],
            truncated: false,
        };

        let json = serde_json::to_value(&listing).expect("listing serializes");

        assert_eq!(
            json,
            serde_json::json!({
                "path": "C:\\Users",
                "name": "Users",
                "parent": "C:\\",
                "entries": [{
                    "name": "notes.txt",
                    "path": "C:\\Users\\notes.txt",
                    "kind": "file",
                    "sizeBytes": 12,
                    "modifiedMs": 1_700_000_000_000u64,
                    "readonly": false,
                }],
                "truncated": false,
            })
        );
        assert!(
            json.get("size_bytes").is_none(),
            "the IPC contract is camelCase"
        );
    }

    #[test]
    fn entry_kind_identifiers_are_stable_on_the_wire() {
        let cases = [
            (EntryKind::Directory, "directory"),
            (EntryKind::File, "file"),
            (EntryKind::Symlink, "symlink"),
            (EntryKind::Other, "other"),
        ];

        for (kind, identifier) in cases {
            let json = serde_json::to_value(kind).expect("entry kind serializes");
            assert_eq!(json, serde_json::json!(identifier));
        }
    }

    #[test]
    fn listing_a_directory_with_only_links_keeps_the_entries() {
        let dir = unique_temp_dir("directory-link-order");
        let target: PathBuf = dir.join("target.txt");
        std::fs::write(&target, b"x").expect("file");
        if !try_symlink_file(&target, &dir.join("z-link.txt")) {
            clean_up(&dir);
            return;
        }
        std::fs::write(dir.join("a-file.txt"), b"x").expect("file");

        let listing = list(&dir).expect("directory is listed");

        let names: Vec<&str> = listing
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["a-file.txt", "target.txt", "z-link.txt"]);

        clean_up(&dir);
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
