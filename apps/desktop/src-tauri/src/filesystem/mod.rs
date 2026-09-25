/* ==========================================================================
 * Filesystem foundation
 * The safe primitives the transfer engine will be built on: absolute path
 * normalization, directory validation, and metadata inspection. Nothing here
 * mutates the filesystem — Phase 1 reads and validates only.
 * ========================================================================== */

pub mod metadata;
pub mod path;

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
