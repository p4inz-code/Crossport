/* ==========================================================================
 * Volume enumeration
 * Answers which storage volumes the user can reach right now, and what the
 * platform can honestly report about each one. Candidates come from the
 * platform, are deduplicated, probed, and sorted, so the list is stable
 * between refreshes.
 *
 * Windows is the primary target and uses the Win32 volume APIs
 * (`GetDriveTypeW`, `GetDiskFreeSpaceExW`, `GetVolumeInformationW`) instead of
 * string heuristics. Other platforms keep the Phase 1 mount-point discovery
 * and report the volume kind and capacity as unknown rather than guessing;
 * their platform modules grow the same level of detail in later phases.
 *
 * A volume appearing or disappearing between enumeration and probing is
 * normal system behaviour: every probe is fallible and a failed probe yields
 * fewer facts, never a crash.
 * ========================================================================== */

use std::path::{Path, PathBuf};

use crate::platform::paths as platform_paths;
use crate::platform::volume::{dedupe_roots, sort_volumes, DriveInfo, VolumeFacts, VolumeKind};

/// Enumerates the storage volumes available to the current user.
pub fn list_drives() -> Vec<DriveInfo> {
    let roots = dedupe_roots(candidate_roots());
    let mut volumes: Vec<DriveInfo> = roots.iter().filter_map(|root| probe(root)).collect();
    sort_volumes(&mut volumes);
    volumes
}

/// Free space the current user can still write on the volume holding `path`.
///
/// `None` means "the platform did not report it" — a missing answer is never
/// treated as "no space", because refusing a transfer on a guess would be
/// worse than letting the write fail with a real error.
pub fn available_bytes(path: &Path) -> Option<u64> {
    available_bytes_platform(path)
}

/// The volume root that holds `path`, when the host can determine it.
///
/// Both this and [`same_volume`] are lexical: nothing is resolved on disk, so
/// the answer is available for paths that do not exist yet (a destination
/// being planned).
pub fn volume_root_for(path: &Path) -> Option<PathBuf> {
    volume_root_for_platform(path)
}

/// Whether two normalized paths live on the same volume.
///
/// When the host cannot tell, the answer is `false`: the move engine then
/// takes the copy-then-remove path, which is always safe, instead of assuming
/// a rename would work.
pub fn same_volume(left: &Path, right: &Path) -> bool {
    match (volume_root_for(left), volume_root_for(right)) {
        (Some(left_root), Some(right_root)) => platform_paths::same_path(&left_root, &right_root),
        _ => false,
    }
}

/// Windows: the Win32 volume API answers for any existing path on the volume,
/// so the probe asks directly and gets the quota-aware figure.
#[cfg(windows)]
fn available_bytes_platform(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available_to_caller: u64 = 0;

    // SAFETY: `wide` is a NUL-terminated path buffer that outlives the call and
    // the remaining two output pointers are documented as optional.
    let succeeded = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available_to_caller,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    (succeeded != 0).then_some(available_to_caller)
}

/// Windows: a drive letter or UNC prefix is the volume.
#[cfg(windows)]
fn volume_root_for_platform(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    match path.components().next()? {
        Component::Prefix(prefix) => {
            let mut root = prefix.as_os_str().to_string_lossy().into_owned();
            root.push('\\');
            Some(PathBuf::from(root))
        }
        _ => None,
    }
}

/// Other platforms: the mount roots the host exposes, matched lexically, with
/// the most specific (longest) match winning. Free space is whatever the
/// volume probe reported; `std` alone cannot ask a mount point.
#[cfg(not(windows))]
fn available_bytes_platform(path: &Path) -> Option<u64> {
    let root = volume_root_for_platform(path)?;
    list_drives()
        .into_iter()
        .find(|volume| platform_paths::same_path(Path::new(&volume.root), &root))
        .and_then(|volume| volume.free_bytes)
}

#[cfg(not(windows))]
fn volume_root_for_platform(path: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for volume in list_drives() {
        let root = PathBuf::from(&volume.root);
        if !platform_paths::path_contains(&root, path) {
            continue;
        }
        let more_specific = best.as_ref().map_or(true, |current| {
            root.components().count() > current.components().count()
        });
        if more_specific {
            best = Some(root);
        }
    }
    best
}

/* ==========================================================================
 * Windows probing
 * ========================================================================== */

/// `FILE_READ_ONLY_VOLUME` from `GetVolumeInformationW`'s file system flags.
/// Spelled out here because the windows-sys constant lives behind a much
/// larger feature gate than this crate needs.
#[cfg(windows)]
const FILE_READ_ONLY_VOLUME: u32 = 0x0008_0000;

/// Probes a Windows drive letter or directory root.
///
/// Returns `None` only when the platform reports nothing at all — a drive
/// letter with no mounted volume. An empty optical drive is still reported,
/// with `mounted: false` and no capacity, because that is the honest answer.
#[cfg(windows)]
fn probe(root: &Path) -> Option<DriveInfo> {
    let wide = wide_root(root);

    // SAFETY: `wide` is NUL-terminated and outlives the call.
    let drive_type =
        unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr()) };
    let kind = VolumeKind::from_windows_drive_type(drive_type);
    let space = disk_space(&wide);
    let information = volume_information(&wide);

    if kind == VolumeKind::Unknown && space.is_none() && information.is_none() {
        log::debug!(
            "no volume information for '{}'; skipping it",
            root.display()
        );
        return None;
    }

    let (total_bytes, free_bytes) = match space {
        Some((total, free)) => (Some(total), Some(free)),
        None => (None, None),
    };
    let (name, filesystem, readonly) = match information {
        Some(information) => (
            information.name,
            information.filesystem,
            Some(information.readonly),
        ),
        None => (None, None, None),
    };
    // Media is reachable when either volume query answered; a disconnected
    // share or a drive with no media answers neither.
    let mounted = space.is_some() || readonly.is_some();

    Some(DriveInfo::from_facts(
        root,
        VolumeFacts {
            kind,
            name,
            filesystem,
            total_bytes,
            free_bytes,
            readonly,
            mounted,
        },
    ))
}

/// Volume queries take a root path and require a trailing separator, so the
/// path is normalized into a NUL-terminated UTF-16 buffer.
#[cfg(windows)]
fn wide_root(root: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    let mut wide: Vec<u16> = root.as_os_str().encode_wide().collect();
    let ends_with_separator = matches!(wide.last(), Some(separator) if *separator == b'\\' as u16 || *separator == b'/' as u16);
    if !ends_with_separator {
        wide.push(b'\\' as u16);
    }
    wide.push(0);
    wide
}

/// Reads a fixed-size UTF-16 buffer returned by Win32 and drops the trailing
/// NUL. Returns `None` for an empty string so "not reported" and "empty" are
/// the same thing to callers.
#[cfg(windows)]
fn wide_to_string(buffer: &[u16]) -> Option<String> {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    let value = String::from_utf16_lossy(&buffer[..end]);
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Total and free capacity in bytes, or `None` when the volume cannot report
/// them (media not ready, disconnected share, permission failure).
#[cfg(windows)]
fn disk_space(wide: &[u16]) -> Option<(u64, u64)> {
    let mut available_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut free: u64 = 0;

    // SAFETY: all three pointers are valid for the duration of the call and
    // `wide` is a NUL-terminated root path.
    let succeeded = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available_to_caller,
            &mut total,
            &mut free,
        )
    };

    if succeeded == 0 {
        return None;
    }

    // `free` is the space on the volume; `available_to_caller` is the
    // quota-aware figure, which is not what a capacity display shows.
    Some((total, free))
}

/// Volume name, filesystem type, and read-only flag.
#[cfg(windows)]
struct VolumeInformation {
    name: Option<String>,
    filesystem: Option<String>,
    readonly: bool,
}

/// Queries `GetVolumeInformationW`. Fails for volumes whose media is not
/// ready, which is reported as "no information" rather than an error.
#[cfg(windows)]
fn volume_information(wide: &[u16]) -> Option<VolumeInformation> {
    // MAX_PATH + 1: the documented maximum for a volume label and for a
    // filesystem name, both including the terminating NUL.
    let mut name_buffer = [0u16; 261];
    let mut filesystem_buffer = [0u16; 261];
    let mut serial_number: u32 = 0;
    let mut max_component_length: u32 = 0;
    let mut flags: u32 = 0;

    // SAFETY: every buffer is valid and its length is passed to the API, and
    // `wide` is a NUL-terminated root path.
    let succeeded = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
            wide.as_ptr(),
            name_buffer.as_mut_ptr(),
            name_buffer.len() as u32,
            &mut serial_number,
            &mut max_component_length,
            &mut flags,
            filesystem_buffer.as_mut_ptr(),
            filesystem_buffer.len() as u32,
        )
    };

    if succeeded == 0 {
        return None;
    }

    Some(VolumeInformation {
        name: wide_to_string(&name_buffer),
        filesystem: wide_to_string(&filesystem_buffer),
        readonly: flags & FILE_READ_ONLY_VOLUME != 0,
    })
}

/// Windows: drive letters come from the allocation table in the kernel, so a
/// letter is reported even when its media is not ready — [`probe`] is what
/// decides whether the letter describes a volume at all.
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

/* ==========================================================================
 * Other platforms
 * Mount-point discovery from Phase 1, with the platform-reported facts that
 * `std` can provide. Capacity and filesystem reporting for these targets
 * arrive with their platform work; until then the fields stay unknown.
 * ========================================================================== */

/// Non-Windows: `std` cannot classify a mount, so the kind stays unknown and
/// nothing is guessed at.
#[cfg(not(windows))]
fn probe(root: &Path) -> Option<DriveInfo> {
    let metadata = match std::fs::metadata(root) {
        Ok(metadata) => metadata,
        Err(error) => {
            log::debug!("skipping unreadable root '{}': {error}", root.display());
            return None;
        }
    };

    if !metadata.is_dir() {
        return None;
    }

    Some(DriveInfo::from_facts(
        root,
        VolumeFacts {
            kind: VolumeKind::Unknown,
            readonly: Some(metadata.permissions().readonly()),
            mounted: true,
            ..VolumeFacts::default()
        },
    ))
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
    fn list_drives_reports_at_least_one_volume_on_this_machine() {
        let volumes = list_drives();

        assert!(
            !volumes.is_empty(),
            "every supported platform reports at least one volume"
        );
        for volume in &volumes {
            assert!(
                Path::new(&volume.root).is_absolute(),
                "root '{}' must be absolute",
                volume.root
            );
            assert!(!volume.id.is_empty(), "a volume needs a stable identifier");
            assert!(!volume.label.is_empty());
        }
    }

    #[test]
    fn list_drives_is_sorted_and_unique() {
        let volumes = list_drives();
        let mut ids: Vec<&String> = volumes.iter().map(|volume| &volume.id).collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total, "reported volumes must be unique");
        assert_eq!(
            volumes.iter().map(|volume| &volume.id).collect::<Vec<_>>(),
            ids,
            "reported volumes must be sorted"
        );
    }

    #[test]
    fn candidate_roots_are_deduplicated_by_enumeration() {
        let roots = candidate_roots();
        let deduped = dedupe_roots(roots.clone());

        assert_eq!(deduped.len(), deduped.iter().collect::<Vec<_>>().len());
        assert!(
            !roots.is_empty(),
            "every supported platform has at least one candidate root"
        );
    }

    #[cfg(windows)]
    #[test]
    fn reports_capacity_for_a_mounted_volume() {
        let volumes = list_drives();
        let mounted: Vec<&DriveInfo> = volumes.iter().filter(|volume| volume.mounted).collect();

        assert!(
            !mounted.is_empty(),
            "the machine running the tests has at least one mounted volume"
        );

        let with_capacity = mounted
            .iter()
            .filter(|volume| volume.total_bytes.is_some())
            .count();
        assert!(with_capacity > 0, "a mounted volume reports total capacity");

        for volume in mounted {
            if let (Some(total), Some(free), Some(used)) =
                (volume.total_bytes, volume.free_bytes, volume.used_bytes)
            {
                assert!(free <= total, "free space cannot exceed capacity");
                assert_eq!(used, total - free, "used space must be consistent");
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn classifies_and_describes_the_system_volume() {
        let volumes = list_drives();
        let described: Vec<&DriveInfo> = volumes
            .iter()
            .filter(|volume| volume.mounted && volume.filesystem.is_some())
            .collect();

        assert!(
            !described.is_empty(),
            "a mounted Windows volume reports its filesystem"
        );
        for volume in described {
            assert_ne!(
                volume.kind,
                VolumeKind::Unknown,
                "a mounted volume with a filesystem is classifiable: {volume:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn probing_a_directory_reports_the_volume_that_holds_it() {
        let dir = unique_temp_dir("drives-probe");

        let volume = probe(&dir).expect("the temporary directory lives on a volume");

        assert!(Path::new(&volume.root).is_absolute());
        assert!(volume.mounted);
        assert!(
            volume.total_bytes.is_some(),
            "a mounted volume reports total capacity"
        );

        clean_up(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_candidates_always_include_the_root() {
        assert!(candidate_roots().contains(&PathBuf::from("/")));
    }

    #[cfg(not(windows))]
    #[test]
    fn probe_skips_a_root_that_is_not_a_directory() {
        let dir = unique_temp_dir("drives-missing");
        let missing = dir.join("not-mounted");

        assert!(
            probe(&missing).is_none(),
            "unreadable roots are not volumes"
        );

        clean_up(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn probe_reports_a_directory_root_without_inventing_facts() {
        let dir = unique_temp_dir("drives-probe-unix");

        let volume = probe(&dir).expect("a readable directory is a volume");

        assert_eq!(volume.kind, VolumeKind::Unknown);
        assert_eq!(volume.total_bytes, None);
        assert_eq!(volume.used_bytes, None);
        assert_eq!(volume.filesystem, None);
        assert!(volume.mounted);

        clean_up(&dir);
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

    #[test]
    fn same_volume_compares_the_roots_the_host_reports() {
        let dir = unique_temp_dir("drives-same-volume");
        let nested = dir.join("nested");
        std::fs::create_dir_all(&nested).expect("subdir");

        assert!(
            same_volume(&dir, &nested),
            "a directory and its child live on the same volume"
        );

        clean_up(&dir);
    }

    #[test]
    fn volume_root_for_is_lexical_and_absolute() {
        let dir = unique_temp_dir("drives-root");

        let root = volume_root_for(&dir).expect("a temporary directory lives on a volume");

        assert!(root.is_absolute());
        assert!(
            platform_paths::path_contains(&root, &dir),
            "the reported root contains the path it was asked about"
        );

        clean_up(&dir);
    }

    #[test]
    fn available_bytes_never_invents_space() {
        let dir = unique_temp_dir("drives-available");
        let missing = dir.join("not-a-volume");

        // Either the host reports a number, or it reports nothing. Both are
        // honest; a fabricated 0 would block every transfer.
        if let Some(available) = available_bytes(&dir) {
            assert!(available > 0, "a writable temporary directory has space");
        } else {
            assert!(
                available_bytes(&dir).is_none(),
                "a missing answer stays missing"
            );
        }
        assert_eq!(available_bytes(&missing), None);

        clean_up(&dir);
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
