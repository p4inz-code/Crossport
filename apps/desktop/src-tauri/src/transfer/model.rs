/* ==========================================================================
 * Transfer domain model
 * The typed vocabulary of the transfer engine: what a request is, what a
 * planned item is, the lifecycle a job moves through, and the numbers the UI
 * is allowed to display.
 *
 * Nothing here is stringly typed. Statuses, operations, conflict strategies,
 * item actions, and issue reasons are enums that serialize to stable lowercase
 * identifiers, and the frontend mirrors them with the same literals
 * (`src/types/transfer.ts`).
 *
 * Internal planning state (`TransferItem`, `TransferPlan`) is deliberately not
 * serializable: only [`TransferPreview`] and [`TransferSnapshot`] cross the IPC
 * boundary, so the wire contract stays small and stable.
 * ========================================================================== */

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::AppError;

/// What a transfer does with each source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferOperation {
    /// Source stays in place.
    Copy,
    /// Source is removed once the destination is complete.
    Move,
}

impl TransferOperation {
    /// Stable identifier sent across the IPC boundary.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

/// Lifecycle of one transfer job.
///
/// `Preparing` is the window between a worker claiming a job and the first byte
/// moving; a cancel or pause taken there is honoured before any work starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    /// Waiting for a worker. Nothing has been touched. Can be cancelled.
    Queued,
    /// Claimed by a worker and about to start.
    Preparing,
    /// Data is moving.
    Running,
    /// Cooperatively parked by the user; state is intact and resumable.
    Paused,
    /// Cancellation requested; the worker is unwinding.
    Cancelling,
    /// Every item finished. Terminal.
    Completed,
    /// At least one item failed, or the job could not proceed. Terminal.
    Failed,
    /// Cancelled by the user. Terminal.
    Cancelled,
}

impl TransferStatus {
    /// Stable identifier sent across the IPC boundary.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preparing => "preparing",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Whether the job is finished and will never run again.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Whether the job is holding (or about to hold) engine resources.
    pub fn is_live(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Preparing | Self::Running | Self::Paused | Self::Cancelling
        )
    }
}

/// How a destination collision is resolved. The choice is made before the job
/// starts and applies to every item in it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    /// Overwrite what is already there.
    Replace,
    /// Leave the existing entry untouched and skip the item. The default:
    /// a transfer never destroys data unless the user asked it to.
    #[default]
    Skip,
    /// Write next to the existing entry under a unique name.
    Rename,
}

impl ConflictStrategy {
    /// Stable identifier sent across the IPC boundary.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Skip => "skip",
            Self::Rename => "rename",
        }
    }
}

/// A request from the UI to place one or more sources inside a destination
/// directory.
///
/// Paths are strings because that is what crosses the IPC boundary; they are
/// normalized and validated in Rust before anything is touched, and the
/// frontend never builds or joins a path itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferRequest {
    /// Absolute source paths. Each source is placed *inside* `destination`.
    pub sources: Vec<String>,
    /// Existing absolute directory the sources are transferred into.
    pub destination: String,
    pub operation: TransferOperation,
    #[serde(default)]
    pub conflict: ConflictStrategy,
}

/// What a planned item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    File,
    Directory,
}

/// What the engine will do with a planned item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// Create it at the destination.
    Transfer,
    /// Leave the destination alone.
    Skip,
}

/// One planned entry: a file to stream or a directory to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferItem {
    /// Absolute, validated source path.
    pub source: PathBuf,
    /// Absolute destination path, already conflict-resolved.
    pub destination: PathBuf,
    pub kind: ItemKind,
    /// Bytes the source reports. Directories carry `0`.
    pub size_bytes: u64,
    /// Index into [`TransferPlan::roots`].
    pub root_index: usize,
    pub action: ItemAction,
    /// Why the item is skipped; `None` for transferred items.
    pub skip_reason: Option<TransferIssueReason>,
    /// Human-readable reason, kept apart from failures.
    pub skip_detail: Option<String>,
    /// An existing destination entry that must be removed before this item is
    /// written, which only the `Replace` strategy ever sets: a file sitting
    /// where a directory goes (or the other way round).
    pub clear_first: Option<ItemKind>,
    /// Whether something already exists at `destination`.
    pub existed: bool,
}

impl TransferItem {
    /// Whether this item counts toward the planned byte total.
    pub fn transfers_bytes(&self) -> bool {
        self.action == ItemAction::Transfer && self.kind == ItemKind::File
    }
}

/// One top-level source of a plan, with the items it expanded into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRoot {
    pub source: PathBuf,
    /// Where the source itself lands.
    pub destination: PathBuf,
    pub kind: ItemKind,
    pub action: ItemAction,
    pub skip_reason: Option<TransferIssueReason>,
    pub skip_detail: Option<String>,
    /// First index in [`TransferPlan::items`] that belongs to this root.
    pub item_start: usize,
    /// Number of items that belong to this root.
    pub item_count: usize,
    /// Files that will be copied for this root.
    pub files: u64,
    /// Bytes that will be copied for this root.
    pub bytes: u64,
    /// Bytes this root will leave behind because the conflict strategy skips
    /// them. Known exactly for skipped files; `0` for a skipped folder that was
    /// never walked.
    pub skipped_bytes: u64,
    /// True when nothing inside this root collides with an existing
    /// destination entry, which lets a same-volume move rename it wholesale.
    pub clean: bool,
}

/// A planned transfer: destinations resolved, sizes known, nothing touching
/// the filesystem yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPlan {
    pub destination: PathBuf,
    pub operation: TransferOperation,
    pub conflict: ConflictStrategy,
    pub roots: Vec<PlanRoot>,
    /// Every directory and file to create, parents before children.
    pub items: Vec<TransferItem>,
    /// Bytes of the files that will actually be copied.
    pub total_bytes: u64,
    /// Files that will actually be copied.
    pub total_files: u64,
    /// Directories referenced by the plan (transferred or pre-existing).
    pub total_directories: u64,
    /// Root, file, and directory items whose destination already existed.
    pub conflicts: u64,
    /// Items left alone by the `Skip` strategy or because they cannot be
    /// followed (links and reparse points).
    pub skipped_items: u64,
    /// Bytes of skipped files; known exactly for files, `0` for skipped
    /// directories that were never walked.
    pub skipped_bytes: u64,
    /// Bytes the destination must be able to hold.
    pub required_bytes: u64,
}

impl TransferPlan {
    /// Items belonging to one root, in plan order.
    pub fn root_items(&self, root: &PlanRoot) -> &[TransferItem] {
        &self.items[root.item_start..root.item_start + root.item_count]
    }
}

/// Why an issue was recorded. Only [`TransferIssueReason::Failed`] fails a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferIssueReason {
    /// The item could not be transferred.
    Failed,
    /// The item was deliberately left alone by the conflict strategy.
    Skipped,
    /// The item is a symlink, junction, or other reparse point, which the
    /// engine reports but never follows or copies.
    Unsupported,
}

/// One thing the user should know about a job that is not plain progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferIssue {
    /// Path the issue is about (the destination for failed items, the source
    /// for skipped and unsupported ones).
    pub path: String,
    pub reason: TransferIssueReason,
    /// Structured backend error, set for failures only.
    pub error: Option<AppError>,
    /// Human-readable detail for skipped and unsupported items.
    pub detail: Option<String>,
}

impl TransferIssue {
    /// A structured failure for one item.
    pub fn failed(path: impl Into<String>, error: AppError) -> Self {
        Self {
            path: path.into(),
            reason: TransferIssueReason::Failed,
            error: Some(error),
            detail: None,
        }
    }

    /// An item left alone by the conflict strategy.
    pub fn skipped(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: TransferIssueReason::Skipped,
            error: None,
            detail: Some(detail.into()),
        }
    }

    /// An entry the engine does not follow or copy.
    pub fn unsupported(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: TransferIssueReason::Unsupported,
            error: None,
            detail: Some(detail.into()),
        }
    }
}

/// Counters the engine updates as data moves. Every number here comes from
/// work that actually happened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferCounters {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub total_files: u64,
    pub completed_files: u64,
    pub total_directories: u64,
    /// Directory items the engine has processed. A directory that already
    /// existed counts as done, which is what makes the item-based percentage
    /// reach 100 for a merge into an existing tree.
    pub completed_directories: u64,
    pub skipped_items: u64,
    pub skipped_bytes: u64,
    pub failed_items: u64,
    /// File currently being written, for display.
    pub current_file: Option<String>,
    pub current_file_bytes: u64,
    pub current_file_total_bytes: u64,
}

/// Timing facts derived from the job's own clock, not from a fixed cadence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransferTiming {
    /// Milliseconds the job has actually been working (paused time excluded).
    pub elapsed_ms: u64,
    /// Speed over the recent window; `0` until two samples exist.
    pub bytes_per_second: u64,
    /// Speed over the whole run; `0` until the first byte is written.
    pub average_bytes_per_second: u64,
    /// Seconds left at the current speed, when that is meaningful.
    pub eta_seconds: Option<u64>,
}

/// Progress as the UI receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub total_files: u64,
    pub completed_files: u64,
    pub total_directories: u64,
    pub completed_directories: u64,
    pub skipped_items: u64,
    pub skipped_bytes: u64,
    pub failed_items: u64,
    pub current_file: Option<String>,
    pub current_file_bytes: u64,
    pub current_file_total_bytes: u64,
    /// Whole percent, clamped. `None` when the plan has nothing to measure.
    pub percent: Option<u32>,
    pub bytes_per_second: u64,
    pub average_bytes_per_second: u64,
    pub eta_seconds: Option<u64>,
    pub elapsed_ms: u64,
}

impl TransferCounters {
    /// Derives the displayable progress. Percent comes from bytes when the plan
    /// has bytes to move, and from item counts otherwise (an empty-directory
    /// transfer still deserves a real progress bar).
    pub fn to_progress(&self, timing: TransferTiming) -> TransferProgress {
        TransferProgress {
            total_bytes: self.total_bytes,
            transferred_bytes: self.transferred_bytes,
            total_files: self.total_files,
            completed_files: self.completed_files,
            total_directories: self.total_directories,
            completed_directories: self.completed_directories,
            skipped_items: self.skipped_items,
            skipped_bytes: self.skipped_bytes,
            failed_items: self.failed_items,
            current_file: self.current_file.clone(),
            current_file_bytes: self.current_file_bytes,
            current_file_total_bytes: self.current_file_total_bytes,
            percent: percent_complete(self),
            bytes_per_second: timing.bytes_per_second,
            average_bytes_per_second: timing.average_bytes_per_second,
            eta_seconds: timing.eta_seconds,
            elapsed_ms: timing.elapsed_ms,
        }
    }
}

/// Whole percent of the work that is done, or `None` when there is nothing to
/// measure.
pub fn percent_complete(counters: &TransferCounters) -> Option<u32> {
    let (done, total) = if counters.total_bytes > 0 {
        (
            counters.transferred_bytes.min(counters.total_bytes),
            counters.total_bytes,
        )
    } else {
        let items = counters
            .total_files
            .saturating_add(counters.total_directories);
        if items == 0 {
            return None;
        }
        (
            counters
                .completed_files
                .saturating_add(counters.completed_directories)
                .min(items),
            items,
        )
    };

    // `done * 100` fits comfortably in u64 for any counter a real volume can
    // produce; the multiply is checked anyway so a counter overflow can never
    // wrap into a nonsense percentage.
    let percent = done.checked_mul(100)?.checked_div(total)?;
    Some(percent.min(100) as u32)
}

/// Seconds left at `bytes_per_second`, or `None` when that cannot be said
/// honestly (no speed yet, nothing left, or no known total).
pub fn estimate_eta_seconds(
    remaining_bytes: u64,
    total_bytes: u64,
    bytes_per_second: u64,
) -> Option<u64> {
    if total_bytes == 0 || remaining_bytes == 0 || bytes_per_second == 0 {
        return None;
    }
    remaining_bytes.checked_div(bytes_per_second)
}

/// One job as the frontend sees it: status, progress, issues, and timing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSnapshot {
    /// Stable identifier for the life of the job.
    pub id: String,
    pub operation: TransferOperation,
    pub conflict: ConflictStrategy,
    pub status: TransferStatus,
    pub sources: Vec<String>,
    pub destination: String,
    pub progress: TransferProgress,
    /// Job-level failure. Per-item failures live in `issues`.
    pub error: Option<AppError>,
    pub issues: Vec<TransferIssue>,
    /// True when more issues happened than are listed.
    pub issues_truncated: bool,
    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}

/// One root of a preview, so the UI can show where each source lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPreviewRoot {
    pub source: String,
    pub destination: String,
    pub kind: ItemKind,
    /// True when this source will be left alone by the conflict strategy.
    pub skipped: bool,
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
}

/// A dry run of a transfer request: what would be copied, where it would land,
/// and what would collide. Nothing is created, moved, or deleted by planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPreview {
    pub sources: Vec<String>,
    pub destination: String,
    pub operation: TransferOperation,
    pub conflict: ConflictStrategy,
    pub total_bytes: u64,
    pub total_files: u64,
    pub total_directories: u64,
    /// Files and directories whose destination already exists.
    pub conflicts: u64,
    /// Items the conflict strategy would leave alone.
    pub skipped_items: u64,
    pub skipped_bytes: u64,
    /// Free space the host reports for the destination, when it reports any.
    pub available_bytes: Option<u64>,
    /// Whether a move of the first source could be a rename instead of a copy.
    pub same_volume: bool,
    pub roots: Vec<TransferPreviewRoot>,
}

impl TransferPreview {
    /// Builds the preview from a plan and the host facts about the destination.
    pub fn from_plan(plan: &TransferPlan, available_bytes: Option<u64>) -> Self {
        let same_volume = plan
            .roots
            .first()
            .map(|root| crate::platform::drives::same_volume(&root.source, &plan.destination))
            .unwrap_or(true);

        Self {
            sources: plan
                .roots
                .iter()
                .map(|root| root.source.display().to_string())
                .collect(),
            destination: plan.destination.display().to_string(),
            operation: plan.operation,
            conflict: plan.conflict,
            total_bytes: plan.total_bytes,
            total_files: plan.total_files,
            total_directories: plan.total_directories,
            conflicts: plan.conflicts,
            skipped_items: plan.skipped_items,
            skipped_bytes: plan.skipped_bytes,
            available_bytes,
            same_volume,
            roots: plan
                .roots
                .iter()
                .map(|root| TransferPreviewRoot {
                    source: root.source.display().to_string(),
                    destination: root.destination.display().to_string(),
                    kind: root.kind,
                    skipped: root.action == ItemAction::Skip,
                    files: root.files,
                    directories: root
                        .item_count
                        .saturating_sub(root.files as usize)
                        .try_into()
                        .unwrap_or(u64::MAX),
                    bytes: root.bytes,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counters(total_bytes: u64, transferred: u64) -> TransferCounters {
        TransferCounters {
            total_bytes,
            transferred_bytes: transferred,
            ..TransferCounters::default()
        }
    }

    #[test]
    fn status_identifiers_are_stable_on_the_wire() {
        let cases = [
            (TransferStatus::Queued, "queued"),
            (TransferStatus::Preparing, "preparing"),
            (TransferStatus::Running, "running"),
            (TransferStatus::Paused, "paused"),
            (TransferStatus::Cancelling, "cancelling"),
            (TransferStatus::Completed, "completed"),
            (TransferStatus::Failed, "failed"),
            (TransferStatus::Cancelled, "cancelled"),
        ];

        for (status, identifier) in cases {
            let json = serde_json::to_value(status).expect("status serializes");
            assert_eq!(json, serde_json::json!(identifier));
        }
    }

    #[test]
    fn operation_and_conflict_identifiers_are_stable_on_the_wire() {
        assert_eq!(
            serde_json::to_value(TransferOperation::Copy).expect("serializes"),
            serde_json::json!("copy")
        );
        assert_eq!(
            serde_json::to_value(TransferOperation::Move).expect("serializes"),
            serde_json::json!("move")
        );
        assert_eq!(
            serde_json::to_value(ConflictStrategy::Replace).expect("serializes"),
            serde_json::json!("replace")
        );
        assert_eq!(
            serde_json::to_value(ConflictStrategy::Skip).expect("serializes"),
            serde_json::json!("skip")
        );
        assert_eq!(
            serde_json::to_value(ConflictStrategy::Rename).expect("serializes"),
            serde_json::json!("rename")
        );
    }

    #[test]
    fn terminal_and_live_states_do_not_overlap() {
        for status in [
            TransferStatus::Queued,
            TransferStatus::Preparing,
            TransferStatus::Running,
            TransferStatus::Paused,
            TransferStatus::Cancelling,
            TransferStatus::Completed,
            TransferStatus::Failed,
            TransferStatus::Cancelled,
        ] {
            assert_ne!(
                status.is_terminal(),
                status.is_live(),
                "{status:?} must be either live or terminal"
            );
        }
    }

    #[test]
    fn skip_is_the_default_conflict_strategy() {
        assert_eq!(ConflictStrategy::default(), ConflictStrategy::Skip);
    }

    #[test]
    fn request_deserializes_the_published_camel_case_contract() {
        let request: TransferRequest = serde_json::from_value(serde_json::json!({
            "sources": ["C:\\a.txt"],
            "destination": "D:\\Backup",
            "operation": "move",
            "conflict": "rename",
        }))
        .expect("the request is valid");

        assert_eq!(request.sources, vec!["C:\\a.txt"]);
        assert_eq!(request.operation, TransferOperation::Move);
        assert_eq!(request.conflict, ConflictStrategy::Rename);
    }

    #[test]
    fn request_defaults_the_conflict_strategy_to_skip() {
        let request: TransferRequest = serde_json::from_value(serde_json::json!({
            "sources": ["C:\\a.txt"],
            "destination": "D:\\Backup",
            "operation": "copy",
        }))
        .expect("the request is valid");

        assert_eq!(request.conflict, ConflictStrategy::Skip);
    }

    #[test]
    fn request_rejects_unknown_fields_and_bad_operations() {
        let unknown = serde_json::from_value::<TransferRequest>(serde_json::json!({
            "sources": ["C:\\a.txt"],
            "destination": "D:\\Backup",
            "operation": "copy",
            "overwrite": true,
        }));
        assert!(unknown.is_err(), "unknown fields are rejected");

        let bad_operation = serde_json::from_value::<TransferRequest>(serde_json::json!({
            "sources": ["C:\\a.txt"],
            "destination": "D:\\Backup",
            "operation": "delete",
        }));
        assert!(bad_operation.is_err(), "operations are a closed set");
    }

    #[test]
    fn percent_comes_from_bytes_when_there_are_bytes() {
        assert_eq!(percent_complete(&counters(1000, 250)), Some(25));
        assert_eq!(percent_complete(&counters(1000, 1000)), Some(100));
        assert_eq!(percent_complete(&counters(1000, 0)), Some(0));
    }

    #[test]
    fn percent_never_exceeds_one_hundred_when_a_source_grows() {
        assert_eq!(
            percent_complete(&counters(1000, 5000)),
            Some(100),
            "a source that grew mid-transfer cannot report 500%"
        );
    }

    #[test]
    fn percent_falls_back_to_item_counts_for_empty_directory_plans() {
        let mut empty = TransferCounters {
            total_directories: 4,
            completed_directories: 1,
            total_files: 0,
            ..TransferCounters::default()
        };

        assert_eq!(percent_complete(&empty), Some(25));

        empty.completed_directories = 4;
        assert_eq!(percent_complete(&empty), Some(100));
    }

    #[test]
    fn percent_is_unknown_when_nothing_is_planned() {
        assert_eq!(percent_complete(&TransferCounters::default()), None);
    }

    #[test]
    fn eta_needs_a_speed_a_total_and_something_left() {
        assert_eq!(estimate_eta_seconds(0, 1000, 100), None);
        assert_eq!(estimate_eta_seconds(1000, 0, 100), None);
        assert_eq!(estimate_eta_seconds(1000, 1000, 0), None);
        assert_eq!(estimate_eta_seconds(1000, 2000, 100), Some(10));
    }

    #[test]
    fn completed_directories_is_what_makes_an_empty_tree_reach_full_progress() {
        let counters = TransferCounters {
            total_directories: 3,
            completed_directories: 3,
            total_files: 0,
            ..TransferCounters::default()
        };

        assert_eq!(percent_complete(&counters), Some(100));
    }

    #[test]
    fn progress_serializes_exactly_the_frontend_contract() {
        let progress = TransferCounters {
            total_bytes: 1000,
            transferred_bytes: 250,
            total_files: 4,
            completed_files: 1,
            total_directories: 2,
            completed_directories: 1,
            skipped_items: 1,
            skipped_bytes: 16,
            failed_items: 0,
            current_file: Some("C:\\a.bin".to_string()),
            current_file_bytes: 250,
            current_file_total_bytes: 400,
        }
        .to_progress(TransferTiming {
            elapsed_ms: 500,
            bytes_per_second: 500,
            average_bytes_per_second: 400,
            eta_seconds: Some(2),
        });

        let json = serde_json::to_value(&progress).expect("progress serializes");

        assert_eq!(
            json,
            serde_json::json!({
                "totalBytes": 1000,
                "transferredBytes": 250,
                "totalFiles": 4,
                "completedFiles": 1,
                "totalDirectories": 2,
                "completedDirectories": 1,
                "skippedItems": 1,
                "skippedBytes": 16,
                "failedItems": 0,
                "currentFile": "C:\\a.bin",
                "currentFileBytes": 250,
                "currentFileTotalBytes": 400,
                "percent": 25,
                "bytesPerSecond": 500,
                "averageBytesPerSecond": 400,
                "etaSeconds": 2,
                "elapsedMs": 500,
            })
        );
        assert!(
            json.get("total_bytes").is_none(),
            "the IPC contract is camelCase"
        );
    }

    #[test]
    fn issue_helpers_keep_failures_and_skips_apart() {
        let failed = TransferIssue::failed("D:\\a.txt", AppError::DiskFull("no room".into()));
        assert_eq!(failed.reason, TransferIssueReason::Failed);
        assert_eq!(failed.error.as_ref().map(AppError::code), Some("disk_full"));
        assert!(failed.detail.is_none());

        let skipped = TransferIssue::skipped("D:\\a.txt", "destination already exists");
        assert_eq!(skipped.reason, TransferIssueReason::Skipped);
        assert!(skipped.error.is_none());

        let unsupported = TransferIssue::unsupported("D:\\link", "symbolic links are not copied");
        assert_eq!(unsupported.reason, TransferIssueReason::Unsupported);
        assert_eq!(
            serde_json::to_value(unsupported.reason).expect("serializes"),
            serde_json::json!("unsupported")
        );
    }

    #[test]
    fn root_items_slice_matches_the_recorded_range() {
        let item = |name: &str| TransferItem {
            source: PathBuf::from(format!("C:\\src\\{name}")),
            destination: PathBuf::from(format!("D:\\dst\\{name}")),
            kind: ItemKind::File,
            size_bytes: 1,
            root_index: 0,
            action: ItemAction::Transfer,
            skip_reason: None,
            skip_detail: None,
            clear_first: None,
            existed: false,
        };
        let plan = TransferPlan {
            destination: PathBuf::from("D:\\dst"),
            operation: TransferOperation::Copy,
            conflict: ConflictStrategy::Skip,
            roots: vec![PlanRoot {
                source: PathBuf::from("C:\\src"),
                destination: PathBuf::from("D:\\dst\\src"),
                kind: ItemKind::Directory,
                action: ItemAction::Transfer,
                skip_reason: None,
                skip_detail: None,
                item_start: 1,
                item_count: 2,
                files: 2,
                bytes: 2,
                skipped_bytes: 0,
                clean: true,
            }],
            items: vec![item("root"), item("a"), item("b")],
            total_bytes: 2,
            total_files: 2,
            total_directories: 1,
            conflicts: 0,
            skipped_items: 0,
            skipped_bytes: 0,
            required_bytes: 2,
        };

        let items = plan.root_items(&plan.roots[0]);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].source, PathBuf::from("C:\\src\\a"));
        assert_eq!(items[1].source, PathBuf::from("C:\\src\\b"));
    }
}
