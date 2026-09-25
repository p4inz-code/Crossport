/* ==========================================================================
 * Path validation foundation
 * Turns strings coming from the frontend into clean, absolute paths and
 * validates that a path is a usable directory before any future transfer code
 * touches it.
 *
 * The rules are enforced by unit tests and are intentionally strict: a path
 * that leaves its own root is rejected rather than silently clamped.
 * ========================================================================== */

use std::path::{Component, Path, PathBuf};

use crate::errors::{AppError, AppResult};

/// Normalizes a user-supplied path into an absolute, lexically clean path.
///
/// - must not be empty or whitespace-only
/// - must not contain NUL bytes (they truncate paths inside the OS)
/// - must be absolute
/// - `.` components and redundant separators are removed
/// - `..` is resolved lexically and must never escape the root
///
/// No filesystem access happens here, so the result is also valid for paths
/// that do not exist yet (a future transfer destination, for example).
pub fn normalize(input: &str) -> AppResult<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("path must not be empty".to_string()));
    }
    if trimmed.contains('\0') {
        return Err(AppError::InvalidInput(
            "path must not contain null bytes".to_string(),
        ));
    }

    let candidate = Path::new(trimmed);
    if !candidate.is_absolute() {
        return Err(AppError::InvalidInput(format!(
            "path must be absolute, got '{trimmed}'"
        )));
    }

    lexical_clean(candidate)
}

/// Asserts that a resolved path is an existing directory.
///
/// Used by the native folder picker, which receives its path from the OS
/// rather than from a string, and by future transfer sources and
/// destinations.
pub fn must_be_directory(path: PathBuf) -> AppResult<PathBuf> {
    let metadata = std::fs::metadata(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::PathNotFound(path.display().to_string())
        } else {
            AppError::from(error)
        }
    })?;

    if !metadata.is_dir() {
        return Err(AppError::PathNotDirectory(path.display().to_string()));
    }

    Ok(path)
}

/// Resolves `.` and `..` without touching the filesystem. `..` that would
/// climb above the root is an error: it is almost always a traversal attempt,
/// never a typo worth guessing at.
fn lexical_clean(path: &Path) -> AppResult<PathBuf> {
    let mut cleaned = PathBuf::new();
    let mut depth = 0usize;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => cleaned.push(prefix.as_os_str()),
            Component::RootDir => cleaned.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if depth == 0 {
                    return Err(AppError::InvalidInput(format!(
                        "path escapes its root: {}",
                        path.display()
                    )));
                }
                cleaned.pop();
                depth -= 1;
            }
            Component::Normal(part) => {
                cleaned.push(part);
                depth += 1;
            }
        }
    }

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;
    use std::path::MAIN_SEPARATOR;

    #[test]
    fn rejects_an_empty_path() {
        let error = normalize("   ").expect_err("blank paths are invalid");
        assert_eq!(error.code(), "invalid_input");
        assert!(error.to_string().contains("must not be empty"));
    }

    #[test]
    fn rejects_null_bytes() {
        let error = normalize("C:\\temp\\\0evil").expect_err("null bytes are invalid");
        assert_eq!(error.code(), "invalid_input");
        assert!(error.to_string().contains("null bytes"));
    }

    #[test]
    fn rejects_relative_paths() {
        for input in ["foo", "foo/bar", "./foo", "../foo", "~"] {
            let error = normalize(input).expect_err("relative paths are invalid");
            assert_eq!(error.code(), "invalid_input", "input: {input}");
            assert!(error.to_string().contains("must be absolute"));
        }
    }

    #[test]
    fn accepts_a_root_path() {
        let root = if cfg!(windows) { "C:\\" } else { "/" };
        let normalized = normalize(root).expect("a root path is absolute");
        assert!(normalized.is_absolute());
        assert_eq!(normalized.parent(), None);
    }

    #[test]
    fn collapses_current_directory_and_trailing_separators() {
        let dir = unique_temp_dir("path-clean");
        let input = format!("{}{sep}.{sep}sub{sep}", dir.display(), sep = MAIN_SEPARATOR);

        let normalized = normalize(&input).expect("the path is absolute");

        assert_eq!(normalized, dir.join("sub"));
        clean_up(&dir);
    }

    #[test]
    fn resolves_parent_components_lexically() {
        let dir = unique_temp_dir("path-parent");
        let input = format!(
            "{}{sep}sub{sep}..{sep}other",
            dir.display(),
            sep = MAIN_SEPARATOR
        );

        let normalized = normalize(&input).expect("the path does not escape its root");

        assert_eq!(normalized, dir.join("other"));
        clean_up(&dir);
    }

    #[test]
    fn rejects_parent_traversal_above_the_root() {
        let escaping = if cfg!(windows) { "C:\\..\\.." } else { "/.." };
        let error = normalize(escaping).expect_err("escaping the root must fail");
        assert_eq!(error.code(), "invalid_input");
        assert!(error.to_string().contains("escapes its root"));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let dir = unique_temp_dir("path-trim");
        let padded = format!("   {}   ", dir.display());

        let normalized = normalize(&padded).expect("the path is absolute");

        assert_eq!(normalized, dir);
        clean_up(&dir);
    }
    #[test]
    fn must_be_directory_accepts_an_existing_directory() {
        let dir = unique_temp_dir("path-validate-ok");

        let validated = must_be_directory(dir.clone()).expect("directory exists");

        assert_eq!(validated, dir);
        clean_up(&dir);
    }

    #[test]
    fn must_be_directory_reports_a_missing_path() {
        let dir = unique_temp_dir("path-validate-missing");
        let missing = dir.join("gone");

        let error = must_be_directory(missing).expect_err("path does not exist");

        assert_eq!(error.code(), "path_not_found");
        clean_up(&dir);
    }

    #[test]
    fn must_be_directory_rejects_a_file() {
        let dir = unique_temp_dir("path-validate-file");
        let file = dir.join("notes.txt");
        std::fs::write(&file, b"hello").expect("file should be writable");

        let error = must_be_directory(file).expect_err("files are not directories");

        assert_eq!(error.code(), "path_not_directory");
        clean_up(&dir);
    }

    #[test]
    fn normalization_and_directory_validation_compose() {
        let dir = unique_temp_dir("path-validate-compose");
        let input = format!("{}{sep}.{sep}", dir.display(), sep = MAIN_SEPARATOR);

        let validated =
            must_be_directory(normalize(&input).expect("absolute path")).expect("directory exists");

        assert_eq!(validated, dir);
        clean_up(&dir);
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
