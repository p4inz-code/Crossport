/* ==========================================================================
 * Transfer engine
 * The queue and the life of a job. Planning lives in `plan`, the actual data
 * movement in `copy`, and this module owns everything around them: stable
 * ordering, a deliberate concurrency policy, cooperative pause/resume/cancel,
 * real progress accounting, and throttled progress events.
 *
 * Synchronization rules (the ones that keep this deadlock-free):
 *
 * - queue state is guarded by `EngineInner::state`; per-job status and counters
 *   by the job's own mutex;
 * - whenever both are taken, the engine lock is always taken first;
 * - a worker never takes the engine lock while holding a job lock, so control
 *   commands and running jobs cannot deadlock;
 * - pause and cancel are plain atomics the worker checks between chunks, so a
 *   control command never has to wait for I/O to return.
 *
 * Concurrency policy: one active job by default. Desktop transfers are
 * disk-bound, and a second concurrent job mostly buys seek contention and a
 * progress display that is harder to reason about. The engine takes the limit
 * as a parameter so the policy is visible, testable, and easy to revisit.
 * ========================================================================== */

pub mod conflict;
pub mod copy;
pub mod model;
pub mod plan;
pub mod safety;

pub use model::*;

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::errors::{AppError, AppResult};
use crate::verification::{FileVerification, VerificationLog, VerificationPolicy};
use copy::{Abort, JobBridge};

/// Tauri event carrying one [`TransferSnapshot`]. Every state change is
/// published on it, so the frontend never polls to stay current.
pub const TRANSFER_EVENT: &str = "transfer:update";

/// Jobs that may run at once. See the module comment for why this is one.
pub const DEFAULT_MAX_ACTIVE: usize = 1;

/// How often byte progress may be published. State changes are always
/// published immediately; byte updates are coalesced to this cadence, which
/// keeps the UI live at roughly eight updates a second without turning every
/// megabyte into an IPC round trip.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

/// How long an idle worker or a paused job waits before re-checking.
const IDLE_POLL: Duration = Duration::from_millis(250);

/// How often a running job's state may be written to durable storage.
///
/// Progress is published eight times a second; a durable write is worth far
/// less often than that, and every write replaces a file. Two seconds bounds
/// what a crash can lose to a couple of seconds of progress while keeping the
/// disk quiet during a long transfer.
const JOURNAL_INTERVAL: Duration = Duration::from_secs(2);

/// Speed is measured over this window, so the reported figure tracks current
/// throughput instead of an average taken over the whole job.
const SPEED_WINDOW: Duration = Duration::from_secs(4);

/// Most issues one job reports. Longer lists are truncated and flagged: a
/// per-file failure list is diagnostics, not an unbounded log.
const MAX_ISSUES: usize = 200;

/// Where the engine publishes progress.
///
/// The engine never talks to the windowing layer itself: the application
/// installs a publisher at startup that forwards snapshots as typed
/// `transfer:update` events. Without one — in tests, or any headless use —
/// publishing is a no-op, which is also what keeps the backend testable
/// without a running app.
///
pub trait TransferPublisher: Send + Sync {
    fn publish(&self, snapshot: &TransferSnapshot);
}

/// Where the engine hands durable state.
///
/// Like the publisher, this is an outbound port: the engine never opens a file
/// itself, so it stays testable, and an application without a journal (tests,
/// headless use) simply does not persist anything.
///
/// The contract matters more than the implementation:
///
/// - [`TransferJournal::record_live`] is called while a job is queued, running,
///   paused, or cancelling, so a crash leaves a record of work in flight. It may
///   be called again and again with newer state, and implementations must treat
///   each call as a replacement rather than an addition.
/// - [`TransferJournal::record_finished`] is called exactly once, after a job
///   reaches a terminal state, and it is called *before* the state it recorded
///   live is forgotten — so an interrupted-job record can never claim a job was
///   unfinished once its result is on disk.
/// - [`TransferJournal::forget`] releases the live state of a job that no longer
///   exists, and must never remove a finished record.
///
/// Implementations must not block for long: these calls happen on the worker
/// thread that is moving the data.
pub trait TransferJournal: Send + Sync {
    /// Records the current state of a job that has not finished.
    fn record_live(&self, snapshot: &TransferSnapshot);
    /// Records the final state of a job.
    fn record_finished(&self, snapshot: &TransferSnapshot);
    /// Releases the live state of a job that is gone.
    fn forget(&self, job_id: &str);
}

/// The transfer queue and its workers.
///
/// Cheap to clone: clones share one engine, which is what the command layer
/// needs to move it onto the blocking pool.
#[derive(Clone)]
pub struct TransferEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    state: Mutex<QueueState>,
    /// Wakes idle workers when work arrives.
    wake: Condvar,
    /// Set once the app is running so progress can be published. Absent in
    /// tests and in any context without a frontend, where publishing is a
    /// no-op instead of a failure.
    publisher: Mutex<Option<Arc<dyn TransferPublisher>>>,
    /// Set once durable storage is available. Absent in tests, where a job
    /// simply is not recorded anywhere.
    journal: Mutex<Option<Arc<dyn TransferJournal>>>,
    shutdown: AtomicBool,
    max_active: usize,
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }
}

struct QueueState {
    /// Jobs in the order they were accepted. Queue positions never change, so
    /// ordering is stable for the life of the process.
    jobs: Vec<Arc<JobSlot>>,
    /// Identifiers of jobs currently owned by a worker.
    active: HashMap<String, ()>,
    /// Monotonic counter behind job identifiers, so identifiers stay unique
    /// even when finished jobs are removed from the queue.
    next_id: u64,
}

/// One job: immutable once queued (its plan is the normalized record of the
/// request), plus the state its worker writes.
struct JobSlot {
    id: String,
    queued_at_ms: u64,
    plan: TransferPlan,
    runtime: Mutex<JobRuntime>,
    /// Cooperative control flags, checked by the worker between chunks.
    pause_requested: AtomicBool,
    cancel_requested: AtomicBool,
    /// Wakes a job that is parked while paused.
    resume_signal: Condvar,
}

/// Everything a job's runtime starts as, so a job can be created without
/// listing every field and the tests can describe only what they care about.
impl Default for JobRuntime {
    fn default() -> Self {
        Self {
            status: TransferStatus::Queued,
            counters: TransferCounters::default(),
            activity: TransferActivity::Transferring,
            verification: VerificationLog::default(),
            error: None,
            issues: Vec::new(),
            issues_truncated: false,
            started_ms: None,
            finished_ms: None,
            started_at: None,
            finished_instant: None,
            paused_at: None,
            paused_total: Duration::ZERO,
            last_emit: None,
            last_journal: None,
            samples: VecDeque::new(),
        }
    }
}

struct JobRuntime {
    status: TransferStatus,
    counters: TransferCounters,
    /// Whether the job is moving or checking the file it is on.
    activity: TransferActivity,
    verification: VerificationLog,
    error: Option<AppError>,
    issues: Vec<TransferIssue>,
    issues_truncated: bool,
    started_ms: Option<u64>,
    finished_ms: Option<u64>,
    started_at: Option<Instant>,
    finished_instant: Option<Instant>,
    paused_at: Option<Instant>,
    /// Total time spent paused, so speed and ETA are never diluted by it.
    paused_total: Duration,
    last_emit: Option<Instant>,
    /// Last time live state was written to the journal. Separate from
    /// [`JobRuntime::last_emit`] because progress is published far more often
    /// than a durable write is worth.
    last_journal: Option<Instant>,
    /// Recent (time, bytes) samples used for the current speed.
    samples: VecDeque<(Instant, u64)>,
}

impl TransferEngine {
    /// Creates an engine with its workers already waiting for work.
    pub fn new(max_active: usize) -> Self {
        let max_active = max_active.max(1);
        let inner = Arc::new(EngineInner {
            state: Mutex::new(QueueState {
                jobs: Vec::new(),
                active: HashMap::new(),
                next_id: 0,
            }),
            wake: Condvar::new(),
            publisher: Mutex::new(None),
            journal: Mutex::new(None),
            shutdown: AtomicBool::new(false),
            max_active,
        });

        for index in 0..max_active {
            let weak = Arc::downgrade(&inner);
            std::thread::Builder::new()
                .name(format!("crossport-transfer-{index}"))
                .spawn(move || worker_loop(weak))
                .expect("the transfer engine must be able to start its worker thread");
        }

        Self { inner }
    }

    /// Installs the progress publisher. Called once during application setup;
    /// before that, and wherever no publisher is installed, every publish is a
    /// no-op.
    pub fn attach(&self, publisher: Arc<dyn TransferPublisher>) {
        *lock(&self.inner.publisher) = Some(publisher);
    }

    /// Installs the durable-state sink. Called once during application setup;
    /// without one, jobs live and die entirely in memory.
    pub fn attach_journal(&self, journal: Arc<dyn TransferJournal>) {
        *lock(&self.inner.journal) = Some(journal);
    }

    /// What the engine will verify with when a request does not say.
    ///
    /// The setting is read at the command boundary and folded into the request,
    /// so this is only the fallback the engine itself honours.
    pub fn default_verification_policy(&self) -> VerificationPolicy {
        VerificationPolicy::default()
    }

    /// Plans a request and queues it, rejecting anything unsafe before it
    /// becomes a job.
    ///
    /// Blocking: this walks the source trees. The command layer runs it on the
    /// blocking pool.
    pub fn enqueue_request(&self, request: TransferRequest) -> AppResult<TransferSnapshot> {
        let plan = plan::plan_for_start(&request)?;
        self.enqueue(plan)
    }

    /// Hands an already-planned job to the queue.
    ///
    /// The plan carries the normalized sources, the resolved destinations, and
    /// the operation and conflict strategy, so it is the authoritative record
    /// of what the user asked for.
    pub fn enqueue(&self, plan: TransferPlan) -> AppResult<TransferSnapshot> {
        let mut state = lock(&self.inner.state);

        // The queue position is part of the identifier, and the counter is
        // never reused: a removed job's identifier cannot come back for a
        // different job, so a stale command can never hit the wrong transfer.
        state.next_id += 1;
        let id = format!("transfer-{}-{}", now_ms(), state.next_id);
        let slot = Arc::new(JobSlot {
            id,
            queued_at_ms: now_ms(),
            runtime: Mutex::new(JobRuntime {
                counters: TransferCounters {
                    total_bytes: plan.total_bytes,
                    total_files: plan.total_files,
                    total_directories: plan.total_directories,
                    ..TransferCounters::default()
                },
                // The plan already carries the policy, and the log counts the
                // files the plan intends to verify.
                verification: {
                    let mut log = VerificationLog::new(plan.verification);
                    log.set_planned_files(plan.total_files);
                    log
                },
                ..JobRuntime::default()
            }),
            pause_requested: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
            resume_signal: Condvar::new(),
            plan,
        });

        // A job that is queued but not yet claimed is still work in flight: a
        // crash before it starts must leave a record of it. The record is
        // written *before* the job becomes claimable, so this write can never
        // land after the job has already finished and been cleared — which
        // would leave a finished job looking interrupted after a restart.
        // The queue lock is held across the write to keep that ordering exact;
        // it is one small document per accepted request.
        let snapshot = snapshot_of(&slot);
        self.inner.journal_live(&snapshot);

        state.jobs.push(Arc::clone(&slot));
        drop(state);

        self.inner.wake.notify_all();
        Ok(snapshot)
    }

    /// Every job in queue order.
    pub fn snapshots(&self) -> Vec<TransferSnapshot> {
        let jobs: Vec<Arc<JobSlot>> = lock(&self.inner.state).jobs.clone();
        jobs.iter().map(|job| snapshot_of(job)).collect()
    }

    /// One job by identifier.
    pub fn snapshot(&self, id: &str) -> AppResult<TransferSnapshot> {
        let slot = find_job(&lock(&self.inner.state), id)?;
        Ok(snapshot_of(&slot))
    }

    /// Parks a running job, or holds a queued one back until it is resumed.
    ///
    /// A paused job keeps its state: the file being written stays incomplete on
    /// disk until the job continues or is cancelled, and nothing is committed
    /// while it is parked.
    pub fn pause(&self, id: &str) -> AppResult<TransferSnapshot> {
        let state = lock(&self.inner.state);
        let slot = find_job(&state, id)?;

        {
            let mut runtime = lock(&slot.runtime);
            match runtime.status {
                TransferStatus::Paused => {}
                TransferStatus::Queued | TransferStatus::Preparing | TransferStatus::Running => {
                    slot.pause_requested.store(true, Ordering::SeqCst);
                    runtime.status = TransferStatus::Paused;
                    runtime.paused_at = Some(Instant::now());
                    // A pause resets the speed window: the gap while parked is
                    // not throughput, and counting it would report a speed the
                    // transfer never had.
                    runtime.samples.clear();
                }
                other => {
                    return Err(AppError::InvalidInput(format!(
                        "transfer '{id}' is already {} and cannot be paused",
                        other.as_str()
                    )))
                }
            }
        }

        drop(state);
        slot.resume_signal.notify_all();
        let snapshot = snapshot_of(&slot);
        self.inner.emit(&snapshot);
        Ok(snapshot)
    }

    /// Continues a paused job.
    ///
    /// A job that had already started keeps writing the same temporary file
    /// from the exact offset it stopped at: the handle, the byte counters, and
    /// the partial file are all still valid, so nothing is re-read and nothing
    /// is corrupted. The destination is still untouched — the file is only
    /// committed once it is complete.
    pub fn resume(&self, id: &str) -> AppResult<TransferSnapshot> {
        let state = lock(&self.inner.state);
        let slot = find_job(&state, id)?;
        let claimed = state.active.contains_key(&slot.id);

        {
            let mut runtime = lock(&slot.runtime);
            if runtime.status != TransferStatus::Paused {
                return Err(AppError::InvalidInput(format!(
                    "transfer '{id}' is {} and is not paused",
                    runtime.status.as_str()
                )));
            }

            slot.pause_requested.store(false, Ordering::SeqCst);
            if let Some(paused_at) = runtime.paused_at.take() {
                runtime.paused_total = runtime.paused_total.saturating_add(paused_at.elapsed());
            }
            runtime.samples.clear();

            // A job that is still waiting for a worker goes back to the queue;
            // one a worker already owns just continues where it stopped.
            runtime.status = if claimed {
                TransferStatus::Running
            } else {
                TransferStatus::Queued
            };
        }

        drop(state);
        slot.resume_signal.notify_all();
        self.inner.wake.notify_all();
        let snapshot = snapshot_of(&slot);
        self.inner.emit(&snapshot);
        Ok(snapshot)
    }

    /// Stops a job.
    ///
    /// A queued job is cancelled outright and never touches the filesystem. A
    /// running one is asked to stop and reports `cancelling` until its worker
    /// has unwound, discarded its partial file, and released its resources.
    pub fn cancel(&self, id: &str) -> AppResult<TransferSnapshot> {
        let state = lock(&self.inner.state);
        let slot = find_job(&state, id)?;
        let claimed = state.active.contains_key(&slot.id);

        slot.cancel_requested.store(true, Ordering::SeqCst);
        {
            let mut runtime = lock(&slot.runtime);
            if runtime.status.is_terminal() {
                return Err(AppError::InvalidInput(format!(
                    "transfer '{id}' is already {}",
                    runtime.status.as_str()
                )));
            }

            runtime.status = if claimed {
                TransferStatus::Cancelling
            } else {
                // Nothing was opened, so there is nothing to release.
                runtime.finished_ms = Some(now_ms());
                TransferStatus::Cancelled
            };
            runtime.counters.current_file = None;
        }

        drop(state);
        // A parked job has to wake up to notice it was cancelled.
        slot.resume_signal.notify_all();
        let snapshot = snapshot_of(&slot);
        self.inner.emit(&snapshot);
        Ok(snapshot)
    }

    /// Drops a finished job from the queue.
    pub fn remove(&self, id: &str) -> AppResult<()> {
        let mut state = lock(&self.inner.state);
        let slot = find_job(&state, id)?;

        if !lock(&slot.runtime).status.is_terminal() {
            return Err(AppError::InvalidInput(format!(
                "transfer '{id}' is still running; cancel it before removing it"
            )));
        }

        state.jobs.retain(|job| job.id != id);
        drop(state);

        // The finished record stays; only the live state is released.
        self.inner.journal_forget(id);
        Ok(())
    }

    /// Drops every finished job. Live jobs are never touched.
    pub fn clear_finished(&self) -> usize {
        // Which jobs disappeared is decided under the queue lock, so the answer
        // and the removal cannot disagree. Their live state is released after
        // the lock is dropped, because a journal write is not something the
        // queue lock should be held across.
        let cleared: Vec<String> = {
            let mut state = lock(&self.inner.state);
            let before: Vec<String> = state.jobs.iter().map(|job| job.id.clone()).collect();
            state
                .jobs
                .retain(|job| !lock(&job.runtime).status.is_terminal());
            let remaining: std::collections::HashSet<&String> =
                state.jobs.iter().map(|job| &job.id).collect();
            before
                .into_iter()
                .filter(|id| !remaining.contains(id))
                .collect()
        };

        for id in &cleared {
            self.inner.journal_forget(id);
        }
        cleared.len()
    }

    /// Stops accepting work and releases the workers.
    ///
    /// A job that is running is asked to stop; a parked one is woken so it can
    /// unwind. Used at shutdown and by tests that must not leave threads
    /// behind.
    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::SeqCst);
        self.inner.wake.notify_all();
        for job in lock(&self.inner.state).jobs.iter() {
            job.cancel_requested.store(true, Ordering::SeqCst);
            job.resume_signal.notify_all();
        }
    }
}

impl Default for TransferEngine {
    /// The engine the application runs with: the deliberate concurrency policy,
    /// not an arbitrary number.
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ACTIVE)
    }
}

impl std::fmt::Debug for TransferEngine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = lock(&self.inner.state).jobs.len();
        formatter
            .debug_struct("TransferEngine")
            .field("jobs", &count)
            .field("max_active", &self.inner.max_active)
            .finish()
    }
}

impl EngineInner {
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Publishes one snapshot. A missing publisher (tests, headless use) never
    /// breaks a transfer: the state itself is already updated and queryable
    /// through the queue commands.
    fn emit(&self, snapshot: &TransferSnapshot) {
        let publisher = lock(&self.publisher).clone();
        if let Some(publisher) = publisher {
            publisher.publish(snapshot);
        }
    }

    /// Records a live job. A journal failure is logged, never fatal: losing the
    /// ability to recover a job is not a reason to stop moving the data.
    fn journal_live(&self, snapshot: &TransferSnapshot) {
        let journal = lock(&self.journal).clone();
        if let Some(journal) = journal {
            journal.record_live(snapshot);
        }
    }

    /// Records a finished job. Called before the job's live state is released.
    fn journal_finished(&self, snapshot: &TransferSnapshot) {
        let journal = lock(&self.journal).clone();
        if let Some(journal) = journal {
            journal.record_finished(snapshot);
        }
    }

    fn journal_forget(&self, job_id: &str) {
        let journal = lock(&self.journal).clone();
        if let Some(journal) = journal {
            journal.forget(job_id);
        }
    }

    /// Whether enough time has passed to write live state again. `true` the
    /// first time, so a job's first progress is always durable.
    fn journal_due(runtime: &mut JobRuntime, now: Instant) -> bool {
        let due = runtime.last_journal.map_or(true, |last| {
            now.saturating_duration_since(last) >= JOURNAL_INTERVAL
        });
        if due {
            runtime.last_journal = Some(now);
        }
        due
    }
}

/// What the execution layer asks of the job it belongs to.
struct TransferContext {
    inner: Arc<EngineInner>,
    slot: Arc<JobSlot>,
}

impl TransferContext {
    fn start(&self) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.status = TransferStatus::Running;
            runtime.activity = TransferActivity::Transferring;
            runtime.started_at = Some(Instant::now());
            runtime.started_ms = Some(now_ms());
            runtime.last_emit = None;
            // A job that has just started is durable immediately.
            runtime.last_journal = None;
        }
        self.emit_now();
    }

    fn finish(&self, result: Result<(), Abort>) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.finished_ms = Some(now_ms());
            runtime.finished_instant = Some(Instant::now());
            runtime.counters.current_file = None;
            runtime.counters.current_file_bytes = 0;
            runtime.counters.current_file_total_bytes = 0;
            runtime.activity = TransferActivity::Transferring;
            runtime.verification.finish();

            match result {
                Ok(()) => {
                    runtime.status = if runtime.counters.failed_items > 0 {
                        TransferStatus::Failed
                    } else {
                        TransferStatus::Completed
                    };
                }
                Err(Abort::Cancelled) => runtime.status = TransferStatus::Cancelled,
                Err(Abort::Failed(error)) => {
                    runtime.error = Some(error);
                    runtime.status = TransferStatus::Failed;
                }
            }

            // A job whose verification did not hold is not a job that worked,
            // no matter how many bytes moved. The item failures already fail
            // the job; this is the job-level statement of why.
            if runtime.status == TransferStatus::Completed && runtime.verification.has_failures() {
                runtime.status = TransferStatus::Failed;
            }

            if runtime.status == TransferStatus::Failed && runtime.error.is_none() {
                let verification = runtime.verification.summary(true);
                runtime.error = Some(if verification.status.is_failure() {
                    AppError::VerificationFailed(format!(
                        "{} of {} files did not verify ({}); {} of {} items did not transfer",
                        verification
                            .mismatched_files
                            .saturating_add(verification.failed_files),
                        verification.planned_files,
                        verification.verdict,
                        runtime.counters.failed_items,
                        runtime
                            .counters
                            .total_files
                            .saturating_add(runtime.counters.total_directories)
                    ))
                } else {
                    AppError::TransferFailed(format!(
                        "{} of {} items did not transfer",
                        runtime.counters.failed_items,
                        runtime
                            .counters
                            .total_files
                            .saturating_add(runtime.counters.total_directories)
                    ))
                });
            }
        }

        {
            let mut state = lock(&self.inner.state);
            state.active.remove(&self.slot.id);
        }

        // The finished record is written before anything is published, so a
        // crash between the two can never make a finished job look unfinished,
        // and a client that reacts to the completion event cannot observe a job
        // that finished but is not yet in history.
        self.journal_finished();
        self.emit_now();
        self.inner.wake.notify_all();
    }

    /// Blocks while the job is paused, and reports cancellation.
    fn wait_while_paused(&self) -> Result<(), Abort> {
        let mut runtime = lock(&self.slot.runtime);

        while self.slot.pause_requested.load(Ordering::SeqCst) {
            if self.slot.cancel_requested.load(Ordering::SeqCst) || self.inner.is_shutdown() {
                return Err(Abort::Cancelled);
            }
            // A poisoned lock is recovered rather than propagated: these
            // counters are diagnostics, and one panicking job must not stop
            // the user from cancelling another.
            runtime = match self.slot.resume_signal.wait_timeout(runtime, IDLE_POLL) {
                Ok((guard, _timeout)) => guard,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }

        if self.slot.cancel_requested.load(Ordering::SeqCst) || self.inner.is_shutdown() {
            return Err(Abort::Cancelled);
        }

        Ok(())
    }

    /// Updates state and publishes it if the progress cadence allows.
    fn touch(&self) {
        let now = Instant::now();
        let snapshot = {
            let mut runtime = lock(&self.slot.runtime);
            let due = runtime.last_emit.map_or(true, |last| {
                now.saturating_duration_since(last) >= PROGRESS_INTERVAL
            });
            if !due {
                return;
            }
            runtime.last_emit = Some(now);
            sample_speed(&mut runtime, now);
            snapshot_locked(&self.slot, &runtime, now)
        };
        self.inner.emit(&snapshot);
        self.journal_live();
    }

    /// Publishes immediately: used for state changes and failures, where
    /// waiting for the cadence would make the UI feel unresponsive.
    fn emit_now(&self) {
        let now = Instant::now();
        let snapshot = {
            let mut runtime = lock(&self.slot.runtime);
            runtime.last_emit = Some(now);
            sample_speed(&mut runtime, now);
            snapshot_locked(&self.slot, &runtime, now)
        };
        self.inner.emit(&snapshot);
        self.journal_live();
    }

    /// Writes the job's live state to durable storage if the cadence allows.
    ///
    /// Skipped for terminal jobs: their record is written once by
    /// [`TransferContext::finish`], and rewriting it as "live" afterwards would
    /// be the one thing recovery must never see.
    fn journal_live(&self) {
        let now = Instant::now();
        let snapshot = {
            let mut runtime = lock(&self.slot.runtime);
            if runtime.status.is_terminal() || !EngineInner::journal_due(&mut runtime, now) {
                return;
            }
            snapshot_locked(&self.slot, &runtime, now)
        };
        self.inner.journal_live(&snapshot);
    }

    /// Writes the job's final state, once.
    fn journal_finished(&self) {
        let now = Instant::now();
        let snapshot = {
            let mut runtime = lock(&self.slot.runtime);
            runtime.finished_instant.get_or_insert(now);
            snapshot_locked(&self.slot, &runtime, now)
        };
        self.inner.journal_finished(&snapshot);
    }
}

impl JobBridge for TransferContext {
    fn job_id(&self) -> &str {
        &self.slot.id
    }

    fn checkpoint(&self) -> Result<(), Abort> {
        self.wait_while_paused()
    }

    fn item_verifying(&self, item: &TransferItem) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.activity = TransferActivity::Verifying;
            runtime.counters.current_file = Some(item.destination.display().to_string());
        }
        self.touch();
    }

    fn item_verified(&self, verification: &FileVerification) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.activity = TransferActivity::Transferring;
            runtime.verification.record(verification);
        }
        self.touch();
    }

    fn item_started(&self, item: &TransferItem) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.counters.current_file = Some(item.source.display().to_string());
            runtime.counters.current_file_bytes = 0;
            runtime.counters.current_file_total_bytes = item.size_bytes;
        }
        self.touch();
    }

    fn record_bytes(&self, bytes: u64) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.counters.transferred_bytes =
                runtime.counters.transferred_bytes.saturating_add(bytes);
            runtime.counters.current_file_bytes =
                runtime.counters.current_file_bytes.saturating_add(bytes);
        }
        self.touch();
    }

    fn item_completed(&self, item: &TransferItem) {
        {
            let mut runtime = lock(&self.slot.runtime);
            match item.kind {
                ItemKind::File => {
                    runtime.counters.completed_files =
                        runtime.counters.completed_files.saturating_add(1);
                    runtime.counters.current_file = None;
                    runtime.counters.current_file_bytes = 0;
                    runtime.counters.current_file_total_bytes = 0;
                }
                ItemKind::Directory => {
                    runtime.counters.completed_directories =
                        runtime.counters.completed_directories.saturating_add(1);
                }
            }
        }
        self.touch();
    }

    fn item_skipped(&self, item: &TransferItem) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.counters.skipped_items = runtime.counters.skipped_items.saturating_add(1);
            if item.kind == ItemKind::File {
                runtime.counters.skipped_bytes = runtime
                    .counters
                    .skipped_bytes
                    .saturating_add(item.size_bytes);
            }
            push_issue(
                &mut runtime,
                TransferIssue {
                    path: item.source.display().to_string(),
                    reason: item.skip_reason.unwrap_or(TransferIssueReason::Skipped),
                    error: None,
                    detail: item.skip_detail.clone(),
                },
            );
        }
        self.touch();
    }

    fn item_failed(&self, path: &Path, error: AppError) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.counters.failed_items = runtime.counters.failed_items.saturating_add(1);
            push_issue(
                &mut runtime,
                TransferIssue::failed(path.display().to_string(), error),
            );
        }
        self.emit_now();
    }

    fn root_skipped(&self, root: &PlanRoot) {
        {
            let mut runtime = lock(&self.slot.runtime);
            // The whole root is left alone, so every item it was planned as
            // counts as skipped. That is what makes the reported skip count
            // match the request instead of only its top-level entry.
            runtime.counters.skipped_items = runtime
                .counters
                .skipped_items
                .saturating_add(u64::try_from(root.item_count).unwrap_or(u64::MAX));
            runtime.counters.skipped_bytes = runtime
                .counters
                .skipped_bytes
                .saturating_add(root.skipped_bytes);
            push_issue(
                &mut runtime,
                TransferIssue {
                    path: root.source.display().to_string(),
                    reason: root.skip_reason.unwrap_or(TransferIssueReason::Skipped),
                    error: None,
                    detail: root.skip_detail.clone(),
                },
            );
        }
        self.touch();
    }

    fn root_moved(&self, root: &PlanRoot) {
        {
            let mut runtime = lock(&self.slot.runtime);
            runtime.counters.transferred_bytes = runtime
                .counters
                .transferred_bytes
                .saturating_add(root.bytes);
            runtime.counters.completed_files =
                runtime.counters.completed_files.saturating_add(root.files);
            runtime.counters.completed_directories = runtime
                .counters
                .completed_directories
                .saturating_add(directories_in(root));
        }
        // A rename completes in one step: publishing immediately is the only
        // way the UI can show that it happened.
        self.emit_now();
    }

    fn root_retained(&self, root: &PlanRoot, detail: &str) {
        {
            let mut runtime = lock(&self.slot.runtime);
            push_issue(
                &mut runtime,
                TransferIssue::unsupported(root.source.display().to_string(), detail),
            );
        }
        self.touch();
    }
}

/// Directory items inside one root: everything that is not a planned file.
fn directories_in(root: &PlanRoot) -> u64 {
    u64::try_from(root.item_count.saturating_sub(root.files as usize)).unwrap_or(u64::MAX)
}

fn push_issue(runtime: &mut JobRuntime, issue: TransferIssue) {
    if runtime.issues.len() >= MAX_ISSUES {
        runtime.issues_truncated = true;
        return;
    }
    runtime.issues.push(issue);
}

/// Long-lived worker: takes one job at a time, in queue order.
fn worker_loop(weak: Weak<EngineInner>) {
    loop {
        let Some(inner) = weak.upgrade() else {
            // The engine is gone; nothing left to run.
            return;
        };

        let Some(slot) = claim_next(&inner, &weak) else {
            return;
        };

        run_job(&inner, slot);
    }
}

/// Claims the oldest queued job, or returns `None` when the engine shut down.
fn claim_next(inner: &Arc<EngineInner>, weak: &Weak<EngineInner>) -> Option<Arc<JobSlot>> {
    let mut state = lock(&inner.state);

    loop {
        if inner.is_shutdown() {
            return None;
        }

        if state.active.len() < inner.max_active {
            let next = state.jobs.iter().find(|job| {
                let runtime = lock(&job.runtime);
                runtime.status == TransferStatus::Queued
            });

            if let Some(slot) = next.cloned() {
                {
                    let mut runtime = lock(&slot.runtime);
                    runtime.status = TransferStatus::Preparing;
                }
                state.active.insert(slot.id.clone(), ());
                drop(state);

                let context = TransferContext {
                    inner: Arc::clone(inner),
                    slot: Arc::clone(&slot),
                };
                context.emit_now();
                return Some(slot);
            }
        }

        state = match inner.wake.wait_timeout(state, IDLE_POLL) {
            Ok((guard, _timeout)) => guard,
            Err(poisoned) => poisoned.into_inner().0,
        };

        // The engine was dropped while this worker waited: nothing left to run.
        weak.upgrade()?;
    }
}

/// Runs one claimed job to a terminal state.
///
/// The work runs inside `catch_unwind` so a panic in one job becomes a failed
/// job instead of a dead worker and a stuck queue.
fn run_job(inner: &Arc<EngineInner>, slot: Arc<JobSlot>) {
    let context = TransferContext {
        inner: Arc::clone(inner),
        slot: Arc::clone(&slot),
    };
    context.start();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        copy::execute(&slot.plan, &context)
    }));

    let result = match outcome {
        Ok(result) => result,
        Err(payload) => Err(Abort::Failed(AppError::Internal(format!(
            "the transfer worker panicked: {}",
            panic_message(&payload)
        )))),
    };

    context.finish(result);
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic".to_string()
}

fn find_job(state: &QueueState, id: &str) -> AppResult<Arc<JobSlot>> {
    state
        .jobs
        .iter()
        .find(|job| job.id == id)
        .cloned()
        .ok_or_else(|| AppError::TransferNotFound(id.to_string()))
}

fn snapshot_of(slot: &JobSlot) -> TransferSnapshot {
    let runtime = lock(&slot.runtime);
    snapshot_locked(slot, &runtime, Instant::now())
}

fn snapshot_locked(slot: &JobSlot, runtime: &JobRuntime, now: Instant) -> TransferSnapshot {
    TransferSnapshot {
        id: slot.id.clone(),
        operation: slot.plan.operation,
        conflict: slot.plan.conflict,
        status: runtime.status,
        verification: runtime.verification.summary(runtime.status.is_terminal()),
        sources: slot
            .plan
            .roots
            .iter()
            .map(|root| root.source.display().to_string())
            .collect(),
        destination: slot.plan.destination.display().to_string(),
        progress: runtime
            .counters
            .to_progress(timing_of(runtime, now), runtime.activity),
        error: runtime.error.clone(),
        issues: runtime.issues.clone(),
        issues_truncated: runtime.issues_truncated,
        queued_at_ms: slot.queued_at_ms,
        started_at_ms: runtime.started_ms,
        finished_at_ms: runtime.finished_ms,
    }
}

/// Derives speed, ETA, and elapsed time from what actually happened: the job's
/// own clock, paused time excluded, and samples of real byte counts.
fn timing_of(runtime: &JobRuntime, now: Instant) -> TransferTiming {
    let elapsed = runtime.active_elapsed(now);
    let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);

    let bytes_per_second = window_speed(&runtime.samples);
    let average_bytes_per_second = if elapsed_ms > 0 {
        runtime
            .counters
            .transferred_bytes
            .saturating_mul(1_000)
            .checked_div(elapsed_ms)
            .unwrap_or(0)
    } else {
        0
    };

    let rate = if bytes_per_second > 0 {
        bytes_per_second
    } else {
        average_bytes_per_second
    };
    let remaining = runtime
        .counters
        .total_bytes
        .saturating_sub(runtime.counters.transferred_bytes);
    let eta_seconds = if runtime.status == TransferStatus::Running {
        estimate_eta_seconds(remaining, runtime.counters.total_bytes, rate)
    } else {
        None
    };

    TransferTiming {
        elapsed_ms,
        bytes_per_second,
        average_bytes_per_second,
        eta_seconds,
    }
}

impl JobRuntime {
    /// Time the job spent working, with paused time removed.
    fn active_elapsed(&self, now: Instant) -> Duration {
        let Some(started_at) = self.started_at else {
            return Duration::ZERO;
        };
        let end = self.finished_instant.unwrap_or(now);
        let wall = end.saturating_duration_since(started_at);
        let paused = match self.paused_at {
            Some(paused_at) if self.finished_instant.is_none() => self
                .paused_total
                .saturating_add(now.saturating_duration_since(paused_at)),
            _ => self.paused_total,
        };
        wall.saturating_sub(paused)
    }
}

/// Records one speed sample and forgets samples older than the window, keeping
/// enough history to compute a rate.
fn sample_speed(runtime: &mut JobRuntime, now: Instant) {
    runtime
        .samples
        .push_back((now, runtime.counters.transferred_bytes));

    while runtime.samples.len() > 2 {
        let Some(front) = runtime.samples.front() else {
            break;
        };
        if now.saturating_duration_since(front.0) > SPEED_WINDOW {
            runtime.samples.pop_front();
        } else {
            break;
        }
    }
}

/// Bytes per second across the recorded window. `0` until two samples exist —
/// a speed that has not been measured is not invented.
fn window_speed(samples: &VecDeque<(Instant, u64)>) -> u64 {
    if samples.len() < 2 {
        return 0;
    }
    let Some((first_at, first_bytes)) = samples.front() else {
        return 0;
    };
    let Some((last_at, last_bytes)) = samples.back() else {
        return 0;
    };

    let micros = last_at.saturating_duration_since(*first_at).as_micros();
    if micros == 0 || last_bytes <= first_bytes {
        return 0;
    }

    let delta = u128::from(last_bytes - first_bytes);
    u64::try_from(delta.saturating_mul(1_000_000) / micros).unwrap_or(u64::MAX)
}

/// Never panics on a poisoned lock: a job's own counters are the only thing
/// such a lock protects, and a panic in one job must not take the queue down.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod sanity;
#[cfg(test)]
mod tests;
