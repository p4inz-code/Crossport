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
