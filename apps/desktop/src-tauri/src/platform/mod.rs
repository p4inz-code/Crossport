/* ==========================================================================
 * Platform abstraction
 * The only module allowed to reason about the host operating system. It
 * derives everything from `std` constants and Tauri's public path resolver —
 * never from internal Tauri or webview details — so a future macOS/iOS/Linux
 * target only needs a new `Platform` arm, not a rewrite.
 * ========================================================================== */

pub mod drives;
pub mod paths;
pub mod volume;

pub use paths::AppPaths;
pub use volume::DriveInfo;

use serde::{Deserialize, Serialize};

/// Operating system family the backend is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
    /// Any other target; the app must stay functional, not crash.
    Other,
}

impl Platform {
    /// Resolves the current platform from the Rust target triple.
    pub fn current() -> Self {
        Self::from_os(std::env::consts::OS)
    }
    /// Maps a `std::env::consts::OS` value onto a [`Platform`].
    pub fn from_os(os: &str) -> Self {
        match os {
            "windows" => Self::Windows,
            "macos" => Self::Macos,
            "linux" => Self::Linux,
            _ => Self::Other,
        }
    }

    /// Stable machine-readable identifier, mirrored by the frontend.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Other => "other",
        }
    }
}

/// Windows `FILE_ATTRIBUTE_REPARSE_POINT`. Spelled out here because the
/// `windows-sys` constant lives behind a much larger feature gate than this
/// crate needs.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// Whether a directory entry is a symlink, junction, or another reparse point.
///
/// The transfer engine reports these entries but never follows, copies, or
/// removes what they point at. Windows hides junctions and mount points behind
/// the same reparse-point attribute, and `std::fs::FileType::is_symlink` only
/// reports true symlinks there, so the attribute is the honest check.
pub fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Raw OS error code the host reports when a write runs out of space:
/// `ERROR_DISK_FULL` on Windows, `ENOSPC` on Unix. Spelled out here instead of
/// matching on `io::ErrorKind` so the crate keeps building on its declared
/// minimum Rust version.
#[cfg(windows)]
const DISK_FULL_CODE: i32 = 112;
#[cfg(not(windows))]
const DISK_FULL_CODE: i32 = 28;

/// Whether an OS error means the destination ran out of space.
///
/// Writes that exhaust a volume are reported per platform; this is the one
/// place that knows how each host spells it.
pub fn is_disk_full_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(DISK_FULL_CODE)
}

/// Working set of this process in bytes, when the host reports it.
///
/// Used by the transfer sanity suite to prove that copying a large file does
/// not grow this process's memory. `None` means the host did not report a
/// figure, which is never treated as zero.
///
/// Compiled only for tests: the application itself never measures its own
/// memory, so the release binary carries no measurement code.
#[cfg(test)]
pub fn memory_usage_bytes() -> Option<u64> {
    memory_usage_bytes_platform()
}

/// Windows: the kernel reports the working set directly for the current
/// process, so no handle to another process is needed.
#[cfg(all(test, windows))]
fn memory_usage_bytes_platform() -> Option<u64> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: the counters struct is zero-initialized, its `cb` field is set to
    // its own size as the API requires, and the pseudo handle for the current
    // process is always valid.
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let succeeded = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb);
        if succeeded == 0 {
            return None;
        }
        Some(counters.WorkingSetSize as u64)
    }
}

#[cfg(all(test, not(windows)))]
fn memory_usage_bytes_platform() -> Option<u64> {
    None
}

/// Runtime facts about the host. Surfaced to the frontend over IPC so the UI
/// never has to guess at the environment it is rendered in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub platform: Platform,
    /// `std::env::consts::OS`, e.g. `"windows"`.
    pub os: String,
    /// `std::env::consts::ARCH`, e.g. `"x86_64"`.
    pub arch: String,
    /// `std::env::consts::FAMILY`, e.g. `"windows"` or `"unix"`.
    pub family: String,
}

impl SystemInfo {
    /// Reads the current runtime facts for the given platform.
    pub fn current(platform: Platform) -> Self {
        Self {
            platform,
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            family: std::env::consts::FAMILY.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_supported_os_name() {
        assert_eq!(Platform::from_os("windows"), Platform::Windows);
        assert_eq!(Platform::from_os("macos"), Platform::Macos);
        assert_eq!(Platform::from_os("linux"), Platform::Linux);
        assert_eq!(Platform::from_os("freebsd"), Platform::Other);
    }

    #[test]
    fn current_platform_matches_std_os_constant() {
        assert_eq!(Platform::current(), Platform::from_os(std::env::consts::OS));
    }

    #[test]
    fn platform_identifiers_are_stable() {
        assert_eq!(Platform::Windows.as_str(), "windows");
        assert_eq!(Platform::Macos.as_str(), "macos");
        assert_eq!(Platform::Linux.as_str(), "linux");
        assert_eq!(Platform::Other.as_str(), "other");
    }

    #[test]
    fn serializes_platform_lowercase() {
        let json = serde_json::to_value(Platform::Windows).expect("platform serializes");
        assert_eq!(json, serde_json::json!("windows"));
    }

    #[test]
    fn regular_files_and_directories_are_not_reparse_points() {
        let dir = crate::filesystem::test_support::unique_temp_dir("platform-reparse");
        let file = dir.join("plain.txt");
        std::fs::write(&file, b"x").expect("file is writable");

        assert!(!is_reparse_point(
            &std::fs::symlink_metadata(&dir).expect("the directory exists")
        ));
        assert!(!is_reparse_point(
            &std::fs::symlink_metadata(&file).expect("the file exists")
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_symbolic_link_is_a_reparse_point() {
        let dir = crate::filesystem::test_support::unique_temp_dir("platform-reparse-link");
        let target = dir.join("target.txt");
        std::fs::write(&target, b"x").expect("file is writable");
        let link = dir.join("link.txt");

        let created = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&target, &link).is_ok()
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(&target, &link).is_ok()
            }
            #[cfg(not(any(unix, windows)))]
            {
                false
            }
        };

        if created {
            assert!(is_reparse_point(
                &std::fs::symlink_metadata(&link).expect("the link exists")
            ));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_a_plausible_memory_figure() {
        if let Some(bytes) = memory_usage_bytes() {
            assert!(bytes > 0, "a running process occupies some memory");
            assert!(
                bytes < 1024u64 * 1024 * 1024 * 1024,
                "a working set below a terabyte is the honest range: {bytes}"
            );
            assert!(
                bytes >= 1024 * 1024,
                "a process running tests holds more than a megabyte: {bytes}"
            );
        }
    }

    #[test]
    fn classifies_disk_full_errors_per_platform() {
        let full = std::io::Error::from_raw_os_error(super::DISK_FULL_CODE);
        assert!(is_disk_full_error(&full));

        let other = std::io::Error::from_raw_os_error(5);
        assert!(!is_disk_full_error(&other));

        let kind_only = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert!(!is_disk_full_error(&kind_only));
    }

    #[test]
    fn system_info_reports_the_host() {
        let info = SystemInfo::current(Platform::current());
        assert_eq!(info.os, std::env::consts::OS);
        assert_eq!(info.arch, std::env::consts::ARCH);
        assert_eq!(info.family, std::env::consts::FAMILY);
        assert_eq!(info.platform, Platform::current());
    }

    #[test]
    fn system_info_serializes_exactly_the_frontend_contract() {
        let info = SystemInfo::current(Platform::Windows);
        let json = serde_json::to_value(&info).expect("system info serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "platform": "windows",
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "family": std::env::consts::FAMILY,
            })
        );
    }
}
