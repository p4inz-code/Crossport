/* ==========================================================================
 * Conflict resolution
 * One reusable layer that decides what happens when a destination path is
 * already taken. Planning calls it for every item, so collision logic exists
 * in exactly one place instead of being sprinkled through the copy code.
 *
 * Resolution is always conservative:
 *
 * - an existing entry is never removed here (`Replace` marks the item, and the
 *   execution layer is the only place that deletes, at the resolved path);
 * - `Rename` never reuses a name that exists on disk or that another item in
 *   the same plan has already reserved, so two sources with the same name
 *   cannot overwrite each other;
 * - `Skip` leaves the destination byte-for-byte untouched.
 * ========================================================================== */

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};
use crate::transfer::model::{ConflictStrategy, ItemKind};

/// How many generated names are tried before the engine gives up. Reaching the
/// limit means a folder holds a thousand same-named entries, which is a real
/// problem the user should hear about rather than an invitation to overwrite.
const MAX_RENAME_ATTEMPTS: u32 = 1_000;

/// What to do with one planned item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// Write to this path. For `Replace` it may be the colliding path itself.
    Write(PathBuf),
    /// Leave the destination as it is.
    Skip,
}

/// Whether anything currently occupies `path`.
///
/// A broken symlink counts as occupied: a path that exists as a link is not
/// free space, and reporting it as free would make the next write replace it.
pub(crate) fn occupied(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// How an existing entry at `path` must be removed, when something is there.
///
/// Reparse points are classified by what the entry itself is (a directory
/// junction is removed as a directory) so a `Replace` can clear it without ever
/// touching whatever it points at.
pub(crate) fn existing_kind(path: &Path) -> Option<ItemKind> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    Some(if metadata.is_dir() {
        ItemKind::Directory
    } else {
        ItemKind::File
    })
}

/// Key used to detect collisions between two paths of the same plan.
/// Windows paths are case-insensitive, so names are folded there.
pub(crate) fn collision_key(path: &Path) -> PathBuf {
    let text = path.display().to_string();
    if cfg!(windows) {
        PathBuf::from(text.to_lowercase())
    } else {
        PathBuf::from(text)
    }
}

/// Resolves one destination path against the filesystem and the paths this
/// plan has already reserved.
///
/// `existing` is passed in when the caller has already inspected the
/// destination, so a plan does not stat the same path twice.
pub(crate) fn resolve(
    destination: &Path,
    strategy: ConflictStrategy,
    taken: &HashSet<PathBuf>,
    existing: Option<ItemKind>,
) -> AppResult<Resolution> {
    let collides = existing.is_some() || taken.contains(&collision_key(destination));
    if !collides {
        return Ok(Resolution::Write(destination.to_path_buf()));
    }

    match strategy {
        ConflictStrategy::Replace => Ok(Resolution::Write(destination.to_path_buf())),
        ConflictStrategy::Skip => Ok(Resolution::Skip),
        ConflictStrategy::Rename => Ok(Resolution::Write(unique_destination(destination, taken)?)),
    }
}

/// A non-colliding name beside `destination`.
///
/// The extension is preserved (`report.txt` → `report (2).txt`) and the parent
/// directory is never changed, so a generated name can only ever land next to
/// the path the user asked for.
pub(crate) fn unique_destination(
    destination: &Path,
    taken: &HashSet<PathBuf>,
) -> AppResult<PathBuf> {
    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    let stem = destination
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "'{}' has no name to work with",
                destination.display()
            ))
        })?;
    let extension = destination
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned());

    for attempt in 2..=MAX_RENAME_ATTEMPTS {
        let name = match &extension {
            Some(extension) => format!("{stem} ({attempt}).{extension}"),
            None => format!("{stem} ({attempt})"),
        };
        let candidate = parent.join(name);
        if !occupied(&candidate) && !taken.contains(&collision_key(&candidate)) {
            return Ok(candidate);
        }
    }

    Err(AppError::Internal(format!(
        "could not find a free name for '{}' after {MAX_RENAME_ATTEMPTS} attempts",
        destination.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    fn taken(paths: &[&Path]) -> HashSet<PathBuf> {
        paths.iter().map(|path| collision_key(path)).collect()
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_free_path_is_written_as_requested() {
        let dir = unique_temp_dir("conflict-free");
        let destination = dir.join("new.txt");

        let resolution = resolve(&destination, ConflictStrategy::Skip, &taken(&[]), None)
            .expect("a free path resolves");

        assert_eq!(resolution, Resolution::Write(destination));

        clean_up(&dir);
    }

    #[test]
    fn replace_reuses_the_colliding_path() {
        let dir = unique_temp_dir("conflict-replace");
        let destination = dir.join("existing.txt");
        std::fs::write(&destination, b"old").expect("file is writable");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Replace,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("replace resolves");

        assert_eq!(resolution, Resolution::Write(destination.clone()));
        assert_eq!(
            std::fs::read(&destination).expect("file is readable"),
            b"old",
            "resolution itself never touches the existing file"
        );

        clean_up(&dir);
    }

    #[test]
    fn skip_leaves_the_destination_alone() {
        let dir = unique_temp_dir("conflict-skip");
        let destination = dir.join("existing.txt");
        std::fs::write(&destination, b"keep me").expect("file is writable");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Skip,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("skip resolves");

        assert_eq!(resolution, Resolution::Skip);
        assert_eq!(
            std::fs::read(&destination).expect("file is readable"),
            b"keep me"
        );

        clean_up(&dir);
    }

    #[test]
    fn rename_preserves_the_extension_and_never_reuses_a_name() {
        let dir = unique_temp_dir("conflict-rename");
        let destination = dir.join("report.txt");
        std::fs::write(&destination, b"first").expect("file is writable");
        std::fs::write(dir.join("report (2).txt"), b"second").expect("file is writable");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Rename,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("rename resolves");

        assert_eq!(resolution, Resolution::Write(dir.join("report (3).txt")));

        clean_up(&dir);
    }

    #[test]
    fn rename_avoids_names_reserved_by_the_same_plan() {
        let dir = unique_temp_dir("conflict-rename-reserved");
        let destination = dir.join("report.txt");
        std::fs::write(&destination, b"first").expect("file is writable");
        let reserved = dir.join("report (2).txt");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Rename,
            &taken(&[&reserved]),
            existing_kind(&destination),
        )
        .expect("rename resolves");

        assert_eq!(resolution, Resolution::Write(dir.join("report (3).txt")));

        clean_up(&dir);
    }

    #[test]
    fn rename_handles_a_name_without_an_extension() {
        let dir = unique_temp_dir("conflict-rename-no-extension");
        let destination = dir.join("notes");
        std::fs::write(&destination, b"x").expect("file is writable");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Rename,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("rename resolves");

        assert_eq!(resolution, Resolution::Write(dir.join("notes (2)")));

        clean_up(&dir);
    }

    #[test]
    fn rename_handles_a_dotfile() {
        let dir = unique_temp_dir("conflict-rename-dotfile");
        let destination = dir.join(".gitignore");
        std::fs::write(&destination, b"x").expect("file is writable");

        let resolution = resolve(
            &destination,
            ConflictStrategy::Rename,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("rename resolves");

        assert_eq!(resolution, Resolution::Write(dir.join(".gitignore (2)")));

        clean_up(&dir);
    }

    #[test]
    fn rename_considers_a_broken_link_a_collision() {
        let dir = unique_temp_dir("conflict-broken-link");
        let destination = dir.join("link.txt");
        let created = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(dir.join("nowhere.txt"), &destination).is_ok()
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(dir.join("nowhere.txt"), &destination).is_ok()
            }
            #[cfg(not(any(unix, windows)))]
            {
                false
            }
        };

        if !created {
            clean_up(&dir);
            return;
        }

        assert!(occupied(&destination));
        let resolution = resolve(
            &destination,
            ConflictStrategy::Rename,
            &taken(&[]),
            existing_kind(&destination),
        )
        .expect("rename resolves");

        assert_eq!(resolution, Resolution::Write(dir.join("link (2).txt")));

        clean_up(&dir);
    }

    #[test]
    fn existing_kind_separates_files_from_directories() {
        let dir = unique_temp_dir("conflict-kinds");
        let file = dir.join("file.txt");
        std::fs::write(&file, b"x").expect("file is writable");
        let folder = dir.join("folder");
        std::fs::create_dir_all(&folder).expect("subdir");

        assert_eq!(existing_kind(&file), Some(ItemKind::File));
        assert_eq!(existing_kind(&folder), Some(ItemKind::Directory));
        assert_eq!(existing_kind(&dir.join("missing")), None);

        clean_up(&dir);
    }

    #[test]
    fn rename_reports_when_every_candidate_is_taken() {
        let dir = unique_temp_dir("conflict-exhausted");
        let destination = dir.join("report.txt");
        std::fs::write(&destination, b"x").expect("file is writable");
        for attempt in 2..=MAX_RENAME_ATTEMPTS {
            std::fs::write(dir.join(format!("report ({attempt}).txt")), b"x").expect("file");
        }

        let error = unique_destination(&destination, &taken(&[]))
            .expect_err("an exhausted name space is reported, not overwritten");

        assert_eq!(error.code(), "internal");

        clean_up(&dir);
    }

    #[test]
    fn collision_keys_fold_case_only_on_windows() {
        let left = collision_key(Path::new("C:\\Data\\File.txt"));
        let right = collision_key(Path::new("c:\\data\\file.txt"));

        if cfg!(windows) {
            assert_eq!(left, right, "Windows paths collide regardless of case");
        } else {
            assert_ne!(left, right, "Unix paths are case-sensitive");
        }
    }
}
