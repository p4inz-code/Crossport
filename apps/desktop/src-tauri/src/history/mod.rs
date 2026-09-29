/* ==========================================================================
 * Transfer history
 * The durable record of what happened: one entry per finished job, plus one
 * per interrupted job that recovery accounted for.
 *
 * History is deliberately separate from the live queue. A queue entry is
 * runtime state that disappears when the process does; a history entry is a
 * fact about the past that must still be readable months later, so it stores
 * plain values (paths, byte counts, timestamps, a status identifier) and never
 * a reference to anything in memory.
 *
 * Retention is by count and by recency: the newest entries are kept and the
 * oldest are pruned, never the other way round. A prune is reported, so a
 * caller can log it rather than discovering a shorter list later.
 * ========================================================================== */

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::errors::StoredError;
use crate::persistence::{read_document, write_document, LoadOutcome, LoadStatus};
use crate::transfer::{ConflictStrategy, TransferOperation, TransferStatus};
use crate::verification::{
    ChecksumAlgorithm, VerificationMethod, VerificationPolicy, VerificationStatus,
    VerificationSummary,
};

/// File name of the persisted history document.
pub const HISTORY_FILE: &str = "transfer-history.json";

/// How many finished jobs are kept by default.
pub const DEFAULT_HISTORY_LIMIT: u32 = 200;

/// Smallest retention limit that leaves a usable history.
pub const MIN_HISTORY_LIMIT: u32 = 20;

/// Largest retention limit the application accepts.
pub const MAX_HISTORY_LIMIT: u32 = 2000;

/// How a transfer ended, as history remembers it.
///
/// A superset of the live [`TransferStatus`]: history also has to describe a
/// job the application never saw finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatus {
    Completed,
    Failed,
    Cancelled,
    /// The application stopped while this job was queued or running, and the
    /// job never proved that it finished.
    Interrupted,
    /// An interrupted job that the archive proved had finished before the
    /// application stopped. Never inferred from a destination file alone.
    Recovered,
}

impl HistoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
            Self::Recovered => "recovered",
        }
    }

    /// Whether this status means the data arrived.
    pub fn succeeded(self) -> bool {
        matches!(self, Self::Completed | Self::Recovered)
    }

    /// Maps a live terminal status onto history. Live jobs that are not
    /// terminal yet have no history status, which the caller must handle
    /// rather than guess.
    pub fn from_transfer_status(status: TransferStatus) -> Option<Self> {
        match status {
            TransferStatus::Completed => Some(Self::Completed),
            TransferStatus::Failed => Some(Self::Failed),
            TransferStatus::Cancelled => Some(Self::Cancelled),
            TransferStatus::Queued
            | TransferStatus::Preparing
            | TransferStatus::Running
            | TransferStatus::Paused
            | TransferStatus::Cancelling => None,
        }
    }
}

/// What is decided about an interrupted transfer.
///
/// One enum serves both directions: a caller asks for an action, and a history
/// entry records the action that was taken about that episode. `Pending` is
/// what an episode that has not been decided yet carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Nothing has been decided yet; the entry is waiting for the user.
    Pending,
    /// The job never ran again; its partial output was removed.
    Discard,
    /// The job was queued again from the start.
    Restart,
    /// The archive proved the job had already finished, so nothing ran again.
    Confirm,
}

impl RecoveryAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Discard => "discard",
            Self::Restart => "restart",
            Self::Confirm => "confirm",
        }
    }

    /// Parses an identifier from the frontend, rejecting anything unknown so a
    /// typo cannot be treated as "do nothing".
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "discard" => Ok(Self::Discard),
            "restart" => Ok(Self::Restart),
            "confirm" => Ok(Self::Confirm),
            other => Err(format!(
                "recovery action must be one of [discard, restart, confirm], got '{other}'"
            )),
        }
    }
}

/// Verification as history keeps it: the verdict, without the per-file detail
/// that a live job carries while it is still collecting mismatches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryVerification {
    pub status: VerificationStatus,
    pub method: VerificationMethod,
    pub policy: VerificationPolicy,
    pub checksum_algorithm: Option<ChecksumAlgorithm>,
    pub checked_files: u64,
    pub verified_files: u64,
    pub mismatched_files: u64,
    pub failed_files: u64,
    pub verified_bytes: u64,
    /// One line describing the verdict, so a reader never has to reinterpret
    /// the fields.
    pub verdict: String,
}

impl From<&VerificationSummary> for HistoryVerification {
    fn from(summary: &VerificationSummary) -> Self {
        Self {
            status: summary.status,
            method: summary.method,
            policy: summary.policy,
            checksum_algorithm: summary.checksum_algorithm,
            checked_files: summary.checked_files,
            verified_files: summary.verified_files,
            mismatched_files: summary.mismatched_files,
            failed_files: summary.failed_files,
            verified_bytes: summary.verified_bytes,
            verdict: summary.verdict.clone(),
        }
    }
}

/// One issue as history keeps it.
///
/// Mirrors the live issue shape field for field, so the same rendering works
/// for both — but stores its error in the storable form, because a history
/// entry has to survive being written to disk and read back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryIssue {
    pub path: String,
    pub reason: crate::transfer::TransferIssueReason,
    pub error: Option<StoredError>,
    pub detail: Option<String>,
}

impl From<&crate::transfer::TransferIssue> for HistoryIssue {
    fn from(issue: &crate::transfer::TransferIssue) -> Self {
        Self {
            path: issue.path.clone(),
            reason: issue.reason,
            error: issue.error.as_ref().map(StoredError::from),
            detail: issue.detail.clone(),
        }
    }
}

/// One finished transfer, as history keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryRecord {
    /// The job identifier. Unique for the life of the process that ran it.
    pub id: String,
    pub operation: TransferOperation,
    pub conflict: ConflictStrategy,
    pub status: HistoryStatus,
    pub sources: Vec<String>,
    pub destination: String,

    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub total_files: u64,
    pub completed_files: u64,
    pub total_directories: u64,
    pub completed_directories: u64,
    pub skipped_items: u64,
    pub failed_items: u64,

    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: u64,
    /// Time the job spent working, milliseconds, paused time excluded.
    pub duration_ms: u64,

    /// Job-level failure, when there was one, in storable form.
    pub error: Option<StoredError>,
    pub issues_truncated: bool,
    /// Bounded sample of the issues the job reported, for the details view.
    pub issues: Vec<HistoryIssue>,

    pub verification: Option<HistoryVerification>,

    /// Set when this record came out of recovery rather than a live finish.
    pub recovery: Option<RecoveryAction>,
    /// The interrupted job this record was restarted from, when it was.
    pub recovered_from: Option<String>,
}

impl Default for HistoryRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            operation: TransferOperation::Copy,
            conflict: ConflictStrategy::Skip,
            status: HistoryStatus::Completed,
            sources: Vec::new(),
            destination: String::new(),
            total_bytes: 0,
            transferred_bytes: 0,
            total_files: 0,
            completed_files: 0,
            total_directories: 0,
            completed_directories: 0,
            skipped_items: 0,
            failed_items: 0,
            queued_at_ms: 0,
            started_at_ms: None,
            finished_at_ms: 0,
            duration_ms: 0,
            error: None,
            issues_truncated: false,
            issues: Vec::new(),
            verification: None,
            recovery: None,
            recovered_from: None,
        }
    }
}

impl HistoryRecord {
    /// Whether the record's own fields are consistent enough to store.
    ///
    /// A record with no identifier or no destination cannot be displayed
    /// meaningfully, so it is refused rather than written and shown as blank.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("a history record needs an identifier".to_string());
        }
        if self.sources.is_empty() {
            return Err("a history record needs at least one source".to_string());
        }
        if self.destination.trim().is_empty() {
            return Err("a history record needs a destination".to_string());
        }
        Ok(())
    }

    /// The record's display title: the first source, or its file name.
    pub fn source_summary(&self) -> String {
        match self.sources.first() {
            Some(first) if self.sources.len() == 1 => first.clone(),
            Some(first) => format!("{} and {} more", first, self.sources.len() - 1),
            None => "(no source)".to_string(),
        }
    }
}

/// Which records a caller wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryFilter {
    #[default]
    All,
    Completed,
    Failed,
    Cancelled,
    /// Interrupted and recovered entries: what recovery accounted for.
    Interrupted,
}

impl HistoryFilter {
    /// Every filter a client may ask for, in display order.
    pub const ALL: [HistoryFilter; 5] = [
        HistoryFilter::All,
        HistoryFilter::Completed,
        HistoryFilter::Failed,
        HistoryFilter::Cancelled,
        HistoryFilter::Interrupted,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    /// Parses a filter identifier, rejecting anything unknown so a typo cannot
    /// silently show the wrong records.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "all" => Ok(Self::All),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted),
            other => Err(format!(
                "history filter must be one of [all, completed, failed, cancelled, interrupted], got '{other}'"
            )),
        }
    }

    pub fn matches(self, status: HistoryStatus) -> bool {
        match self {
            Self::All => true,
            Self::Completed => status == HistoryStatus::Completed,
            Self::Failed => status == HistoryStatus::Failed,
            Self::Cancelled => status == HistoryStatus::Cancelled,
            Self::Interrupted => {
                matches!(
                    status,
                    HistoryStatus::Interrupted | HistoryStatus::Recovered
                )
            }
        }
    }
}

/// The persisted history document.
///
/// Records are stored newest first, which is the order every caller wants and
/// the order retention prunes from the end of.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryDocument {
    pub records: Vec<HistoryRecord>,
}

/// What a prune removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneOutcome {
    pub removed: usize,
    /// Records dropped because they could not be stored.
    pub invalid: usize,
}

/// The history file plus the in-memory view of it.
///
/// All mutation goes through one lock, so a query can never observe a half
/// pruned list, and a write always reflects the list that was just pruned.
pub struct HistoryStore {
    path: std::path::PathBuf,
    inner: std::sync::Mutex<Inner>,
}

struct Inner {
    records: Vec<HistoryRecord>,
    limit: usize,
    /// False when the file belongs to a newer build: reads work in memory, but
    /// nothing may be written over that file.
    writable: bool,
}

impl HistoryStore {
    /// Opens the store at `path`, reading whatever is there.
    ///
    /// `limit` is the retention bound; it is clamped to the accepted range so a
    /// hostile or corrupt setting cannot produce an empty history or an
    /// unbounded one.
    pub fn open(path: std::path::PathBuf, limit: u32) -> (Self, LoadStatus) {
        let outcome: LoadOutcome<HistoryDocument> = read_document(&path, migrate);
        let status = outcome.status.clone();
        let writable = outcome.writable();
        let records = sanitize(outcome.value.records);

        (
            Self {
                path,
                inner: std::sync::Mutex::new(Inner {
                    records,
                    limit: clamp_limit(limit),
                    writable,
                }),
            },
            status,
        )
    }

    /// The path this store persists to.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Whether the store may write. False only for a newer-schema file.
    pub fn writable(&self) -> bool {
        lock(&self.inner).writable
    }

    /// Changes the retention bound and prunes immediately.
    pub fn set_limit(&self, limit: u32) -> Result<PruneOutcome, crate::errors::AppError> {
        let mut inner = lock(&self.inner);
        let limit = clamp_limit(limit);
        inner.limit = limit;
        let removed = prune(&mut inner.records, limit);

        // The write happens under the same lock as the mutation, so two
        // callers can never replace the document with lists in an order that
        // loses one of their records.
        if removed > 0 && inner.writable {
            persist(&self.path, &inner.records)?;
        }
        Ok(PruneOutcome {
            removed,
            invalid: 0,
        })
    }

    /// Every record that matches, newest first.
    pub fn list(&self, filter: HistoryFilter) -> Vec<HistoryRecord> {
        lock(&self.inner)
            .records
            .iter()
            .filter(|record| filter.matches(record.status))
            .cloned()
            .collect()
    }

    /// One record by identifier.
    pub fn get(&self, id: &str) -> Option<HistoryRecord> {
        lock(&self.inner)
            .records
            .iter()
            .find(|record| record.id == id)
            .cloned()
    }

    /// How many records are kept.
    pub fn len(&self) -> usize {
        lock(&self.inner).records.len()
    }

    /// The retention bound currently in force.
    pub fn limit(&self) -> u32 {
        lock(&self.inner).limit as u32
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Records a finished job, pruning to the retention bound.
    ///
    /// A record whose identifier is already known, or that fails validation, is
    /// refused: the caller finds out rather than silently losing the entry.
    pub fn record(&self, record: HistoryRecord) -> Result<PruneOutcome, crate::errors::AppError> {
        if let Err(detail) = record.validate() {
            return Err(crate::errors::AppError::InvalidInput(detail));
        }

        let mut inner = lock(&self.inner);
        let mut invalid = 0;

        if inner
            .records
            .iter()
            .any(|existing| existing.id == record.id)
        {
            return Err(crate::errors::AppError::InvalidInput(format!(
                "history already has an entry for '{}'",
                record.id
            )));
        }

        // Newest first: the list is kept in the order it is displayed and
        // pruned from the tail.
        let limit = inner.limit;
        inner.records.insert(0, record);
        let before = inner.records.len();
        invalid += prune(&mut inner.records, limit);
        let removed = before.saturating_sub(inner.records.len());

        // Written while the lock is held: a concurrent record must not be able
        // to commit a document that later overwrites this one by accident.
        if inner.writable {
            persist(&self.path, &inner.records)?;
        }

        Ok(PruneOutcome { removed, invalid })
    }

    /// Replaces an existing record, for a transfer whose outcome recovery
    /// revised after the fact.
    ///
    /// `false` means the identifier is unknown, so the caller decides whether
    /// to insert instead. Retention is untouched: the record was already in the
    /// list, so replacing it cannot exceed the bound.
    pub fn update(&self, record: HistoryRecord) -> Result<bool, crate::errors::AppError> {
        if let Err(detail) = record.validate() {
            return Err(crate::errors::AppError::InvalidInput(detail));
        }

        let mut inner = lock(&self.inner);
        let Some(index) = inner
            .records
            .iter()
            .position(|existing| existing.id == record.id)
        else {
            return Ok(false);
        };

        inner.records[index] = record;

        if inner.writable {
            persist(&self.path, &inner.records)?;
        }
        Ok(true)
    }

    /// Removes one record. `false` means it was not there.
    pub fn delete(&self, id: &str) -> Result<bool, crate::errors::AppError> {
        let mut inner = lock(&self.inner);
        let before = inner.records.len();
        inner.records.retain(|record| record.id != id);
        let removed = inner.records.len() != before;

        if removed && inner.writable {
            persist(&self.path, &inner.records)?;
        }
        Ok(removed)
    }

    /// Removes every record and returns how many were removed.
    pub fn clear(&self) -> Result<usize, crate::errors::AppError> {
        let mut inner = lock(&self.inner);
        let removed = inner.records.len();
        inner.records.clear();

        if removed > 0 && inner.writable {
            persist(&self.path, &[])?;
        }
        Ok(removed)
    }
}

impl std::fmt::Debug for HistoryStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = lock(&self.inner);
        formatter
            .debug_struct("HistoryStore")
            .field("path", &self.path)
            .field("records", &inner.records.len())
            .field("limit", &inner.limit)
            .field("writable", &inner.writable)
            .finish()
    }
}

/// Clamps a retention limit into the accepted range.
///
/// A limit below the minimum is raised rather than honoured: an accidental `0`
/// must not silently erase the user's history.
pub fn clamp_limit(limit: u32) -> usize {
    limit.clamp(MIN_HISTORY_LIMIT, MAX_HISTORY_LIMIT) as usize
}

/// Validates a retention limit as a setting, so it is rejected at the boundary
/// rather than silently clamped when it is used.
pub fn validate_limit(limit: u32) -> Result<(), String> {
    if (MIN_HISTORY_LIMIT..=MAX_HISTORY_LIMIT).contains(&limit) {
        Ok(())
    } else {
        Err(format!(
            "history limit must be between {MIN_HISTORY_LIMIT} and {MAX_HISTORY_LIMIT}"
        ))
    }
}

/// Drops records that cannot be stored or displayed, keeping the rest.
///
/// A single unreadable entry must not cost the user their whole history.
fn sanitize(records: Vec<HistoryRecord>) -> Vec<HistoryRecord> {
    let mut kept = Vec::with_capacity(records.len());
    let mut dropped = 0usize;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for record in records {
        if record.validate().is_err() || !seen.insert(record.id.clone()) {
            dropped += 1;
            continue;
        }
        kept.push(record);
    }

    if dropped > 0 {
        log::warn!("dropped {dropped} unusable transfer history entries");
    }
    kept
}

/// Trims a list to `limit`, removing from the oldest end. Returns how many went.
fn prune(records: &mut Vec<HistoryRecord>, limit: usize) -> usize {
    if records.len() <= limit {
        return 0;
    }
    let removed = records.len() - limit;
    records.truncate(limit);
    removed
}

fn persist(
    path: &std::path::Path,
    records: &[HistoryRecord],
) -> Result<(), crate::errors::AppError> {
    write_document(
        path,
        &HistoryDocument {
            records: records.to_vec(),
        },
    )
}

/// Migration for the history document.
///
/// Version 0 is a document written before this module existed. It has no
/// records, so the additive default migration is exactly right, and it is the
/// shape a future rename would extend rather than replace.
fn migrate(
    _from: u32,
    document: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    crate::persistence::migrate_fill_defaults(0, document)
}

fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
