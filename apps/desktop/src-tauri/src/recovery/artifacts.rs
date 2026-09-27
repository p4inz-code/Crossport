/* ==========================================================================
 * Partial artifacts
 * A transfer writes every file to `.crossport-<job>-<index>.partial` beside its
 * destination and renames it into place only when it is complete. When the
 * application dies mid-transfer those temporary files stay behind, and they are
 * the only thing on disk that identifies unfinished work.
 *
 * Finding them needs no persisted bookkeeping: the name embeds the job
 * identifier, so a walk of the destination tree finds exactly the files a given
 * job left. Deleting them is bounded to that same test — a file is only ever
 * removed when its name is a partial artifact of the job being cleaned up, so
 * user data can never be caught by a cleanup.
 * ========================================================================== */

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};

/// Prefix every temporary transfer file starts with.
pub const PARTIAL_PREFIX: &str = ".crossport-";

/// Suffix every temporary transfer file ends with.
pub const PARTIAL_SUFFIX: &str = ".partial";

/// Most directory entries a scan will visit before giving up.
///
/// The scan only runs when an interrupted job exists, but it walks a whole
/// destination tree, so it is bounded: reaching the bound is reported rather
/// than silently returning a partial answer.
pub const SCAN_BUDGET: usize = 200_000;

/// A temporary file one job left behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialArtifact {
    pub path: String,
    /// Size on disk, which is how much of that file had been written.
    pub bytes: u64,
}

/// The result of scanning a destination tree for one job's artifacts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactScan {
    pub artifacts: Vec<PartialArtifact>,
    /// True when the budget ran out: there may be more artifacts than listed.
    pub truncated: bool,
    /// Directories that could not be read, reported so a truncated scan is not
    /// mistaken for a complete one.
    pub unreadable: Vec<String>,
}

impl ArtifactScan {
    pub fn bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .fold(0u64, |total, artifact| total.saturating_add(artifact.bytes))
    }
}

/// Whether `name` is a partial artifact belonging to `job_id`.
///
/// The test is exact on both sides: the file must be named
/// `.crossport-<job_id>-<digits>.partial`. A file called `.crossport-notes.txt`
/// or one belonging to another job never matches.
pub fn is_artifact_of(name: &str, job_id: &str) -> bool {
    let Some(rest) = name.strip_prefix(PARTIAL_PREFIX) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(job_id) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix('-') else {
        return false;
    };
    let Some(index) = rest.strip_suffix(PARTIAL_SUFFIX) else {
        return false;
    };
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
}

/// Scans `root` recursively for the partial artifacts of `job_id`.
///
/// Directories are walked without following links, unreadable directories are
/// reported instead of failing the scan, and the visit budget bounds the work.
pub fn scan(root: &Path, job_id: &str) -> ArtifactScan {
    let mut scan = ArtifactScan::default();
    let mut visited = 0usize;
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                scan.unreadable
                    .push(format!("{}: {error}", directory.display()));
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    scan.unreadable
                        .push(format!("{}: {error}", directory.display()));
                    continue;
                }
            };

            visited += 1;
            if visited > SCAN_BUDGET {
                scan.truncated = true;
                return scan;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    scan.unreadable.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };

            // A reparse point is never descended and never treated as an
            // artifact, so a link cannot lead this scan out of the tree.
            if metadata.file_type().is_symlink() {
                continue;
            }

            if metadata.is_dir() {
                pending.push(path);
                continue;
            }

            if is_artifact_of(&name, job_id) {
                scan.artifacts.push(PartialArtifact {
                    path: path.display().to_string(),
                    bytes: metadata.len(),
                });
            }
        }
    }

    scan.artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    scan
}

/// Deletes one job's partial artifacts.
///
/// Returns the paths that were removed. A file that vanished between the scan
/// and the removal is treated as already cleaned up, not as a failure. Nothing
/// outside the artifact names is ever touched.
pub fn discard(artifacts: &[PartialArtifact]) -> AppResult<Vec<PathBuf>> {
    let mut removed = Vec::with_capacity(artifacts.len());
    let mut failures = Vec::new();

    for artifact in artifacts {
        let path = PathBuf::from(&artifact.path);
        match std::fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                removed.push(path);
            }
            Err(error) => failures.push(format!("{}: {error}", artifact.path)),
        }
    }

    if !failures.is_empty() {
        return Err(AppError::RecoveryUnavailable(format!(
            "{} partial file(s) could not be removed: {}",
            failures.len(),
            failures.join("; ")
        )));
    }

    Ok(removed)
}

/// The directories a scan found artifacts in, deepest first.
///
/// Used to report where unfinished work was left, so the user can see it
/// without reading the file list.
pub fn artifact_directories(artifacts: &[PartialArtifact]) -> Vec<String> {
    let mut directories: Vec<String> = artifacts
        .iter()
        .filter_map(|artifact| {
            Path::new(&artifact.path)
                .parent()
                .map(|parent| parent.display().to_string())
        })
        .collect();
    directories.sort();
    directories.dedup();
    directories.reverse();
    directories
}
