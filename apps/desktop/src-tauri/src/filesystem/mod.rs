/* ==========================================================================
 * Filesystem foundation
 * The safe primitives the transfer engine will be built on: absolute path
 * normalization, directory validation, metadata inspection, and directory
 * listing for the browser surface. Nothing here mutates the filesystem —
 * CrossPort reads and validates only.
 * ========================================================================== */

pub mod directory;
pub mod metadata;
pub mod path;

use std::path::Path;

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
