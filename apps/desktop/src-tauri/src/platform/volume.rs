/* ==========================================================================
 * Volume model
 * The CrossPort representation of a storage volume. Everything a platform can
 * report reliably lives here as a typed field; anything a platform cannot
 * report stays `None` instead of being guessed at.
 *
 * The helpers in this module are pure so they are testable on every target,
 * including the Windows drive-type classification, which takes the raw code
 * `GetDriveTypeW` returns and maps it without touching the OS.
 * ========================================================================== */

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// What kind of storage a volume is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeKind {
    /// Internal disk.
    Fixed,
    /// USB stick, memory card, external drive.
    Removable,
    /// Mapped or remote share.
    Network,
    /// CD/DVD/Blu-ray drive.
    Optical,
    /// RAM disk.
    Ram,
    /// The platform cannot classify the volume reliably.
    #[default]
    Unknown,
}

impl VolumeKind {
    /// Maps the value returned by the Win32 `GetDriveType` function onto a
    /// [`VolumeKind`].
    ///
    /// The codes are the documented `DRIVE_*` constants; they are spelled out
    /// here instead of pulling in the `DRIVE_*` constants from windows-sys,
    /// which live behind a much larger feature than this crate needs.
    pub fn from_windows_drive_type(code: u32) -> Self {
        match code {
            2 => Self::Removable,
            3 => Self::Fixed,
            4 => Self::Network,
            5 => Self::Optical,
            6 => Self::Ram,
            // 0 (DRIVE_UNKNOWN), 1 (DRIVE_NO_ROOT_DIR), and anything the
            // platform adds later: report "unknown" rather than guessing.
            _ => Self::Unknown,
        }
    }
}

/// Volume facts as reported by a platform probe.
///
/// Every field is optional because no platform answers all of them for every
/// volume: an empty optical drive has no capacity and no filesystem, and a
/// volume can disappear between enumeration and probing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VolumeFacts {
    pub kind: VolumeKind,
    /// Volume name (Windows volume label), when the OS reports a non-empty one.
    pub name: Option<String>,
    pub filesystem: Option<String>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    /// Read-only when the platform reports it (Windows volume flags).
    pub readonly: Option<bool>,
    /// Whether the volume is mounted and its media is reachable right now.
    pub mounted: bool,
}

/// A storage volume the user can browse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Stable identifier derived from the root (`C:`, `/mnt/usb`). The
    /// frontend keys its list off this value.
    pub id: String,
    /// Absolute mount root, e.g. `C:\` or `/`.
    pub root: String,
    /// Best available display label: the volume name when the OS reports one,
    /// otherwise the root label (`C:`).
    pub label: String,
    /// Raw volume name, or `None` when the volume is unnamed or the platform
    /// cannot report one.
    pub name: Option<String>,
    pub kind: VolumeKind,
    /// Filesystem type (`NTFS`, `APFS`, …) when the platform reports one.
    pub filesystem: Option<String>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    /// Derived from total and free space; `None` whenever either is missing.
    pub used_bytes: Option<u64>,
    pub readonly: Option<bool>,
    /// False when the root exists as a drive letter but its media is not
    /// reachable (an empty optical drive, a disconnected volume).
    pub mounted: bool,
}

impl DriveInfo {
    /// Builds the reported volume from a root path and the facts a probe
    /// collected. Missing facts stay missing.
    pub fn from_facts(root: &Path, facts: VolumeFacts) -> Self {
        let id = root_label(root);
        Self {
            id: id.clone(),
            root: root.display().to_string(),
            label: display_label(facts.name.as_deref(), &id),
            name: facts.name,
            kind: facts.kind,
            filesystem: normalize_filesystem(facts.filesystem),
            total_bytes: facts.total_bytes,
            free_bytes: facts.free_bytes,
            used_bytes: derive_used_bytes(facts.total_bytes, facts.free_bytes),
            readonly: facts.readonly,
            mounted: facts.mounted,
        }
    }
}

/// `C:\` → `C:`, `/mnt/usb/` → `/mnt/usb`, `/` → `/`.
///
/// Filesystem roots have no final path component, so the root itself is the
/// label in that case.
pub fn root_label(root: &Path) -> String {
    let display = root.display().to_string();
    let trimmed = display.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        display
    } else {
        trimmed.to_string()
    }
}

/// Picks the label to display: the volume name when the OS reports one, the
/// root label otherwise.
pub fn display_label(name: Option<&str>, root_label: &str) -> String {
    match name.map(str::trim) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => root_label.to_string(),
    }
}

/// Trims a reported filesystem type and drops an empty one.
pub fn normalize_filesystem(filesystem: Option<String>) -> Option<String> {
    filesystem
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Used space is only reported when the platform gave both numbers and they
/// are consistent. `free > total` means the values came from different
/// sources or the volume changed shape mid-probe, so the honest answer is
/// "unknown" instead of a negative or wrapped number.
pub fn derive_used_bytes(total_bytes: Option<u64>, free_bytes: Option<u64>) -> Option<u64> {
    match (total_bytes, free_bytes) {
        (Some(total), Some(free)) if free <= total => Some(total - free),
        _ => None,
    }
}

/// Collapses duplicates and orders roots so probing is deterministic.
pub fn dedupe_roots(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots
        .into_iter()
        .collect::<BTreeSet<PathBuf>>()
        .into_iter()
        .collect()
}

/// Deterministic volume order: by identifier, case-insensitively first so
/// `c:` and `C:` cannot swap places between refreshes.
pub fn sort_volumes(volumes: &mut [DriveInfo]) {
    volumes.sort_by(|left, right| {
        left.id
            .to_ascii_lowercase()
            .cmp(&right.id.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(id: &str) -> DriveInfo {
        DriveInfo::from_facts(
            Path::new(id),
            VolumeFacts {
                kind: VolumeKind::Fixed,
                name: None,
                filesystem: Some("NTFS".to_string()),
                total_bytes: Some(500),
                free_bytes: Some(200),
                readonly: Some(false),
                mounted: true,
            },
        )
    }

    #[test]
    fn classifies_every_documented_windows_drive_type() {
        assert_eq!(
            VolumeKind::from_windows_drive_type(2),
            VolumeKind::Removable
        );
        assert_eq!(VolumeKind::from_windows_drive_type(3), VolumeKind::Fixed);
        assert_eq!(VolumeKind::from_windows_drive_type(4), VolumeKind::Network);
        assert_eq!(VolumeKind::from_windows_drive_type(5), VolumeKind::Optical);
        assert_eq!(VolumeKind::from_windows_drive_type(6), VolumeKind::Ram);
    }

    #[test]
    fn reports_unknown_for_unclassifiable_drive_types() {
        for code in [0, 1, 7, 99, u32::MAX] {
            assert_eq!(
                VolumeKind::from_windows_drive_type(code),
                VolumeKind::Unknown,
                "code {code} must not be guessed at"
            );
        }
    }

    #[test]
    fn kind_identifiers_are_stable_on_the_wire() {
        let cases = [
            (VolumeKind::Fixed, "fixed"),
            (VolumeKind::Removable, "removable"),
            (VolumeKind::Network, "network"),
            (VolumeKind::Optical, "optical"),
            (VolumeKind::Ram, "ram"),
            (VolumeKind::Unknown, "unknown"),
        ];

        for (kind, identifier) in cases {
            let json = serde_json::to_value(kind).expect("kind serializes");
            assert_eq!(json, serde_json::json!(identifier));
        }
    }

    #[test]
    fn derives_used_space_from_capacity_and_free_space() {
        assert_eq!(derive_used_bytes(Some(500), Some(200)), Some(300));
        assert_eq!(derive_used_bytes(Some(0), Some(0)), Some(0));
    }

    #[test]
    fn derives_used_space_never_underflows() {
        // u64::MAX - 1 must not panic or wrap.
        assert_eq!(
            derive_used_bytes(Some(u64::MAX), Some(1)),
            Some(u64::MAX - 1)
        );
    }

    #[test]
    fn refuses_to_derive_used_space_from_inconsistent_values() {
        assert_eq!(derive_used_bytes(Some(100), Some(200)), None);
        assert_eq!(derive_used_bytes(Some(100), None), None);
        assert_eq!(derive_used_bytes(None, Some(100)), None);
        assert_eq!(derive_used_bytes(None, None), None);
    }

    #[test]
    fn root_labels_keep_roots_and_trim_separators() {
        assert_eq!(root_label(Path::new("C:\\")), "C:");
        assert_eq!(root_label(Path::new("/")), "/");
        assert_eq!(root_label(Path::new("/mnt/usb/")), "/mnt/usb");
        if cfg!(windows) {
            assert_eq!(root_label(Path::new("C:\\Work\\")), "C:\\Work");
        }
    }

    #[test]
    fn display_label_prefers_the_volume_name() {
        assert_eq!(display_label(Some("Data"), "E:"), "Data");
        assert_eq!(display_label(Some("  Data  "), "E:"), "Data");
    }

    #[test]
    fn display_label_falls_back_to_the_root_label() {
        assert_eq!(display_label(None, "E:"), "E:");
        assert_eq!(display_label(Some("   "), "E:"), "E:");
        assert_eq!(display_label(Some(""), "/"), "/");
    }

    #[test]
    fn filesystem_names_are_trimmed_and_empty_ones_dropped() {
        assert_eq!(
            normalize_filesystem(Some("  NTFS ".to_string())),
            Some("NTFS".to_string())
        );
        assert_eq!(normalize_filesystem(Some("".to_string())), None);
        assert_eq!(normalize_filesystem(None), None);
    }

    #[test]
    fn builds_a_volume_from_root_and_facts() {
        let volume = DriveInfo::from_facts(
            Path::new("E:\\"),
            VolumeFacts {
                kind: VolumeKind::Removable,
                name: Some("USB DISK".to_string()),
                filesystem: Some("exFAT".to_string()),
                total_bytes: Some(64_000),
                free_bytes: Some(1_000),
                readonly: Some(false),
                mounted: true,
            },
        );

        assert_eq!(volume.id, "E:");
        assert_eq!(volume.label, "USB DISK");
        assert_eq!(volume.name.as_deref(), Some("USB DISK"));
        assert_eq!(volume.kind, VolumeKind::Removable);
        assert_eq!(volume.filesystem.as_deref(), Some("exFAT"));
        assert_eq!(volume.used_bytes, Some(63_000));
        assert!(volume.mounted);
    }

    #[test]
    fn builds_an_unnamed_unmounted_volume_without_inventing_facts() {
        let volume = DriveInfo::from_facts(
            Path::new("D:\\"),
            VolumeFacts {
                kind: VolumeKind::Optical,
                ..VolumeFacts::default()
            },
        );

        assert_eq!(volume.id, "D:");
        assert_eq!(volume.label, "D:");
        assert_eq!(volume.name, None);
        assert_eq!(volume.filesystem, None);
        assert_eq!(volume.total_bytes, None);
        assert_eq!(volume.free_bytes, None);
        assert_eq!(volume.used_bytes, None);
        assert_eq!(volume.readonly, None);
        assert!(!volume.mounted);
    }

    #[test]
    fn serializes_exactly_the_frontend_contract() {
        let volume = DriveInfo::from_facts(
            Path::new("C:\\"),
            VolumeFacts {
                kind: VolumeKind::Fixed,
                name: Some("Windows".to_string()),
                filesystem: Some("NTFS".to_string()),
                total_bytes: Some(500),
                free_bytes: Some(200),
                readonly: Some(false),
                mounted: true,
            },
        );

        let json = serde_json::to_value(&volume).expect("volume serializes");

        assert_eq!(
            json,
            serde_json::json!({
                "id": "C:",
                "root": "C:\\",
                "label": "Windows",
                "name": "Windows",
                "kind": "fixed",
                "filesystem": "NTFS",
                "totalBytes": 500,
                "freeBytes": 200,
                "usedBytes": 300,
                "readonly": false,
                "mounted": true,
            })
        );
        assert!(
            json.get("total_bytes").is_none(),
            "the IPC contract is camelCase"
        );
    }

    #[test]
    fn sorts_volumes_by_identifier() {
        let mut volumes = vec![volume("D:"), volume("c:"), volume("A:"), volume("C:")];

        sort_volumes(&mut volumes);

        let ids: Vec<&str> = volumes.iter().map(|volume| volume.id.as_str()).collect();
        assert_eq!(ids, vec!["A:", "C:", "c:", "D:"]);
    }

    #[test]
    fn deduplicates_roots_and_returns_them_sorted() {
        let roots = vec![
            PathBuf::from("/mnt/b"),
            PathBuf::from("/mnt/a"),
            PathBuf::from("/mnt/b"),
        ];

        assert_eq!(
            dedupe_roots(roots),
            vec![PathBuf::from("/mnt/a"), PathBuf::from("/mnt/b")]
        );
    }
}
