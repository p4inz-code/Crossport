/* ==========================================================================
 * Drive enumeration foundation
 * Answers one question for Phase 1: which storage roots can the user reach
 * right now? Candidates come from the platform, are filtered down to readable
 * directories, deduplicated, and sorted so the list is stable between
 * refreshes.
 *
 * Deliberately out of scope (Phase 2 drives feature): volume kinds
 * (fixed/removable/network/optical), capacity, filesystem type, and mount
 * table introspection.
 * ========================================================================== */

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// A storage root the user can browse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Absolute mount root, e.g. `C:\` or `/`.
    pub root: String,
    /// Short display label, e.g. `C:` or `/mnt/usb`.
    pub label: String,
}

impl DriveInfo {
    fn new(root: &Path) -> Self {
        Self {
            root: root.display().to_string(),
            label: label_for(root),
        }
    }
}

/// Enumerates the storage roots available to the current user.
pub fn list_drives() -> Vec<DriveInfo> {
    describe_roots(candidate_roots())
}

/// Turns raw candidate roots into the reported drive list: deduplicates,
/// drops roots that cannot be read as directories, and sorts by path.
fn describe_roots(roots: Vec<PathBuf>) -> Vec<DriveInfo> {
    let unique: BTreeSet<PathBuf> = roots.into_iter().collect();
    unique
        .into_iter()
        .filter(|root| root.is_dir())
        .map(|root| DriveInfo::new(&root))
        .collect()
}

/// Derives a short label from a root: `C:\` → `C:`, `/` → `/`,
/// `/mnt/usb/` → `/mnt/usb`.
fn label_for(root: &Path) -> String {
    let display = root.display().to_string();
    let trimmed = display.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        display
    } else {
        trimmed.to_string()
    }
}

/// Windows: drive letters come from the allocation table in the kernel, so a
/// letter is reported even when its media is not ready — the `is_dir` filter
/// in [`describe_roots`] is what drops unreadable drives.
#[cfg(windows)]
fn candidate_roots() -> Vec<PathBuf> {
    // SAFETY: `GetLogicalDrives` takes no arguments, cannot fail with memory
    // unsafety, and returns a bitmask of allocated drive letters.
    let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };

    if mask == 0 {
        // Documented failure mode: the call reports the allocation table on
        // every supported Windows version, but a failure must not produce an
        // empty UI, so fall back to probing the letters directly.
        log::warn!("GetLogicalDrives returned no drives; probing drive letters instead");
        return (b'A'..=b'Z')
            .map(letter_root)
            .filter(|root| root.is_dir())
            .collect();
    }

    (0u8..26)
        .filter(|index| mask & (1 << index) != 0)
        .map(|index| letter_root(b'A' + index))
        .collect()
}

#[cfg(windows)]
fn letter_root(letter: u8) -> PathBuf {
    PathBuf::from(format!("{}:\\", letter as char))
}

/// macOS and Linux: `/` plus the conventional mount points. Discovery is
/// best-effort by design — an unreadable mount point yields nothing instead of
/// failing the command.
#[cfg(not(windows))]
fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/")];

    // macOS volumes.
    roots.extend(scan_children(Path::new("/Volumes")));
    // Manual and temporarily attached mounts.
    roots.extend(scan_children(Path::new("/mnt")));
    // Linux removable media: either `/media/<user>/<label>` or `/media/<label>`.
    for base in ["/media", "/run/media"] {
        for mount in scan_children(Path::new(base)) {
            let nested = scan_children(&mount);
            if nested.is_empty() {
                roots.push(mount);
            } else {
                roots.extend(nested);
            }
        }
    }

    roots
}

/// Immediate subdirectories of `base`, sorted. A missing or unreadable
/// directory is not an error for discovery.
#[cfg(not(windows))]
fn scan_children(base: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(base) else {
        return Vec::new();
    };

    let mut children: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    children
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    #[test]
    fn label_keeps_a_drive_root_intact() {
        assert_eq!(label_for(Path::new("C:\\")), "C:");
    }

    #[test]
    fn label_keeps_a_filesystem_root_intact() {
        assert_eq!(label_for(Path::new("/")), "/");
    }

    #[test]
    fn label_strips_a_trailing_separator() {
        assert_eq!(label_for(Path::new("/mnt/usb/")), "/mnt/usb");
        assert_eq!(label_for(Path::new("C:\\Work\\")), "C:\\Work");
    }

    #[test]
    fn describe_roots_skips_roots_that_are_not_directories() {
        let dir = unique_temp_dir("drives-missing");
        let missing = dir.join("not-mounted");

        let drives = describe_roots(vec![missing.clone()]);

        assert!(drives.is_empty(), "unreadable roots must not be reported");
        clean_up(&dir);
    }

    #[test]
    fn describe_roots_deduplicates_and_sorts() {
        let first = unique_temp_dir("drives-sort-a");
        let second = unique_temp_dir("drives-sort-b");

        let drives = describe_roots(vec![
            second.clone(),
            first.clone(),
            first.clone(),
            second.clone(),
        ]);

        assert_eq!(drives.len(), 2, "duplicate roots must collapse");
        let roots: Vec<String> = drives.iter().map(|drive| drive.root.clone()).collect();
        let mut expected = roots.clone();
        expected.sort();
        assert_eq!(roots, expected, "the drive list must have a stable order");

        clean_up(&first);
        clean_up(&second);
    }

    #[test]
    fn describe_roots_reports_the_root_and_label() {
        let dir = unique_temp_dir("drives-label");

        let drives = describe_roots(vec![dir.clone()]);

        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].root, dir.display().to_string());
        assert!(!drives[0].label.is_empty());

        clean_up(&dir);
    }

    #[test]
    fn list_drives_finds_at_least_one_storage_root_on_this_machine() {
        let drives = list_drives();

        assert!(
            !drives.is_empty(),
            "every supported platform reports at least one root"
        );
        for drive in &drives {
            assert!(
                Path::new(&drive.root).is_absolute(),
                "root '{}' must be absolute",
                drive.root
            );
            assert!(!drive.label.is_empty());
        }
    }

    #[test]
    fn list_drives_is_sorted_and_unique() {
        let drives = list_drives();
        let mut roots: Vec<&String> = drives.iter().map(|drive| &drive.root).collect();
        let total = roots.len();
        roots.sort();
        roots.dedup();
        assert_eq!(roots.len(), total, "reported drives must be unique");
        assert_eq!(
            drives.iter().map(|drive| &drive.root).collect::<Vec<_>>(),
            roots,
            "reported drives must be sorted"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_candidates_always_include_the_root() {
        assert!(candidate_roots().contains(&PathBuf::from("/")));
    }

    #[cfg(not(windows))]
    #[test]
    fn scan_children_treats_a_missing_directory_as_empty() {
        assert!(scan_children(Path::new("/crossport-does-not-exist")).is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn scan_children_returns_sorted_subdirectories() {
        let base = unique_temp_dir("drives-scan");
        std::fs::create_dir_all(base.join("beta")).expect("subdir");
        std::fs::create_dir_all(base.join("alpha")).expect("subdir");
        std::fs::write(base.join("file.txt"), b"not a directory").expect("file");

        let children = scan_children(&base);

        assert_eq!(
            children,
            vec![base.join("alpha"), base.join("beta")],
            "only directories are listed, in sorted order"
        );

        clean_up(&base);
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
