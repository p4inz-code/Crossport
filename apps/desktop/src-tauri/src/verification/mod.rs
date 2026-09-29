/* ==========================================================================
 * Verification domain
 * Answers the question a transfer alone cannot: did the destination actually
 * receive what the source held?
 *
 * Three policies exist, and every one of them is recorded rather than implied:
 *
 * - `None`    — nothing was checked. Reported as `skipped`, never as success.
 * - `Size`    — the default. Every copied file must exist as a file and be
 *               exactly the size the plan measured.
 * - `Checksum`— the size checks plus SHA-256 of the bytes that were streamed
 *               out of the source compared against SHA-256 of the file that
 *               ended up on disk.
 *
 * Verification is inline: an item is only reported complete once its own
 * verification passed, so a job can never report `completed` while its data is
 * still unverified. A mismatched or unverifiable item fails the item, which
 * fails the job — a transfer that did not verify is never presented as a
 * transfer that worked.
 *
 * Coverage is stated explicitly in every summary: size, structure, and checksum
 * are claimed only when they ran, and the metadata this engine deliberately
 * does not preserve (modified time, read-only attribute) is reported as not
 * preserved instead of being implied.
 * ========================================================================== */

pub mod hash;

#[cfg(test)]
mod tests;

pub use hash::{hash_file, StreamHasher, HASH_BUFFER_BYTES, SHA256_PREFIX};

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult, StoredError};
use crate::transfer::safety;

/// Checksum algorithms CrossPort can compute.
///
/// One variant today. The type exists because the wire contract carries the
/// algorithm name alongside every digest, so adding a second algorithm cannot
/// change the meaning of a stored digest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumAlgorithm {
    #[default]
    Sha256,
}

impl ChecksumAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }

    /// The prefix a digest of this algorithm carries, e.g. `sha256:`.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Sha256 => SHA256_PREFIX,
        }
    }
}

/// How thoroughly a transfer verifies what it wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationPolicy {
    /// No verification. Only ever chosen explicitly, and always reported as
    /// `skipped` so nobody reads it as success.
    None,
    /// Every copied file must exist and match the size the plan measured.
    Size,
    /// The size checks plus a SHA-256 comparison of streamed source bytes
    /// against the committed destination file.
    Checksum,
}

impl Default for VerificationPolicy {
    /// Size verification: cheap, always meaningful, and it catches the
    /// failure modes a disk actually produces (truncated writes, a volume that
    /// filled up, a destination that vanished).
    fn default() -> Self {
        Self::Size
    }
}

impl VerificationPolicy {
    /// Stable identifier, used by settings and by every error message.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Size => "size",
            Self::Checksum => "checksum",
        }
    }

    /// Parses a setting value. Unknown values are rejected rather than
    /// silently downgraded to something weaker.
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "none" => Ok(Self::None),
            "size" => Ok(Self::Size),
            "checksum" => Ok(Self::Checksum),
            other => Err(AppError::InvalidInput(format!(
                "verification must be one of [none, size, checksum], got '{other}'"
            ))),
        }
    }

    /// What this policy checks, for display and for the summary.
    pub fn method(self) -> VerificationMethod {
        match self {
            Self::None => VerificationMethod::None,
            Self::Size => VerificationMethod::Size,
            Self::Checksum => VerificationMethod::SizeAndChecksum,
        }
    }

    /// The algorithm this policy computes, if any.
    pub fn algorithm(self) -> Option<ChecksumAlgorithm> {
        match self {
            Self::Checksum => Some(ChecksumAlgorithm::Sha256),
            Self::None | Self::Size => None,
        }
    }
}

/// What actually ran for an item or a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    None,
    Size,
    SizeAndChecksum,
}

impl VerificationMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Size => "size",
            Self::SizeAndChecksum => "size_and_checksum",
        }
    }
}

/// Lifecycle of a verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Planned but not started.
    Pending,
    /// Work is in progress and nothing can be claimed yet.
    Verifying,
    /// Everything checked matched.
    Verified,
    /// Something did not match. This is a real, provable discrepancy.
    Mismatch,
    /// Verification could not be carried out. Never treated as success.
    Failed,
    /// Nothing was checked, because the policy said so or because there was
    /// nothing to check.
    Skipped,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Verifying => "verifying",
            Self::Verified => "verified",
            Self::Mismatch => "mismatch",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    /// Whether this state means the data was proven to have arrived.
    pub fn is_proven(self) -> bool {
        self == Self::Verified
    }

    /// Whether this state must fail the item it belongs to.
    pub fn is_failure(self) -> bool {
        matches!(self, Self::Mismatch | Self::Failed)
    }
}

/// Why a verification did not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMismatchReason {
    /// The destination is a different size than the source was measured to be.
    SizeMismatch,
    /// The destination's digest differs from the source's.
    ChecksumMismatch,
    /// The destination is not there at all.
    DestinationMissing,
    /// Something exists at the destination, but it is not a file.
    DestinationNotAFile,
}

impl VerificationMismatchReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SizeMismatch => "size_mismatch",
            Self::ChecksumMismatch => "checksum_mismatch",
            Self::DestinationMissing => "destination_missing",
            Self::DestinationNotAFile => "destination_not_a_file",
        }
    }
}

/// One provable discrepancy, with both sides of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMismatch {
    pub reason: VerificationMismatchReason,
    /// Destination path the mismatch is about.
    pub path: String,
    /// What the transfer promised, rendered for display.
    pub expected: String,
    /// What the destination actually holds, rendered for display.
    pub actual: String,
    pub detail: String,
}

impl VerificationMismatch {
    fn new(
        reason: VerificationMismatchReason,
        path: &Path,
        expected: impl Into<String>,
        actual: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            reason,
            path: path.display().to_string(),
            expected: expected.into(),
            actual: actual.into(),
            detail: detail.into(),
        }
    }
}

/// The result of verifying one copied file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVerification {
    /// Destination path that was checked.
    pub path: String,
    pub method: VerificationMethod,
    pub status: VerificationStatus,
    /// Size the plan measured, when there is one to compare against.
    pub expected_bytes: Option<u64>,
    /// Size the destination reported, when it could be read.
    pub actual_bytes: Option<u64>,
    pub checksum_algorithm: Option<ChecksumAlgorithm>,
    /// Digest of the bytes that came from the source.
    pub expected_checksum: Option<String>,
    /// Digest of the file on disk.
    pub actual_checksum: Option<String>,
    pub mismatch: Option<VerificationMismatch>,
    /// Structured failure, when the check could not be carried out. Stored as
    /// an error record so a verification result survives persistence.
    pub error: Option<StoredError>,
    pub duration_ms: u64,
}

impl FileVerification {
    /// A verification that did not run, recorded as such.
    pub fn skipped(path: &Path) -> Self {
        Self {
            path: path.display().to_string(),
            method: VerificationMethod::None,
            status: VerificationStatus::Skipped,
            expected_bytes: None,
            actual_bytes: None,
            checksum_algorithm: None,
            expected_checksum: None,
            actual_checksum: None,
            mismatch: None,
            error: None,
            duration_ms: 0,
        }
    }

    fn failed(
        path: &Path,
        method: VerificationMethod,
        error: AppError,
        duration: Duration,
    ) -> Self {
        let error = StoredError::from(&error);
        Self {
            path: path.display().to_string(),
            method,
            status: VerificationStatus::Failed,
            expected_bytes: None,
            actual_bytes: None,
            checksum_algorithm: None,
            expected_checksum: None,
            actual_checksum: None,
            mismatch: None,
            error: Some(error),
            duration_ms: millis(duration),
        }
    }

    /// The error a failed verification must fail its item with.
    ///
    /// A mismatch is reported as a `verification_failed` error so the job's own
    /// failure carries the reason, rather than only living in the summary.
    pub fn failure_error(&self) -> AppError {
        if let Some(error) = &self.error {
            return error.to_error();
        }
        let mismatch = self.mismatch.as_ref();
        match mismatch {
            Some(mismatch) => AppError::VerificationFailed(format!(
                "'{}' did not verify ({}): {}",
                mismatch.path,
                mismatch.reason.as_str(),
                mismatch.detail
            )),
            None => AppError::VerificationFailed(format!(
                "'{}' did not verify, and no reason was recorded",
                self.path
            )),
        }
    }
}

/// One item about to be verified.
pub struct ItemVerification<'a> {
    pub source: &'a Path,
    pub destination: &'a Path,
    /// Size the plan measured for this item.
    pub expected_bytes: u64,
    /// Digest of the bytes streamed out of the source during the copy, when the
    /// policy computed one. Absent means a checksum comparison is impossible.
    pub streamed_checksum: Option<String>,
}

/// Verifies one committed file. `Err` means the checkpoint stopped it.
///
/// The destination is inspected with `symlink_metadata`, so a symlink standing
/// where a file should be is reported as a mismatch instead of being followed
/// and reported as a match.
pub fn verify_item(
    request: ItemVerification<'_>,
    policy: VerificationPolicy,
    checkpoint: &mut dyn FnMut() -> bool,
) -> Result<FileVerification, VerificationAborted> {
    let started = Instant::now();
    let method = policy.method();

    if policy == VerificationPolicy::None {
        return Ok(FileVerification::skipped(request.destination));
    }

    let metadata = match std::fs::symlink_metadata(request.destination) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileVerification {
                path: request.destination.display().to_string(),
                method,
                status: VerificationStatus::Mismatch,
                expected_bytes: Some(request.expected_bytes),
                actual_bytes: None,
                checksum_algorithm: None,
                expected_checksum: None,
                actual_checksum: None,
                mismatch: Some(VerificationMismatch::new(
                    VerificationMismatchReason::DestinationMissing,
                    request.destination,
                    format!("{} bytes", request.expected_bytes),
                    "nothing",
                    "the destination is not there, so the transfer cannot have completed",
                )),
                error: None,
                duration_ms: millis(started.elapsed()),
            });
        }
        Err(error) => {
            return Ok(FileVerification::failed(
                request.destination,
                method,
                safety::read_error(error, request.destination),
                started.elapsed(),
            ))
        }
    };

    if !metadata.file_type().is_file() {
        return Ok(FileVerification {
            path: request.destination.display().to_string(),
            method,
            status: VerificationStatus::Mismatch,
            expected_bytes: Some(request.expected_bytes),
            actual_bytes: None,
            checksum_algorithm: None,
            expected_checksum: None,
            actual_checksum: None,
            mismatch: Some(VerificationMismatch::new(
                VerificationMismatchReason::DestinationNotAFile,
                request.destination,
                "a file",
                if metadata.file_type().is_symlink() {
                    "a link"
                } else {
                    "a directory or special entry"
                },
                "the destination is not the kind of entry the plan wrote there",
            )),
            error: None,
            duration_ms: millis(started.elapsed()),
        });
    }

    let actual_bytes = metadata.len();
    if actual_bytes != request.expected_bytes {
        return Ok(FileVerification {
            path: request.destination.display().to_string(),
            method,
            status: VerificationStatus::Mismatch,
            expected_bytes: Some(request.expected_bytes),
            actual_bytes: Some(actual_bytes),
            checksum_algorithm: None,
            expected_checksum: None,
            actual_checksum: None,
            mismatch: Some(VerificationMismatch::new(
                VerificationMismatchReason::SizeMismatch,
                request.destination,
                format!("{} bytes", request.expected_bytes),
                format!("{actual_bytes} bytes"),
                "the destination holds a different number of bytes than the source did",
            )),
            error: None,
            duration_ms: millis(started.elapsed()),
        });
    }

    if policy != VerificationPolicy::Checksum {
        return Ok(FileVerification {
            path: request.destination.display().to_string(),
            method,
            status: VerificationStatus::Verified,
            expected_bytes: Some(request.expected_bytes),
            actual_bytes: Some(actual_bytes),
            checksum_algorithm: None,
            expected_checksum: None,
            actual_checksum: None,
            mismatch: None,
            error: None,
            duration_ms: millis(started.elapsed()),
        });
    }

    let algorithm = ChecksumAlgorithm::Sha256;
    let Some(expected_checksum) = request.streamed_checksum else {
        // The policy asked for a checksum that was never computed. Saying
        // "verified" here would be the single most misleading thing this module
        // could do, so it is a failure with the reason attached.
        return Ok(FileVerification::failed(
            request.destination,
            method,
            AppError::VerificationFailed(format!(
                "'{}' needs a {} comparison, but the source bytes were never hashed",
                request.destination.display(),
                algorithm.as_str()
            )),
            started.elapsed(),
        ));
    };

    let outcome = match hash_file(request.destination, checkpoint) {
        Ok(outcome) => outcome,
        Err(error) => {
            return Ok(FileVerification::failed(
                request.destination,
                method,
                error,
                started.elapsed(),
            ))
        }
    };

    if !outcome.completed {
        return Err(VerificationAborted);
    }

    // The file could have grown or shrunk between the size check above and the
    // read that just happened; that is a size mismatch, not a checksum one.
    if outcome.bytes != request.expected_bytes {
        let actual_bytes = outcome.bytes;
        return Ok(FileVerification {
            path: request.destination.display().to_string(),
            method,
            status: VerificationStatus::Mismatch,
            expected_bytes: Some(request.expected_bytes),
            actual_bytes: Some(actual_bytes),
            checksum_algorithm: Some(algorithm),
            expected_checksum: Some(expected_checksum),
            actual_checksum: Some(outcome.digest),
            mismatch: Some(VerificationMismatch::new(
                VerificationMismatchReason::SizeMismatch,
                request.destination,
                format!("{} bytes", request.expected_bytes),
                format!("{actual_bytes} bytes"),
                "the destination changed size while it was being verified",
            )),
            error: None,
            duration_ms: millis(started.elapsed()),
        });
    }

    if outcome.digest != expected_checksum {
        let expected_display = format!("{} {}", algorithm.as_str(), short(&expected_checksum));
        let actual_display = format!("{} {}", algorithm.as_str(), short(&outcome.digest));
        return Ok(FileVerification {
            path: request.destination.display().to_string(),
            method,
            status: VerificationStatus::Mismatch,
            expected_bytes: Some(request.expected_bytes),
            actual_bytes: Some(outcome.bytes),
            checksum_algorithm: Some(algorithm),
            expected_checksum: Some(expected_checksum),
            actual_checksum: Some(outcome.digest),
            mismatch: Some(VerificationMismatch::new(
                VerificationMismatchReason::ChecksumMismatch,
                request.destination,
                expected_display,
                actual_display,
                "the bytes on disk are not the bytes that were read from the source",
            )),
            error: None,
            duration_ms: millis(started.elapsed()),
        });
    }

    Ok(FileVerification {
        path: request.destination.display().to_string(),
        method,
        status: VerificationStatus::Verified,
        expected_bytes: Some(request.expected_bytes),
        actual_bytes: Some(outcome.bytes),
        checksum_algorithm: Some(algorithm),
        expected_checksum: Some(expected_checksum),
        actual_checksum: Some(outcome.digest),
        mismatch: None,
        error: None,
        duration_ms: millis(started.elapsed()),
    })
}

/// Returned when a checkpoint stopped a verification. Never an error: the
/// caller turns it into a cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationAborted;

/// What a job's verification covered. Every claim is made here or not at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCoverage {
    /// Destination size was compared against the planned size.
    pub size: bool,
    /// Every planned entry was confirmed to exist as the kind of entry it was
    /// planned as (files through size verification, directories as part of the
    /// transfer's own completion).
    pub structure: bool,
    /// A checksum was compared.
    pub checksum: bool,
    /// Whether the source's modified time is preserved. This engine does not
    /// reapply it, so this is always false and says so.
    pub modified_time_preserved: bool,
    /// Whether the source's read-only attribute is preserved. Also not
    /// reapplied, so also always false.
    pub readonly_preserved: bool,
}

impl VerificationCoverage {
    fn for_policy(policy: VerificationPolicy) -> Self {
        Self {
            size: policy != VerificationPolicy::None,
            structure: policy != VerificationPolicy::None,
            checksum: policy == VerificationPolicy::Checksum,
            modified_time_preserved: false,
            readonly_preserved: false,
        }
    }
}

/// The verification verdict for a whole job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSummary {
    pub status: VerificationStatus,
    pub policy: VerificationPolicy,
    pub method: VerificationMethod,
    pub checksum_algorithm: Option<ChecksumAlgorithm>,
    pub planned_files: u64,
    pub checked_files: u64,
    pub verified_files: u64,
    pub mismatched_files: u64,
    pub failed_files: u64,
    pub skipped_files: u64,
    /// Planned files that no verification result was recorded for. Reported so
    /// `checked_files` can never be mistaken for "everything".
    pub unverified_files: u64,
    /// Bytes that passed verification.
    pub verified_bytes: u64,
    pub duration_ms: u64,
    pub coverage: VerificationCoverage,
    /// Bounded list of discrepancies, newest last.
    pub mismatches: Vec<VerificationMismatch>,
    /// True when more discrepancies happened than are listed.
    pub mismatches_truncated: bool,
    pub error: Option<StoredError>,
    /// One line the UI shows without reinterpreting any field. Rendered by the
    /// backend so the queue, a history record, and every notification describe
    /// the same run in the same words.
    pub verdict: String,
}

impl VerificationSummary {
    /// A summary for a job nothing has verified yet.
    pub fn pending(policy: VerificationPolicy) -> Self {
        Self {
            status: VerificationStatus::Pending,
            policy,
            method: policy.method(),
            checksum_algorithm: policy.algorithm(),
            planned_files: 0,
            checked_files: 0,
            verified_files: 0,
            mismatched_files: 0,
            failed_files: 0,
            skipped_files: 0,
            unverified_files: 0,
            verified_bytes: 0,
            duration_ms: 0,
            coverage: VerificationCoverage::for_policy(policy),
            mismatches: Vec::new(),
            mismatches_truncated: false,
            error: None,
            verdict: String::new(),
        }
        .with_verdict()
    }

    /// Whether the data this summary covers was proven to have arrived.
    pub fn is_proven(&self) -> bool {
        self.status.is_proven()
    }

    /// Fills the rendered verdict line from the rest of the summary.
    fn with_verdict(mut self) -> Self {
        self.verdict = self.render_verdict();
        self
    }

    /// One line the UI can show without reinterpreting any field. Kept private:
    /// `verdict` is the field that crosses the wire, so the rendered text and
    /// the serialized text can never drift apart.
    fn render_verdict(&self) -> String {
        match self.status {
            VerificationStatus::Verified => format!(
                "verified ({}{})",
                self.method.as_str(),
                match (self.checked_files, self.verified_bytes) {
                    (0, _) => String::new(),
                    (files, bytes) => format!(", {files} files, {bytes} bytes"),
                }
            ),
            VerificationStatus::Mismatch => format!(
                "{} of {} files did not verify",
                self.mismatched_files, self.checked_files
            ),
            VerificationStatus::Failed => format!(
                "verification could not be completed for {} files",
                self.failed_files
            ),
            VerificationStatus::Skipped => "not verified".to_string(),
            VerificationStatus::Pending => "verification has not started".to_string(),
            VerificationStatus::Verifying => "verification is in progress".to_string(),
        }
    }
}

/// Most discrepancies one job reports. Longer lists are counted and flagged.
const MAX_MISMATCHES: usize = 100;

/// Accumulates per-file results into the job-wide verdict.
///
/// Runs on the job's worker thread behind the job's own lock, so recording a
/// result never touches the queue lock.
#[derive(Debug, Default)]
pub struct VerificationLog {
    policy: VerificationPolicy,
    checked: u64,
    verified: u64,
    mismatched: u64,
    failed: u64,
    skipped: u64,
    verified_bytes: u64,
    planned_files: u64,
    mismatches: Vec<VerificationMismatch>,
    mismatch_count: u64,
    error: Option<StoredError>,
    started: Option<Instant>,
    finished: Option<Instant>,
}

impl VerificationLog {
    pub fn new(policy: VerificationPolicy) -> Self {
        Self {
            policy,
            checked: 0,
            verified: 0,
            mismatched: 0,
            failed: 0,
            skipped: 0,
            verified_bytes: 0,
            planned_files: 0,
            mismatches: Vec::new(),
            mismatch_count: 0,
            error: None,
            started: None,
            finished: None,
        }
    }

    pub fn policy(&self) -> VerificationPolicy {
        self.policy
    }

    /// How many planned files the job intends to verify.
    pub fn set_planned_files(&mut self, files: u64) {
        self.planned_files = files;
    }

    /// Records one file's result.
    pub fn record(&mut self, verification: &FileVerification) {
        let now = Instant::now();
        if verification.status != VerificationStatus::Skipped {
            self.started.get_or_insert(now);
        }

        match verification.status {
            VerificationStatus::Verified => {
                self.checked = self.checked.saturating_add(1);
                self.verified = self.verified.saturating_add(1);
                self.verified_bytes = self
                    .verified_bytes
                    .saturating_add(verification.actual_bytes.unwrap_or(0));
            }
            VerificationStatus::Mismatch => {
                self.checked = self.checked.saturating_add(1);
                self.mismatched = self.mismatched.saturating_add(1);
                if let Some(mismatch) = &verification.mismatch {
                    self.push_mismatch(mismatch.clone());
                }
            }
            VerificationStatus::Failed => {
                self.checked = self.checked.saturating_add(1);
                self.failed = self.failed.saturating_add(1);
                if self.error.is_none() {
                    self.error = verification.error.clone();
                }
            }
            VerificationStatus::Skipped => self.skipped = self.skipped.saturating_add(1),
            VerificationStatus::Pending | VerificationStatus::Verifying => {}
        }
    }

    /// Records that a verification stopped because the job was cancelled or a
    /// file could not be verified at all.
    pub fn record_error(&mut self, error: AppError) {
        self.failed = self.failed.saturating_add(1);
        self.checked = self.checked.saturating_add(1);
        if self.error.is_none() {
            self.error = Some(StoredError::from(&error));
        }
    }

    /// Whether anything recorded so far means the job must not report success.
    pub fn has_failures(&self) -> bool {
        self.mismatched > 0 || self.failed > 0
    }

    /// True when at least one file was verified.
    pub fn has_checked_anything(&self) -> bool {
        self.checked > 0
    }

    /// The verdict. `finished` is what the job's own lifecycle knows, and it is
    /// the only thing that can turn an in-progress log into a settled one.
    pub fn summary(&self, finished: bool) -> VerificationSummary {
        let now = Instant::now();
        let duration = match (self.started, self.finished) {
            (Some(started), Some(finished)) => finished.saturating_duration_since(started),
            (Some(started), None) => now.saturating_duration_since(started),
            (None, _) => Duration::ZERO,
        };

        let status = if self.mismatched > 0 {
            VerificationStatus::Mismatch
        } else if self.failed > 0 {
            VerificationStatus::Failed
        } else if self.skipped > 0 && self.checked == 0 {
            VerificationStatus::Skipped
        } else if self.verified > 0 {
            if finished {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Verifying
            }
        } else if finished {
            VerificationStatus::Skipped
        } else {
            VerificationStatus::Pending
        };

        let accounted = self
            .checked
            .saturating_add(self.skipped)
            .min(self.planned_files);

        VerificationSummary {
            status,
            policy: self.policy,
            method: self.policy.method(),
            checksum_algorithm: self.policy.algorithm(),
            planned_files: self.planned_files,
            checked_files: self.checked,
            verified_files: self.verified,
            mismatched_files: self.mismatched,
            failed_files: self.failed,
            skipped_files: self.skipped,
            unverified_files: self.planned_files.saturating_sub(accounted),
            verified_bytes: self.verified_bytes,
            duration_ms: millis(duration),
            coverage: VerificationCoverage::for_policy(self.policy),
            mismatches: self.mismatches.clone(),
            mismatches_truncated: self.mismatch_count > self.mismatches.len() as u64,
            error: self.error.clone(),
            verdict: String::new(),
        }
        .with_verdict()
    }

    /// Freezes the duration once the job is done.
    pub fn finish(&mut self) {
        self.finished.get_or_insert(Instant::now());
    }

    fn push_mismatch(&mut self, mismatch: VerificationMismatch) {
        self.mismatch_count = self.mismatch_count.saturating_add(1);
        if self.mismatches.len() < MAX_MISMATCHES {
            self.mismatches.push(mismatch);
        }
    }
}

/// Builds the identity of a file for later verification: the path it was
/// written to, plus the size and digest of what was read from the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSource {
    pub path: PathBuf,
    pub bytes: u64,
    pub checksum: Option<String>,
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// First twelve hex characters of a digest, for messages that should stay
/// readable. The full digest travels in the structured fields.
fn short(digest: &str) -> String {
    let hex = digest.trim_start_matches(SHA256_PREFIX);
    format!("{}…", &hex[..hex.len().min(12)])
}
