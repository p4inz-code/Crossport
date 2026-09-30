/* ==========================================================================
 * Filesystem foundation
 * The safe primitives the transfer engine will be built on: absolute path
 * normalization, directory validation, metadata inspection, and directory
 * listing for the browser surface. Nothing here mutates the filesystem —
 * CrossPort reads and validates only.
 * ========================================================================== */

pub mod ancestors;
pub mod directory;
pub mod metadata;
pub mod path;

use std::path::{Path, PathBuf};

/// The existing entry in front of `path` that is not a directory, if there is
/// one.
///
/// A path that runs through a file can never exist. Windows reports that as
/// "not found" and POSIX as ENOTDIR, so callers that must reach the same
/// verdict on both platforms ask here instead of reading the OS's error kind —
/// `io::ErrorKind::NotADirectory` would raise the crate's declared MSRV
/// (1.77.2) just to reword an error.
pub(crate) fn blocking_file(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path.parent();
    while let Some(candidate) = ancestor {
        if let Ok(metadata) = std::fs::symlink_metadata(candidate) {
            return if metadata.is_dir() {
                None
            } else {
                Some(candidate.to_path_buf())
            };
        }
        ancestor = candidate.parent();
    }
    None
}

/// Final path component for display. A filesystem root has no such component,
/// so it keeps its own form (`C:\`, `/`) instead of rendering as an empty
/// label.
pub(crate) fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Creates a unique temporary directory for filesystem tests. Tests are
    /// responsible for removing it again.
    pub fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "crossport-test-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("test directory should be creatable");
        dir
    }
}

#[cfg(test)]
mod tests {
    use super::blocking_file;
    use crate::filesystem::test_support::unique_temp_dir;
    use std::fs;

    /// A path that runs through a regular file can never exist. Windows reports
    /// that as "not found" and POSIX as ENOTDIR, so `blocking_file` is what lets
    /// the persistence and safety layers reach one verdict on both.
    #[test]
    fn blocking_file_finds_the_file_a_path_runs_through() {
        let dir = unique_temp_dir("blocking-file");
        let file = dir.join("a-file");
        fs::write(&file, b"x").expect("the blocking file should be writable");

        let through = file.join("nested").join("leaf.txt");
        assert_eq!(
            blocking_file(&through).as_deref(),
            Some(file.as_path()),
            "the closest non-directory ancestor is what blocks the path"
        );

        let through_dir = dir.join("sub").join("leaf.txt");
        assert_eq!(
            blocking_file(&through_dir),
            None,
            "a path through real directories is not blocked"
        );

        assert_eq!(
            blocking_file(&file),
            None,
            "a file does not block itself; only what is in front of it counts"
        );

        fs::remove_dir_all(&dir).expect("the test directory should be removable");
    }

    /// Nothing exists on the path, so nothing is in its way either. This is the
    /// "missing", not "blocked" case the persistence layer reports.
    #[test]
    fn blocking_file_ignores_a_path_that_simply_does_not_exist() {
        let dir = unique_temp_dir("blocking-file-absent");
        let nothing = dir.join("missing").join("deeper").join("leaf.txt");

        assert_eq!(blocking_file(&nothing), None);

        fs::remove_dir_all(&dir).expect("the test directory should be removable");
    }
}
