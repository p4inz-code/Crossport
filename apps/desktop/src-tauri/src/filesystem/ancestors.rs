/* ==========================================================================
 * Path ancestry
 * The breadcrumb trail for a directory: the directory itself and every
 * directory above it, oldest first.
 *
 * This exists because the frontend never assembles a path. A breadcrumb is a
 * navigation control, so every step it offers has to be a path the backend
 * produced — there is no way to show `C:\Users\Ada\Projects` as three steps
 * without the backend saying what those three steps are.
 *
 * Nothing here mutates the filesystem: the requested path is normalized and
 * validated as a directory, and each step is the lexical parent of the one
 * below it.
 * ========================================================================== */

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::errors::{AppError, AppResult};
use crate::filesystem::display_name;
use crate::filesystem::path::{must_be_directory, normalize};

/// One step of a breadcrumb trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathAncestor {
    /// Absolute path of this step, as Rust resolved it.
    pub path: String,
    /// What to show for it: the directory's own name, or a root's own form
    /// (`C:\`, `/`) rather than an empty label.
    pub label: String,
}

/// How many steps a trail may have.
///
/// A real path is never this deep. The bound exists so a pathological input
/// cannot make the walk unbounded, and reaching it is an error rather than a
/// silently truncated trail.
const MAX_TRAIL_STEPS: usize = 256;

/// Builds the breadcrumb trail for `path`, oldest step first.
///
/// The last step is always the requested directory, so a caller can treat it
/// as the current location without re-deriving it.
pub fn trail_for(path: &str) -> AppResult<Vec<PathAncestor>> {
    let directory = must_be_directory(normalize(path)?)?;

    let mut steps: Vec<PathBuf> = Vec::new();
    let mut cursor: Option<&Path> = Some(directory.as_path());
    while let Some(current) = cursor {
        if steps.len() >= MAX_TRAIL_STEPS {
            return Err(AppError::InvalidInput(format!(
                "'{}' has more path components than this build will describe",
                directory.display()
            )));
        }
        steps.push(current.to_path_buf());
        cursor = current.parent();
    }
    steps.reverse();

    Ok(steps
        .into_iter()
        .map(|step| PathAncestor {
            label: display_name(&step),
            path: step.display().to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    #[test]
    fn a_trail_ends_at_the_requested_directory_and_starts_at_a_root() {
        let workspace = unique_temp_dir("ancestors-order");
        let nested = workspace.join("one").join("two");
        std::fs::create_dir_all(&nested).expect("directories are creatable");

        let trail = trail_for(&nested.display().to_string()).expect("the trail is built");

        let last = trail.last().expect("the trail is not empty");
        assert_eq!(last.path, nested.display().to_string());
        assert_eq!(last.label, "two");

        let first = trail.first().expect("the trail has a first step");
        assert_eq!(
            Path::new(&first.path).parent(),
            None,
            "the trail starts at a filesystem root, got '{}'",
            first.path
        );
        assert!(
            first.path.ends_with(':') || first.path.ends_with(['\\', '/']),
            "a root keeps its own form as a label: '{first:?}'",
        );

        // Every step is real, and each one is the parent of the next.
        for pair in trail.windows(2) {
            assert!(
                Path::new(&pair[0].path).is_dir(),
                "'{}' is a directory",
                pair[0].path
            );
            assert_eq!(
                Path::new(&pair[1].path)
                    .parent()
                    .map(|parent| parent.display().to_string()),
                Some(pair[0].path.clone()),
                "'{}' is the parent of '{}'",
                pair[0].path,
                pair[1].path
            );
        }

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn a_normalized_trail_ignores_lexical_noise() {
        let workspace = unique_temp_dir("ancestors-normalize");
        let nested = workspace.join("inner");
        std::fs::create_dir_all(&nested).expect("directories are creatable");

        let noisy = format!("{}/inner/../inner/.", workspace.display());
        let trail = trail_for(&noisy).expect("the trail is built");

        assert_eq!(
            trail.last().expect("a last step").path,
            nested.display().to_string(),
            "the trail reports where the path really is"
        );

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn a_missing_directory_is_reported_rather_than_walked() {
        let workspace = unique_temp_dir("ancestors-missing");
        let missing = workspace.join("not-there").join("deeper");

        let error =
            trail_for(&missing.display().to_string()).expect_err("a missing path has no trail");
        assert_eq!(error.code(), "path_not_found");

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn a_file_has_no_breadcrumb_trail() {
        let workspace = unique_temp_dir("ancestors-file");
        let file = workspace.join("notes.txt");
        std::fs::write(&file, b"hello").expect("the file is writable");

        let error = trail_for(&file.display().to_string()).expect_err("a file is not a directory");
        assert_eq!(error.code(), "path_not_directory");

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn an_escaping_path_is_rejected_before_anything_is_read() {
        assert!(trail_for("../").is_err());
        assert!(trail_for("relative/path").is_err());
        assert!(trail_for("").is_err());
    }
}
