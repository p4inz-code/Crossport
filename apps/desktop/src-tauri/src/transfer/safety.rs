/* ==========================================================================
 * Transfer safety
 * The rules that must hold before data moves, and the small filesystem
 * primitives the copy and move paths share.
 *
 * Everything in this module is deliberately conservative: when the engine
 * cannot prove an operation is safe, it refuses or leaves the existing data
 * alone rather than guessing. The frontend is never trusted here — Rust
 * validates every path, and the relationship checks run before the first byte
 * of a job, not after it has already started copying.
 * ========================================================================== */

use std::io;
use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};
use crate::filesystem::path::normalize;
use crate::platform;
use crate::platform::paths as platform_paths;
use crate::transfer::model::{ConflictStrategy, ItemKind};

/// What a source path turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    File,
    Directory,
    /// A symlink, junction, or other reparse point, or a special entry the
    /// engine reports but never follows, copies, or deletes.
    Unsupported,
}

/// A validated source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFacts {
    /// Absolute, lexically clean path.
    pub path: PathBuf,
    pub kind: SourceKind,
    /// Size in bytes for regular files; `0` for everything else.
    pub size_bytes: u64,
}

impl SourceFacts {
    /// The name the source takes inside the destination directory.
    pub fn name(&self) -> String {
        crate::filesystem::display_name(&self.path)
    }
}

/// Validates one source path coming from the frontend.
///
/// Rejects a malformed path (`invalid_input`), a path that does not exist
/// (`path_not_found`), and a path the process may not read
/// (`permission_denied`) — the last one by asking for its metadata, which the
/// OS will not return without at least directory-traverse or file-read access.
pub(crate) fn inspect_source(input: &str) -> AppResult<SourceFacts> {
    let path = normalize(input)?;
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| read_error(error, &path))?;

    // A reparse point is reported as a link no matter what it points at, and
    // is never followed: walking through one could leave the tree the user
    // selected, or loop forever.
    if platform::is_reparse_point(&metadata) {
        return Ok(SourceFacts {
            path,
            kind: SourceKind::Unsupported,
            size_bytes: 0,
        });
    }

    let file_type = metadata.file_type();
    if file_type.is_dir() {
        return Ok(SourceFacts {
            path,
            kind: SourceKind::Directory,
            size_bytes: 0,
        });
    }
    if file_type.is_file() {
        return Ok(SourceFacts {
            path,
            size_bytes: metadata.len(),
            kind: SourceKind::File,
        });
    }

    Ok(SourceFacts {
        path,
        kind: SourceKind::Unsupported,
        size_bytes: 0,
    })
}

/// Validates the destination directory.
///
/// The destination must already exist: CrossPort transfers into a folder the
/// user chose, and inventing one the user never picked would be guessing.
/// Probing every missing ancestor for write access needs the probe, so it is
/// skipped for a read-only planning pass.
pub(crate) fn validate_destination(input: &str, probe: bool) -> AppResult<PathBuf> {
    let path = normalize(input)?;

    let metadata = std::fs::symlink_metadata(&path).map_err(|error| read_error(error, &path))?;
    if !metadata.is_dir() {
        return Err(AppError::PathNotDirectory(path.display().to_string()));
    }

    // Listing proves the directory can be read; it is the cheapest honest
    // check `std` offers without creating anything.
    std::fs::read_dir(&path).map_err(|error| read_error(error, &path))?;

    if probe {
        probe_writable(&path)?;
    }

    Ok(path)
}

/// Rejects the source/destination relationships the engine will not act on.
///
/// `destination_child` is where the source itself would land. The rules:
///
/// - the destination may never be *inside* the source folder, which is how a
///   folder ends up copying into itself (`C:\Data` → `C:\Data\Backup`);
/// - a destination that resolves to the source itself is refused, except with
///   the rename strategy, where it becomes a new sibling instead
///   (`C:\Data` → `C:\Data (2)`). Copying, replacing, or skipping a path with
///   itself is never what the user meant, and the request is refused before
///   anything is enumerated.
pub(crate) fn ensure_safe_relationship(
    source: &Path,
    destination_child: &Path,
    kind: SourceKind,
    strategy: ConflictStrategy,
) -> AppResult<()> {
    if !platform_paths::path_contains(source, destination_child) {
        return Ok(());
    }

    let is_self = platform_paths::same_path(source, destination_child);
    let builds_a_sibling = is_self
        && matches!(kind, SourceKind::File | SourceKind::Directory)
        && strategy == ConflictStrategy::Rename;

    if builds_a_sibling {
        // Nothing is overwritten: rename never reuses a name, so the copy lands
        // beside the original.
        return Ok(());
    }

    if is_self {
        return Err(AppError::UnsafeRelationship(format!(
            "'{}' would be written over itself; choose the rename conflict strategy to place a copy beside it",
            source.display()
        )));
    }

    Err(AppError::UnsafeRelationship(format!(
        "destination '{}' is inside the source folder '{}'",
        destination_child.display(),
        source.display()
    )))
}

/// Removes one existing destination entry of a known kind.
///
/// Only ever called for a planned `Replace`, and only for a path the plan
/// resolved inside the requested destination. An entry that vanished in the
/// meantime is not an error.
pub(crate) fn remove_entry(path: &Path, kind: ItemKind) -> AppResult<()> {
    let result = match kind {
        ItemKind::File => std::fs::remove_file(path),
        ItemKind::Directory => std::fs::remove_dir_all(path),
    };

    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(write_error(error, path)),
    }
}

/// Creates the missing ancestors of `path` (including `path` itself) and
/// reports exactly which directories were created, shallowest first.
///
/// Precise bookkeeping is what lets an aborted transfer clean up after itself:
/// only directories this engine created are ever considered for removal, and
/// only when they are still empty.
pub(crate) fn create_chain(path: &Path) -> AppResult<Vec<PathBuf>> {
    let mut missing: Vec<PathBuf> = Vec::new();
    let mut current = Some(path);

    while let Some(candidate) = current {
        match std::fs::symlink_metadata(candidate) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(AppError::PathNotDirectory(candidate.display().to_string()));
                }
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing.push(candidate.to_path_buf());
            }
            // POSIX reports ENOTDIR for a path whose ancestor is a file where
            // Windows reports not-found. Answer with the same structured error
            // either way, naming the entry that is not a directory.
            Err(error) => match crate::filesystem::blocking_file(candidate) {
                Some(blocking) => {
                    return Err(AppError::PathNotDirectory(blocking.display().to_string()))
                }
                None => return Err(write_error(error, candidate)),
            },
        }
        current = candidate.parent();
    }

    missing.reverse();
    for directory in &missing {
        match std::fs::create_dir(directory) {
            Ok(()) => {}
            // Another process created it first; the intent is satisfied.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(write_error(error, directory)),
        }
    }

    Ok(missing)
}

/// Removes the directories an aborted transfer created, deepest first.
///
/// `remove_dir` refuses to touch a non-empty directory, so anything the user or
/// another process has put there survives untouched. Failures are ignored on
/// purpose: cleanup must never turn into a second error the user has to read.
pub(crate) fn clean_up_created_directories(created: &[PathBuf]) {
    let mut directories: Vec<&PathBuf> = created.iter().collect();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    directories.dedup();

    for directory in directories {
        let _ = std::fs::remove_dir(directory);
    }
}

/// Maps a failure while writing, creating, or removing a path onto the closest
/// structured category, including the one case the user can act on fastest.
pub(crate) fn write_error(error: io::Error, path: &Path) -> AppError {
    if platform::is_disk_full_error(&error) {
        return AppError::DiskFull(format!(
            "'{}' could not be written: {error}",
            path.display()
        ));
    }

    match error.kind() {
        io::ErrorKind::NotFound => AppError::PathNotFound(path.display().to_string()),
        io::ErrorKind::PermissionDenied => AppError::PermissionDenied(path.display().to_string()),
        io::ErrorKind::InvalidInput => AppError::InvalidInput(format!(
            "'{}' is not a usable path: {error}",
            path.display()
        )),
        _ => AppError::Io(format!(
            "'{}' could not be written: {error}",
            path.display()
        )),
    }
}

/// Maps a failure while reading a path onto the closest structured category.
pub(crate) fn read_error(error: io::Error, path: &Path) -> AppError {
    match error.kind() {
        io::ErrorKind::NotFound => AppError::PathNotFound(path.display().to_string()),
        io::ErrorKind::PermissionDenied => AppError::PermissionDenied(path.display().to_string()),
        _ => AppError::Io(format!("'{}' could not be read: {error}", path.display())),
    }
}

/// Writes and removes a probe file inside `directory`.
///
/// Read access and write access are different questions, and only a write
/// answers the second one. The probe is removed immediately; a leftover probe
/// file would be the only trace of a failed probe, and it is removed even then
/// as far as the OS allows.
fn probe_writable(directory: &Path) -> AppResult<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static PROBE: AtomicUsize = AtomicUsize::new(0);

    let name = format!(
        ".crossport-write-probe-{}-{}",
        std::process::id(),
        PROBE.fetch_add(1, Ordering::Relaxed)
    );
    let probe = directory.join(name);

    match std::fs::File::create(&probe) {
        Ok(file) => {
            drop(file);
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(error) => Err(write_error(error, directory)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    fn unique_temp_dir_label(label: &str) -> PathBuf {
        unique_temp_dir(label)
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn dir_source(dir: &Path) -> PathBuf {
        dir.join("Data")
    }

    #[test]
    fn inspect_source_reports_files_and_directories() {
        let dir = unique_temp_dir_label("safety-inspect");
        let file = dir.join("payload.bin");
        std::fs::write(&file, vec![7u8; 321]).expect("file is writable");
        std::fs::create_dir_all(dir.join("folder")).expect("subdir");

        let file_facts = inspect_source(&file.display().to_string()).expect("file exists");
        assert_eq!(file_facts.kind, SourceKind::File);
        assert_eq!(file_facts.size_bytes, 321);
        assert_eq!(file_facts.name(), "payload.bin");

        let dir_facts =
            inspect_source(&dir.join("folder").display().to_string()).expect("directory exists");
        assert_eq!(dir_facts.kind, SourceKind::Directory);
        assert_eq!(dir_facts.size_bytes, 0);

        clean_up(&dir);
    }

    #[test]
    fn inspect_source_rejects_the_paths_the_backend_must_not_accept() {
        assert_eq!(
            inspect_source("relative/path")
                .expect_err("relative paths are invalid")
                .code(),
            "invalid_input"
        );
        // An absolute path that does not exist, built with the platform's own
        // separators: a Windows-style drive path is not absolute on POSIX, where
        // it would be rejected as invalid input instead.
        let missing = unique_temp_dir_label("safety-missing").join("definitely-missing");
        assert_eq!(
            inspect_source(&missing.display().to_string())
                .expect_err("missing paths are reported")
                .code(),
            "path_not_found"
        );

        clean_up(missing.parent().expect("the temp directory has a parent"));
    }

    #[test]
    fn inspect_source_treats_reparse_points_as_unsupported() {
        let dir = unique_temp_dir_label("safety-link");
        let target = dir.join("target.txt");
        std::fs::write(&target, b"payload").expect("file is writable");
        let link = dir.join("link.txt");

        if !try_symlink_file(&target, &link) {
            // Windows without developer mode cannot create symlinks.
            clean_up(&dir);
            return;
        }

        let facts = inspect_source(&link.display().to_string()).expect("link exists");

        assert_eq!(
            facts.kind,
            SourceKind::Unsupported,
            "links are never followed or copied"
        );
        assert_eq!(facts.size_bytes, 0);

        clean_up(&dir);
    }

    #[test]
    fn validate_destination_requires_an_existing_directory() {
        let dir = unique_temp_dir_label("safety-destination");
        let file = dir.join("notes.txt");
        std::fs::write(&file, b"x").expect("file is writable");

        assert_eq!(
            validate_destination(&dir.display().to_string(), false)
                .expect("an existing directory is a valid destination"),
            dir
        );
        assert_eq!(
            validate_destination(&file.display().to_string(), false)
                .expect_err("a file is not a destination")
                .code(),
            "path_not_directory"
        );
        assert_eq!(
            validate_destination(&dir.join("gone").display().to_string(), false)
                .expect_err("a missing directory is not a destination")
                .code(),
            "path_not_found"
        );

        clean_up(&dir);
    }

    #[test]
    fn validate_destination_probe_leaves_nothing_behind() {
        let dir = unique_temp_dir_label("safety-probe");

        validate_destination(&dir.display().to_string(), true).expect("the directory is writable");

        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("directory is readable")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            leftovers.is_empty(),
            "the write probe must clean up after itself: {leftovers:?}"
        );

        clean_up(&dir);
    }

    #[test]
    fn relationship_rejects_a_folder_inside_itself() {
        let dir = unique_temp_dir_label("safety-inside");
        let source = dir_source(&dir);
        std::fs::create_dir_all(&source).expect("subdir");

        let error = ensure_safe_relationship(
            &source,
            &source.join("Backup").join("Data"),
            SourceKind::Directory,
            ConflictStrategy::Replace,
        )
        .expect_err("copying a folder into itself is refused");

        assert_eq!(error.code(), "unsafe_relationship");
        assert!(error.to_string().contains("inside the source folder"));

        clean_up(&dir);
    }

    #[test]
    fn relationship_rejects_replacing_a_path_with_itself() {
        let dir = unique_temp_dir_label("safety-self-replace");
        let source = dir_source(&dir);
        std::fs::create_dir_all(&source).expect("subdir");
        for strategy in [ConflictStrategy::Replace, ConflictStrategy::Skip] {
            let error = ensure_safe_relationship(&source, &source, SourceKind::Directory, strategy)
                .expect_err("a folder is never copied over, replaced, or skipped onto itself");
            assert_eq!(error.code(), "unsafe_relationship");
        }

        clean_up(&dir);
    }

    #[test]
    fn relationship_allows_a_renamed_sibling_copy() {
        let dir = unique_temp_dir_label("safety-sibling");
        let source = dir_source(&dir);
        std::fs::create_dir_all(&source).expect("subdir");

        ensure_safe_relationship(
            &source,
            &source,
            SourceKind::Directory,
            ConflictStrategy::Rename,
        )
        .expect("a renamed copy beside the original is what the user asked for");

        clean_up(&dir);
    }

    #[test]
    fn relationship_allows_a_sibling_that_is_not_inside_the_source() {
        let dir = unique_temp_dir_label("safety-outside");
        let source = dir_source(&dir);
        std::fs::create_dir_all(&source).expect("subdir");

        for strategy in [
            ConflictStrategy::Replace,
            ConflictStrategy::Skip,
            ConflictStrategy::Rename,
        ] {
            ensure_safe_relationship(
                &source,
                &dir.join("Data (2)"),
                SourceKind::Directory,
                strategy,
            )
            .expect("a destination outside the source is always fine");
            ensure_safe_relationship(
                &source,
                &source.join("child"),
                SourceKind::Directory,
                strategy,
            )
            .expect_err("a destination inside the source is never fine");
        }

        clean_up(&dir);
    }

    #[test]
    fn create_chain_reports_exactly_what_it_created() {
        let dir = unique_temp_dir_label("safety-chain");
        let target = dir.join("a").join("b").join("c");

        let created = create_chain(&target).expect("the chain is creatable");

        assert_eq!(
            created,
            vec![dir.join("a"), dir.join("a").join("b"), target.clone()]
        );
        assert!(target.is_dir());

        // A second call creates nothing, so an abort cannot remove directories
        // another job owns.
        assert!(create_chain(&target).expect("already exists").is_empty());

        clean_up(&dir);
    }

    #[test]
    fn create_chain_rejects_a_file_in_the_way() {
        let dir = unique_temp_dir_label("safety-chain-file");
        std::fs::write(dir.join("a"), b"x").expect("file is writable");

        let error = create_chain(&dir.join("a").join("b")).expect_err("a file blocks the chain");

        assert_eq!(error.code(), "path_not_directory");

        clean_up(&dir);
    }

    #[test]
    fn clean_up_removes_only_empty_directories_it_created() {
        let dir = unique_temp_dir_label("safety-cleanup");
        let created = create_chain(&dir.join("out").join("deep")).expect("chain is creatable");
        let kept = dir.join("out").join("deep").join("user-file.txt");
        std::fs::write(&kept, b"user data").expect("file is writable");

        clean_up_created_directories(&created);

        assert!(kept.exists(), "user data is never removed by cleanup");
        assert!(dir.join("out").join("deep").is_dir());

        // With nothing left inside, the created directories go away deepest
        // first.
        std::fs::remove_file(&kept).expect("file is removable");
        clean_up_created_directories(&created);
        assert!(!dir.join("out").exists());

        clean_up(&dir);
    }

    #[test]
    fn remove_entry_removes_files_and_folders_and_ignores_vanished_entries() {
        let dir = unique_temp_dir_label("safety-remove");
        let file = dir.join("a.txt");
        std::fs::write(&file, b"x").expect("file is writable");
        let folder = dir.join("folder");
        std::fs::create_dir_all(folder.join("nested")).expect("subdir");

        remove_entry(&file, ItemKind::File).expect("file is removed");
        remove_entry(&folder, ItemKind::Directory).expect("folder is removed");
        remove_entry(&file, ItemKind::File).expect("a vanished entry is not an error");

        assert!(!file.exists());
        assert!(!folder.exists());

        clean_up(&dir);
    }

    #[test]
    fn disk_full_is_reported_as_its_own_code() {
        let full = std::io::Error::from_raw_os_error(if cfg!(windows) { 112 } else { 28 });
        assert_eq!(
            write_error(full, Path::new("D:\\big.bin")).code(),
            "disk_full"
        );
    }

    #[test]
    fn write_and_read_errors_keep_their_structured_categories() {
        let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(
            write_error(denied, Path::new("C:\\Windows")).code(),
            "permission_denied"
        );

        let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(
            read_error(missing, Path::new("C:\\gone")).code(),
            "path_not_found"
        );

        let busy = std::io::Error::new(std::io::ErrorKind::WouldBlock, "busy");
        let mapped = write_error(busy, Path::new("C:\\busy.bin"));
        assert_eq!(mapped.code(), "io");
        assert!(mapped.to_string().contains("C:\\busy.bin"));
    }

    /// Creates a file symlink, returning `false` when the platform refuses
    /// (Windows without developer mode).
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
}
