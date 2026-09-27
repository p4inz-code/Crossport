/* ==========================================================================
 * Transfer execution
 * Where data actually moves. Three rules shape everything here:
 *
 * 1. Nothing is ever held in memory. Files are streamed through one reused
 *    buffer in bounded chunks, so a 40 GiB file and a 4 KiB file cost the same
 *    resident memory.
 * 2. A destination file only ever appears complete. Bytes are written to a
 *    temporary file beside the destination and renamed into place as the last
 *    step, so a cancelled, failed, or crashed transfer cannot leave a
 *    half-written file where a complete one used to be — and an existing file
 *    is never touched until the new one is ready.
 * 3. A move is only called a move when it really moved. A same-volume move is
 *    a single atomic rename; a cross-volume move copies every item first and
 *    removes the source only after the whole root succeeded and every file's
 *    byte count matched what the plan measured.
 *
 * Item failures are isolated: one unreadable file does not cancel the rest of
 * the job, it is reported and the transfer continues.
 * ========================================================================== */

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::errors::AppError;
use crate::platform::drives;
use crate::transfer::conflict;
use crate::transfer::model::{
    ConflictStrategy, ItemAction, ItemKind, PlanRoot, TransferIssueReason, TransferItem,
    TransferOperation, TransferPlan,
};
use crate::transfer::safety;
use crate::verification::{
    self, FileVerification, StreamHasher, VerificationAborted, VerificationPolicy,
};

/// Size of the buffer one transfer streams through.
///
/// 1 MiB is the deliberate middle ground: large enough that the syscall and
/// allocation cost disappears against real I/O, small enough that a job holds
/// one megabyte — not the size of the file — and that a pause request is
/// honoured within about a megabyte of progress on any device.
pub const COPY_BUFFER_BYTES: usize = 1024 * 1024;

/// Why a job stopped before it finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Abort {
    /// The user cancelled, or the engine is shutting down.
    Cancelled,
    /// The job could not continue at all.
    Failed(AppError),
}

/// Which side of the copy failed, so the error keeps the right category
/// (`path_not_found` for a vanished source, `disk_full` for a full
/// destination).
#[derive(Debug)]
pub(crate) enum StreamError {
    Read(io::Error),
    Write(io::Error),
}

/// What the execution layer is allowed to ask of the job it belongs to.
///
/// Keeping this a trait means the copy engine is testable without a queue: the
/// tests drive it with a bridge that records what happened instead of talking
/// to a running engine.
pub(crate) trait JobBridge: Send + Sync {
    /// Stable identifier of the job, used to name temporary files.
    fn job_id(&self) -> &str;
    /// Blocks while the job is paused. Errors only when the job must stop.
    fn checkpoint(&self) -> Result<(), Abort>;
    /// The next file is about to be read.
    fn item_started(&self, item: &TransferItem);
    /// Bytes were written to the destination.
    fn record_bytes(&self, bytes: u64);
    /// Verification of one item is about to start, so the job can say what it
    /// is doing while its bytes are being checked.
    fn item_verifying(&self, item: &TransferItem);
    /// One item's verification result.
    fn item_verified(&self, verification: &FileVerification);
    /// An item finished successfully.
    fn item_completed(&self, item: &TransferItem);
    /// An item was left alone by the plan.
    fn item_skipped(&self, item: &TransferItem);
    /// An item failed; the job continues with the remaining items.
    fn item_failed(&self, path: &Path, error: AppError);
    /// A whole root was left alone.
    fn root_skipped(&self, root: &PlanRoot);
    /// A whole root was renamed into place by a same-volume move.
    fn root_moved(&self, root: &PlanRoot);
    /// A move finished but the source was kept, with the reason why.
    fn root_retained(&self, root: &PlanRoot, detail: &str);
}

/// Receives bounded-chunk progress from [`stream_copy`].
trait CopyObserver {
    /// Called before each read. Returning `false` stops the copy.
    fn before_chunk(&mut self) -> bool;
    /// Called after each written chunk.
    fn after_chunk(&mut self, bytes: u64);
}

/// Result of one streamed copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CopyOutcome {
    bytes: u64,
    /// True when the observer stopped it (pause turned into a cancel, or a
    /// shutdown); the destination is incomplete and must not be committed.
    aborted: bool,
}

/// Streams `reader` into `writer` through `buffer`.
///
/// The function never allocates: it reuses the caller's buffer and never holds
/// more than `buffer.len()` bytes at a time, which is what makes memory usage
/// independent of file size.
fn stream_copy<R: Read, W: Write, O: CopyObserver>(
    reader: &mut R,
    writer: &mut W,
    buffer: &mut [u8],
    observer: &mut O,
    mut hasher: Option<&mut StreamHasher>,
) -> Result<CopyOutcome, StreamError> {
    let mut bytes: u64 = 0;

    loop {
        if !observer.before_chunk() {
            return Ok(CopyOutcome {
                bytes,
                aborted: true,
            });
        }

        let read = match reader.read(buffer) {
            Ok(read) => read,
            // A read that a signal interrupted is retried, exactly as
            // `Read::read_to_end` would.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(StreamError::Read(error)),
        };
        if read == 0 {
            break;
        }

        writer
            .write_all(&buffer[..read])
            .map_err(StreamError::Write)?;

        // The hash is of the bytes that were read from the source, fed as they
        // pass through. Verifying a source therefore costs no second read of
        // it, which is the difference between a checksum policy that is usable
        // and one that doubles every transfer's read cost.
        if let Some(hasher) = hasher.as_deref_mut() {
            hasher.update(&buffer[..read]);
        }

        bytes = bytes
            .checked_add(read as u64)
            .ok_or_else(|| StreamError::Write(io::Error::other("byte counter overflowed")))?;
        observer.after_chunk(read as u64);
    }

    Ok(CopyOutcome {
        bytes,
        aborted: false,
    })
}

/// Runs a planned transfer to completion.
///
/// Returns `Ok(())` when every item was processed, even if individual items
/// failed — those are reported through the bridge and the job's own counters.
pub(crate) fn execute(plan: &TransferPlan, job: &dyn JobBridge) -> Result<(), Abort> {
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut created: Vec<PathBuf> = Vec::new();

    let outcome = run(plan, job, &mut buffer, &mut created);
    if outcome.is_err() {
        // Roll back only what this job created, deepest first, empty only.
        safety::clean_up_created_directories(&created);
    }
    outcome
}

fn run(
    plan: &TransferPlan,
    job: &dyn JobBridge,
    buffer: &mut [u8],
    created: &mut Vec<PathBuf>,
) -> Result<(), Abort> {
    for root in &plan.roots {
        job.checkpoint()?;

        if root.action == ItemAction::Skip {
            job.root_skipped(root);
            continue;
        }

        let items = plan.root_items(root);
        let mut failures: usize = 0;

        if can_rename_wholesale(plan, root) {
            // Same volume, nothing in the way: one atomic rename, no bytes
            // written, no partial state possible. The source is gone because
            // it *is* the destination now, so there is nothing left to remove.
            match rename_root(root) {
                Ok(()) => job.root_moved(root),
                Err(Abort::Cancelled) => return Err(Abort::Cancelled),
                Err(Abort::Failed(error)) => job.item_failed(&root.source, error),
            }
            // A successful rename *is* the destination, and a failed one left
            // the source exactly where it was: either way there is nothing
            // left to remove.
            continue;
        }

        {
            for (offset, item) in items.iter().enumerate() {
                job.checkpoint()?;

                if item.action == ItemAction::Skip {
                    job.item_skipped(item);
                    continue;
                }

                let item_index = root.item_start + offset;
                let outcome = match item.kind {
                    ItemKind::Directory => transfer_directory(item, created),
                    ItemKind::File => transfer_file(plan, job, item, item_index, buffer, created),
                };

                match outcome {
                    Ok(()) => job.item_completed(item),
                    Err(Abort::Cancelled) => return Err(Abort::Cancelled),
                    Err(Abort::Failed(error)) => {
                        job.item_failed(&item.destination, error);
                        failures += 1;
                    }
                }
            }
        }

        if plan.operation == TransferOperation::Move {
            finish_move(job, root, items, failures);
        }
    }

    Ok(())
}

/// Whether a whole root can be moved with a single rename.
fn can_rename_wholesale(plan: &TransferPlan, root: &PlanRoot) -> bool {
    plan.operation == TransferOperation::Move
        && root.clean
        && drives::same_volume(&root.source, &root.destination)
}

/// Removes the source of a move, or explains why it was kept.
///
/// The source is only ever removed when the whole root transferred and nothing
/// in it was skipped, so a partially transferred tree never loses data.
fn finish_move(job: &dyn JobBridge, root: &PlanRoot, items: &[TransferItem], failures: usize) {
    if failures > 0 {
        job.root_retained(
            root,
            "the source was kept because part of this folder did not transfer",
        );
        return;
    }

    let unsupported = items
        .iter()
        .filter(|item| item.skip_reason == Some(TransferIssueReason::Unsupported))
        .count();
    if unsupported > 0 {
        job.root_retained(
            root,
            "the source was kept because it holds entries CrossPort does not copy",
        );
        return;
    }

    let skipped = items
        .iter()
        .filter(|item| item.skip_reason == Some(TransferIssueReason::Skipped))
        .count();
    if skipped > 0 {
        job.root_retained(
            root,
            "the source was kept because existing destinations were skipped",
        );
        return;
    }

    match delete_source_root(root) {
        Ok(()) => {}
        Err(Abort::Cancelled) => {}
        Err(Abort::Failed(error)) => {
            // The destination is complete; its source could not be removed.
            job.item_failed(&root.source, error);
        }
    }
}

fn rename_root(root: &PlanRoot) -> Result<(), Abort> {
    std::fs::rename(&root.source, &root.destination)
        .map_err(|error| Abort::Failed(safety::write_error(error, &root.destination)))
}

fn delete_source_root(root: &PlanRoot) -> Result<(), Abort> {
    let result = match root.kind {
        ItemKind::File => std::fs::remove_file(&root.source),
        ItemKind::Directory => std::fs::remove_dir_all(&root.source),
    };
    result.map_err(|error| Abort::Failed(safety::write_error(error, &root.source)))
}

/// Creates one planned directory, clearing a blocking entry first when the plan
/// says the destination is being replaced by something of another kind.
fn transfer_directory(item: &TransferItem, created: &mut Vec<PathBuf>) -> Result<(), Abort> {
    if let Some(kind) = item.clear_first {
        safety::remove_entry(&item.destination, kind).map_err(Abort::Failed)?;
    }

    if !item.destination.exists() {
        created.extend(safety::create_chain(&item.destination).map_err(Abort::Failed)?);
    }

    Ok(())
}

/// Streams one planned file into place.
fn transfer_file(
    plan: &TransferPlan,
    job: &dyn JobBridge,
    item: &TransferItem,
    item_index: usize,
    buffer: &mut [u8],
    created: &mut Vec<PathBuf>,
) -> Result<(), Abort> {
    job.item_started(item);

    if let Some(kind) = item.clear_first {
        safety::remove_entry(&item.destination, kind).map_err(Abort::Failed)?;
    }

    // The parent exists for any planned item, but a destination that vanished
    // mid-transfer is recreated rather than failing the item outright.
    let parent = item.destination.parent().unwrap_or_else(|| Path::new(""));
    if !parent.exists() {
        created.extend(safety::create_chain(parent).map_err(Abort::Failed)?);
    }

    let mut reader = File::open(&item.source)
        .map_err(|error| Abort::Failed(safety::read_error(error, &item.source)))?;

    let temp_path = partial_path(job.job_id(), item_index, &item.destination);
    let mut partial = PartialFile::create(&temp_path)?;

    // Only a checksum policy pays for a source digest, and it is computed from
    // the bytes that are already flowing past rather than by reading the source
    // again.
    let mut hasher = plan
        .verification
        .algorithm()
        .map(|_algorithm| StreamHasher::new());

    let copied = {
        let writer = partial.writer()?;
        let mut observer = JobObserver { job };
        match stream_copy(&mut reader, writer, buffer, &mut observer, hasher.as_mut()) {
            Ok(outcome) if !outcome.aborted => outcome,
            // The observer stopped the copy: that is a cancellation, and the
            // temporary file is removed when `partial` goes out of scope.
            Ok(_) => return Err(Abort::Cancelled),
            Err(StreamError::Read(error)) => {
                return Err(Abort::Failed(safety::read_error(error, &item.source)))
            }
            Err(StreamError::Write(error)) => {
                return Err(Abort::Failed(safety::write_error(error, &item.destination)))
            }
        }
    };

    drop(reader);
    partial.finalize()?;

    // A move only deletes its source after proving the destination holds all
    // of the bytes the plan measured. A file that grew or shrank while being
    // copied keeps its source: a partial move is worse than no move.
    if plan.operation == TransferOperation::Move && copied.bytes != item.size_bytes {
        return Err(Abort::Failed(AppError::TransferFailed(format!(
            "'{}' changed while it was being copied ({} bytes expected, {} bytes read); the copy was kept and the source left in place",
            item.source.display(),
            item.size_bytes,
            copied.bytes
        ))));
    }

    let destination = commit_destination(plan, item)?;
    std::fs::rename(partial.path(), &destination)
        .map_err(|error| Abort::Failed(safety::write_error(error, &destination)))?;
    partial.disarm();

    // Verification happens on the committed file, so what is checked is what a
    // user would open. The expected size is the number of bytes that actually
    // came out of the source, which is the only figure this transfer can prove.
    verify_committed(plan, job, item, &destination, copied.bytes, hasher, partial)?;

    Ok(())
}

/// Verifies one committed file and reports the result to the job.
///
/// A result that means the data did not arrive is turned into an item failure
/// here. The file is left in place: removing it would destroy something the
/// user can inspect, and the job's failure already says the copy is not
/// trustworthy.
fn verify_committed(
    plan: &TransferPlan,
    job: &dyn JobBridge,
    item: &TransferItem,
    destination: &Path,
    written_bytes: u64,
    hasher: Option<StreamHasher>,
    partial: PartialFile,
) -> Result<(), Abort> {
    // Nothing to check when the policy is off, but the fact that nothing was
    // checked is recorded rather than left implicit.
    if plan.verification == VerificationPolicy::None {
        job.item_verified(&verification::FileVerification::skipped(destination));
        drop(partial);
        return Ok(());
    }

    job.item_verifying(item);

    let streamed_checksum = hasher.map(StreamHasher::finish);
    // The temporary file is gone by now (it became the destination), so
    // dropping it here cannot remove the committed file.
    drop(partial);

    let mut checkpoint = || job.checkpoint().is_ok();
    let result = verification::verify_item(
        verification::ItemVerification {
            source: &item.source,
            destination,
            expected_bytes: written_bytes,
            streamed_checksum,
        },
        plan.verification,
        &mut checkpoint,
    );

    match result {
        Ok(result) => {
            job.item_verified(&result);
            if result.status.is_failure() {
                return Err(Abort::Failed(result.failure_error()));
            }
            Ok(())
        }
        // The checkpoint stopped the verification: that is a cancellation, and
        // the item must not be reported as completed.
        Err(VerificationAborted) => Err(Abort::Cancelled),
    }
}

/// The path the completed file is renamed onto.
///
/// With the rename strategy the plan already picked a free name, but something
/// can appear at that name between planning and committing. Re-checking here
/// means the engine never overwrites an entry it did not plan to overwrite.
fn commit_destination(plan: &TransferPlan, item: &TransferItem) -> Result<PathBuf, Abort> {
    if plan.conflict != ConflictStrategy::Rename || !conflict::occupied(&item.destination) {
        return Ok(item.destination.clone());
    }

    conflict::unique_destination(&item.destination, &HashSet::new()).map_err(Abort::Failed)
}

/// Temporary file beside the destination it will become.
///
/// The job identifier keeps two jobs from sharing a name, and the item index
/// keeps two files of the same job apart. The name is never derived from the
/// user's file names, so a transfer cannot collide with real data.
fn partial_path(job_id: &str, item_index: usize, destination: &Path) -> PathBuf {
    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!(".crossport-{job_id}-{item_index}.partial"))
}

struct JobObserver<'a> {
    job: &'a dyn JobBridge,
}

impl CopyObserver for JobObserver<'_> {
    fn before_chunk(&mut self) -> bool {
        self.job.checkpoint().is_ok()
    }

    fn after_chunk(&mut self, bytes: u64) {
        self.job.record_bytes(bytes);
    }
}

/// A file being written at a temporary path.
///
/// Dropping it without committing removes the temporary file, so a cancelled,
/// failed, or panicking transfer cannot leave a partial file behind, and an
/// existing destination file is never touched before the new data is complete.
struct PartialFile {
    path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl PartialFile {
    fn create(path: &Path) -> Result<Self, Abort> {
        let file =
            File::create(path).map_err(|error| Abort::Failed(safety::write_error(error, path)))?;
        Ok(Self {
            path: path.to_path_buf(),
            file: Some(file),
            committed: false,
        })
    }

    fn writer(&mut self) -> Result<&mut File, Abort> {
        self.file.as_mut().ok_or_else(|| {
            Abort::Failed(AppError::Internal(
                "the temporary file was already closed".to_string(),
            ))
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Flushes and closes the temporary file. `File` has no user-space buffer,
    /// so this is the last chance to hear about a deferred write failure before
    /// the rename makes the file look complete.
    fn finalize(&mut self) -> Result<(), Abort> {
        if let Some(mut file) = self.file.take() {
            file.flush()
                .map_err(|error| Abort::Failed(safety::write_error(error, &self.path)))?;
        }
        Ok(())
    }

    /// Marks the temporary path as successfully renamed away.
    fn disarm(&mut self) {
        self.committed = true;
    }
}

impl Drop for PartialFile {
    fn drop(&mut self) {
        // Close the handle before removing: Windows cannot delete an open file.
        let _ = self.file.take();
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;
    use crate::transfer::model::{ConflictStrategy, TransferRequest};
    use crate::transfer::plan;
    use std::sync::Mutex;

    fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// A bridge that records everything the engine reports, so tests assert on
    /// the same facts the UI receives.
    #[derive(Default)]
    struct TestJob {
        id: String,
        state: Mutex<TestJobState>,
    }

    #[derive(Default)]
    struct TestJobState {
        /// Cancel once this many bytes have been recorded.
        cancel_after_bytes: Option<u64>,
        bytes: u64,
        started: Vec<String>,
        completed: Vec<String>,
        skipped: Vec<String>,
        failed: Vec<(String, AppError)>,
        moved: Vec<String>,
        retained: Vec<String>,
        /// How many times verification was announced.
        verifying: usize,
        /// The status of every verification result reported, in order.
        verifications: Vec<String>,
    }

    impl TestJob {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                state: Mutex::new(TestJobState::default()),
            }
        }

        fn cancel_after(bytes: u64) -> Self {
            let job = Self::new("test");
            lock(&job.state).cancel_after_bytes = Some(bytes);
            job
        }

        fn snapshot(&self) -> TestJobState {
            let state = lock(&self.state);
            TestJobState {
                cancel_after_bytes: state.cancel_after_bytes,
                bytes: state.bytes,
                started: state.started.clone(),
                completed: state.completed.clone(),
                skipped: state.skipped.clone(),
                failed: state.failed.clone(),
                moved: state.moved.clone(),
                verifying: state.verifying,
                verifications: state.verifications.clone(),
                retained: state.retained.clone(),
            }
        }
    }

    impl JobBridge for TestJob {
        fn job_id(&self) -> &str {
            &self.id
        }

        fn checkpoint(&self) -> Result<(), Abort> {
            let state = lock(&self.state);
            if let Some(limit) = state.cancel_after_bytes {
                if state.bytes >= limit {
                    return Err(Abort::Cancelled);
                }
            }
            Ok(())
        }

        fn item_started(&self, item: &TransferItem) {
            lock(&self.state)
                .started
                .push(item.source.display().to_string());
        }

        fn record_bytes(&self, bytes: u64) {
            let mut state = lock(&self.state);
            state.bytes = state.bytes.saturating_add(bytes);
        }

        fn item_verifying(&self, _item: &TransferItem) {
            let mut state = lock(&self.state);
            state.verifying = state.verifying.saturating_add(1);
        }

        fn item_verified(&self, verification: &FileVerification) {
            lock(&self.state)
                .verifications
                .push(verification.status.as_str().to_string());
        }

        fn item_completed(&self, item: &TransferItem) {
            lock(&self.state)
                .completed
                .push(item.destination.display().to_string());
        }

        fn item_skipped(&self, item: &TransferItem) {
            lock(&self.state)
                .skipped
                .push(item.destination.display().to_string());
        }

        fn item_failed(&self, path: &Path, error: AppError) {
            lock(&self.state)
                .failed
                .push((path.display().to_string(), error));
        }

        fn root_skipped(&self, root: &PlanRoot) {
            lock(&self.state)
                .skipped
                .push(root.source.display().to_string());
        }

        fn root_moved(&self, root: &PlanRoot) {
            lock(&self.state)
                .moved
                .push(root.source.display().to_string());
        }

        fn root_retained(&self, root: &PlanRoot, detail: &str) {
            lock(&self.state)
                .retained
                .push(format!("{}: {detail}", root.source.display()));
        }
    }

    /// A reader that produces `total` zero bytes without ever allocating them,
    /// so a test can stream far more data than the machine should hold.
    struct ZeroSource {
        remaining: u64,
        largest_request: usize,
    }

    impl Read for ZeroSource {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.largest_request = self.largest_request.max(buffer.len());
            if self.remaining == 0 {
                return Ok(0);
            }
            let take = self.remaining.min(buffer.len() as u64) as usize;
            buffer[..take].fill(0);
            self.remaining -= take as u64;
            Ok(take)
        }
    }

    /// A writer that counts bytes and can fail at a given point.
    struct CountingWriter {
        written: u64,
        fail_after: Option<u64>,
    }

    impl Write for CountingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if let Some(limit) = self.fail_after {
                if self.written >= limit {
                    return Err(io::Error::other("destination went away"));
                }
            }
            self.written += buffer.len() as u64;
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Recorder {
        chunks: Vec<u64>,
        stops_after: Option<usize>,
    }

    impl CopyObserver for Recorder {
        fn before_chunk(&mut self) -> bool {
            match self.stops_after {
                Some(limit) => self.chunks.len() < limit,
                None => true,
            }
        }

        fn after_chunk(&mut self, bytes: u64) {
            self.chunks.push(bytes);
        }
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_file(path: &Path, bytes: usize) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent is creatable");
        }
        std::fs::write(path, vec![42u8; bytes]).expect("file is writable");
    }

    fn request(
        sources: &[&Path],
        destination: &Path,
        operation: TransferOperation,
        conflict: ConflictStrategy,
    ) -> TransferRequest {
        TransferRequest {
            sources: sources
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            destination: destination.display().to_string(),
            operation,
            conflict,
            verification: None,
        }
    }

    fn plan_for(
        sources: &[&Path],
        destination: &Path,
        operation: TransferOperation,
        conflict: ConflictStrategy,
    ) -> TransferPlan {
        let request = request(sources, destination, operation, conflict);
        if operation == TransferOperation::Copy {
            plan::plan_request(&request).expect("plan")
        } else {
            plan::plan_for_start(&request).expect("plan")
        }
    }

    #[test]
    fn streaming_never_holds_more_than_one_buffer_of_data() {
        let mut source = ZeroSource {
            remaining: 64 * 1024 * 1024,
            largest_request: 0,
        };
        let mut writer = CountingWriter {
            written: 0,
            fail_after: None,
        };
        let mut buffer = vec![0u8; 64 * 1024];
        let mut recorder = Recorder::default();

        let outcome = stream_copy(&mut source, &mut writer, &mut buffer, &mut recorder, None)
            .expect("the copy succeeds");

        assert_eq!(outcome.bytes, 64 * 1024 * 1024);
        assert_eq!(writer.written, 64 * 1024 * 1024);
        assert_eq!(
            source.largest_request,
            64 * 1024,
            "the reader is never asked for more than the buffer"
        );
        assert!(
            recorder.chunks.iter().all(|chunk| *chunk <= 64 * 1024),
            "no chunk is ever larger than the buffer"
        );
        assert!(
            recorder.chunks.len() >= 1024,
            "64 MiB through a 64 KiB buffer means a thousand bounded reads"
        );
    }

    #[test]
    fn streaming_stops_before_writing_when_the_observer_says_stop() {
        let mut source = ZeroSource {
            remaining: 8 * 1024 * 1024,
            largest_request: 0,
        };
        let mut writer = CountingWriter {
            written: 0,
            fail_after: None,
        };
        let mut buffer = vec![0u8; 16 * 1024];
        let mut recorder = Recorder {
            chunks: Vec::new(),
            stops_after: Some(2),
        };

        let outcome = stream_copy(&mut source, &mut writer, &mut buffer, &mut recorder, None)
            .expect("the copy stops cleanly");

        assert!(outcome.aborted, "the caller is told nothing was committed");
        assert_eq!(outcome.bytes, 32 * 1024);
        assert_eq!(writer.written, 32 * 1024);
    }

    #[test]
    fn streaming_reports_which_side_failed() {
        let mut source = ZeroSource {
            remaining: 1024 * 1024,
            largest_request: 0,
        };
        let mut writer = CountingWriter {
            written: 0,
            fail_after: Some(4096),
        };
        let mut buffer = vec![0u8; 1024];
        let mut recorder = Recorder::default();

        let error = stream_copy(&mut source, &mut writer, &mut buffer, &mut recorder, None)
            .expect_err("the write failure is reported");

        assert!(
            matches!(error, StreamError::Write(_)),
            "a destination failure keeps its own side of the copy"
        );
    }

    #[test]
    fn copies_a_single_file_and_reports_real_bytes() {
        let workspace = unique_temp_dir("copy-single");
        let source = workspace.join("payload.bin");
        write_file(&source, 3 * 1024 * 1024 + 7);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("copy-single");

        execute(&plan, &job).expect("the transfer completes");

        let state = job.snapshot();
        assert_eq!(state.completed.len(), 1);
        assert_eq!(state.bytes, 3 * 1024 * 1024 + 7);
        assert!(state.failed.is_empty());
        let copied =
            std::fs::read(destination.join("payload.bin")).expect("destination is readable");
        assert_eq!(copied.len(), 3 * 1024 * 1024 + 7);
        assert_eq!(copied, std::fs::read(&source).expect("source is readable"));

        clean_up(&workspace);
    }

    #[test]
    fn copies_nested_directories_and_empty_directories() {
        let workspace = unique_temp_dir("copy-tree");
        let source = workspace.join("Data");
        write_file(&source.join("root.txt"), 16);
        write_file(&source.join("sub").join("inner.txt"), 32);
        std::fs::create_dir_all(source.join("empty").join("deeper")).expect("subdir");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("copy-tree");

        execute(&plan, &job).expect("the transfer completes");

        assert!(destination.join("Data").join("root.txt").is_file());
        assert!(destination
            .join("Data")
            .join("sub")
            .join("inner.txt")
            .is_file());
        assert!(
            destination
                .join("Data")
                .join("empty")
                .join("deeper")
                .is_dir(),
            "empty folders are created"
        );
        assert_eq!(job.snapshot().bytes, 48);
        assert_eq!(
            std::fs::read(source.join("sub").join("inner.txt")).expect("readable"),
            std::fs::read(destination.join("Data").join("sub").join("inner.txt"))
                .expect("readable")
        );

        clean_up(&workspace);
    }

    #[test]
    fn skipping_leaves_the_existing_file_exactly_as_it_was() {
        let workspace = unique_temp_dir("copy-skip");
        let source = workspace.join("notes.txt");
        write_file(&source, 10);
        let destination = workspace.join("out");
        write_file(&destination.join("notes.txt"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("copy-skip");

        execute(&plan, &job).expect("the transfer completes");

        let state = job.snapshot();
        assert_eq!(state.skipped.len(), 1);
        assert_eq!(state.bytes, 0, "a skipped item transfers nothing");
        assert_eq!(
            std::fs::read(destination.join("notes.txt"))
                .expect("readable")
                .len(),
            4,
            "the existing file still holds its own bytes"
        );

        clean_up(&workspace);
    }

    #[test]
    fn replace_overwrites_an_existing_file_atomically() {
        let workspace = unique_temp_dir("copy-replace");
        let source = workspace.join("notes.txt");
        write_file(&source, 64);
        let destination = workspace.join("out");
        write_file(&destination.join("notes.txt"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        );
        let job = TestJob::new("copy-replace");

        execute(&plan, &job).expect("the transfer completes");

        assert_eq!(
            std::fs::read(destination.join("notes.txt"))
                .expect("readable")
                .len(),
            64
        );
        assert_eq!(job.snapshot().bytes, 64);

        clean_up(&workspace);
    }

    #[test]
    fn replace_clears_a_blocking_directory_so_a_file_can_be_written() {
        let workspace = unique_temp_dir("copy-replace-dir");
        let source = workspace.join("payload.bin");
        write_file(&source, 8);
        let destination = workspace.join("out");
        write_file(&destination.join("payload.bin").join("inside.txt"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        );
        let job = TestJob::new("copy-replace-dir");

        execute(&plan, &job).expect("the transfer completes");

        assert!(destination.join("payload.bin").is_file());
        assert_eq!(job.snapshot().bytes, 8);

        clean_up(&workspace);
    }

    #[test]
    fn replace_clears_a_blocking_file_so_a_directory_can_be_created() {
        let workspace = unique_temp_dir("copy-replace-file");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 8);
        let destination = workspace.join("out");
        write_file(&destination.join("Data"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        );
        let job = TestJob::new("copy-replace-file");

        execute(&plan, &job).expect("the transfer completes");

        assert!(destination.join("Data").join("inner.txt").is_file());

        clean_up(&workspace);
    }

    #[test]
    fn rename_never_overwrites_a_file_that_appeared_after_planning() {
        let workspace = unique_temp_dir("copy-rename-race");
        let source = workspace.join("report.txt");
        write_file(&source, 16);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let request = request(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Rename,
        );
        // The name `report.txt` is free when the plan is made...
        let plan = plan::plan_request(&request).expect("plan");
        assert_eq!(plan.items[0].destination, destination.join("report.txt"));

        // ...and taken by the time the file is committed.
        write_file(&destination.join("report.txt"), 3);
        let job = TestJob::new("copy-rename-race");

        execute(&plan, &job).expect("the transfer completes");

        assert_eq!(
            std::fs::read(destination.join("report.txt"))
                .expect("readable")
                .len(),
            3,
            "the file that appeared first is untouched"
        );
        assert_eq!(
            std::fs::read(destination.join("report (2).txt"))
                .expect("readable")
                .len(),
            16,
            "the transfer writes beside it instead"
        );

        clean_up(&workspace);
    }

    #[test]
    fn cancelling_leaves_no_partial_file_and_keeps_the_old_one() {
        let workspace = unique_temp_dir("copy-cancel");
        let source = workspace.join("big.bin");
        write_file(&source, 4 * 1024 * 1024);
        let destination = workspace.join("out");
        write_file(&destination.join("big.bin"), 8);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        );
        let job = TestJob::cancel_after(512 * 1024);

        let outcome = execute(&plan, &job);

        assert_eq!(outcome, Err(Abort::Cancelled));
        assert_eq!(
            std::fs::read(destination.join("big.bin"))
                .expect("readable")
                .len(),
            8,
            "the file being replaced still holds its original bytes"
        );
        let leftovers: Vec<String> = std::fs::read_dir(&destination)
            .expect("readable")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            leftovers,
            vec!["big.bin".to_string()],
            "no partial file is left"
        );

        clean_up(&workspace);
    }

    #[test]
    fn cancelling_removes_the_directories_it_created() {
        let workspace = unique_temp_dir("copy-cancel-dirs");
        let source = workspace.join("Data");
        write_file(&source.join("sub").join("inner.bin"), 4 * 1024 * 1024);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        let job = TestJob::cancel_after(64 * 1024);

        assert_eq!(execute(&plan, &job), Err(Abort::Cancelled));
        assert!(
            !destination.join("Data").exists(),
            "the folders this job created are cleaned up when the job is cancelled"
        );
        assert!(destination.is_dir(), "the destination the user chose stays");

        clean_up(&workspace);
    }

    #[test]
    fn one_failed_item_does_not_stop_the_others() {
        let workspace = unique_temp_dir("copy-isolation");
        let good = workspace.join("good.bin");
        write_file(&good, 2048);
        let bad = workspace.join("bad.bin");
        write_file(&bad, 2048);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&good, &bad],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        // The source disappears between planning and execution.
        std::fs::remove_file(&bad).expect("the test owns the file");
        let job = TestJob::new("copy-isolation");

        execute(&plan, &job).expect("the job still runs to the end");

        let state = job.snapshot();
        assert_eq!(state.failed.len(), 1);
        assert_eq!(state.failed[0].1.code(), "path_not_found");
        assert!(destination.join("good.bin").is_file());
        assert!(!destination.join("bad.bin").exists());

        clean_up(&workspace);
    }

    #[test]
    fn a_failed_item_leaves_no_partial_file_behind() {
        let workspace = unique_temp_dir("copy-failed-partial");
        let source = workspace.join("payload.bin");
        write_file(&source, 4096);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        std::fs::remove_file(&source).expect("the test owns the file");
        let job = TestJob::new("copy-failed-partial");

        execute(&plan, &job).expect("the job runs to the end");

        assert_eq!(job.snapshot().failed.len(), 1);
        assert_eq!(
            std::fs::read_dir(&destination)
                .expect("readable")
                .flatten()
                .count(),
            0,
            "nothing is left at the destination"
        );

        clean_up(&workspace);
    }

    #[test]
    fn a_same_volume_move_renames_in_one_step() {
        let workspace = unique_temp_dir("move-rename");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 64);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        );
        assert!(plan.roots[0].clean);
        let job = TestJob::new("move-rename");

        execute(&plan, &job).expect("the move completes");

        let state = job.snapshot();
        assert_eq!(state.moved.len(), 1, "the root is renamed, not copied");
        assert_eq!(state.bytes, 0, "a rename writes no bytes");
        assert!(!source.exists(), "the source is gone");
        assert!(destination.join("Data").join("inner.txt").is_file());

        clean_up(&workspace);
    }

    #[test]
    fn a_move_with_a_conflict_copies_and_then_removes_the_source() {
        let workspace = unique_temp_dir("move-replace");
        let source = workspace.join("notes.txt");
        write_file(&source, 128);
        let destination = workspace.join("out");
        write_file(&destination.join("notes.txt"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Replace,
        );
        assert!(!plan.roots[0].clean, "a collision disables the rename path");
        let job = TestJob::new("move-replace");

        execute(&plan, &job).expect("the move completes");

        assert_eq!(
            std::fs::read(destination.join("notes.txt"))
                .expect("readable")
                .len(),
            128
        );
        assert!(!source.exists(), "the source is removed after the copy");
        assert_eq!(job.snapshot().bytes, 128);

        clean_up(&workspace);
    }

    #[test]
    fn a_move_keeps_the_source_when_an_item_was_skipped() {
        let workspace = unique_temp_dir("move-skipped");
        let source = workspace.join("notes.txt");
        write_file(&source, 32);
        let destination = workspace.join("out");
        write_file(&destination.join("notes.txt"), 4);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("move-skipped");

        execute(&plan, &job).expect("the job completes");

        assert!(source.exists(), "a skipped item keeps its source");
        assert_eq!(
            std::fs::read(destination.join("notes.txt"))
                .expect("readable")
                .len(),
            4
        );

        clean_up(&workspace);
    }

    #[test]
    fn a_move_keeps_the_source_when_the_file_changed_while_copying() {
        let workspace = unique_temp_dir("move-changed");
        let source = workspace.join("payload.bin");
        write_file(&source, 4096);
        let destination = workspace.join("out");
        // An existing destination entry forces the copy-then-remove path, which
        // is the only path where a size check can protect the source.
        write_file(&destination.join("payload.bin"), 4);

        let mut plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Replace,
        );
        // Pretend the plan measured more bytes than the file now holds, which
        // is exactly what a source that shrank mid-copy looks like.
        plan.items[0].size_bytes = 8192;
        let job = TestJob::new("move-changed");

        execute(&plan, &job).expect("the job completes");

        let state = job.snapshot();
        assert_eq!(state.failed.len(), 1);
        assert_eq!(state.failed[0].1.code(), "transfer_failed");
        assert!(
            source.exists(),
            "a source that changed is never deleted, even though the copy exists"
        );
        assert!(
            destination.join("payload.bin").is_file(),
            "the bytes that were read are still there"
        );

        clean_up(&workspace);
    }

    #[test]
    fn a_move_keeps_the_source_when_a_folder_holds_entries_it_did_not_copy() {
        let workspace = unique_temp_dir("move-links");
        let source = workspace.join("Data");
        write_file(&source.join("real.txt"), 16);
        let target = workspace.join("outside.txt");
        write_file(&target, 16);
        let link = source.join("link.txt");

        if !try_symlink_file(&target, &link) {
            clean_up(&workspace);
            return;
        }

        let destination = workspace.join("out");
        // An existing entry inside the destination root means this move runs
        // item by item instead of taking the whole-tree rename path, so the
        // engine has to decide what deleting the source would cost.
        write_file(&destination.join("Data").join("real.txt"), 4);
        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Replace,
        );
        let job = TestJob::new("move-links");

        execute(&plan, &job).expect("the job completes");

        let state = job.snapshot();
        assert_eq!(
            state.retained.len(),
            1,
            "the source is kept and the reason is reported: {:?}",
            state.retained
        );
        assert!(
            source.join("link.txt").exists(),
            "the entry that was not copied is still where it was"
        );
        assert_eq!(
            std::fs::read(destination.join("Data").join("real.txt"))
                .expect("readable")
                .len(),
            16,
            "the copied file did land in the destination"
        );

        clean_up(&workspace);
    }

    #[test]
    fn a_same_volume_move_of_a_folder_holding_links_moves_it_whole() {
        let workspace = unique_temp_dir("move-links-rename");
        let source = workspace.join("Data");
        write_file(&source.join("real.txt"), 16);
        let target = workspace.join("outside.txt");
        write_file(&target, 16);
        let link = source.join("link.txt");

        if !try_symlink_file(&target, &link) {
            clean_up(&workspace);
            return;
        }

        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");
        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("move-links-rename");

        execute(&plan, &job).expect("the move completes");

        let state = job.snapshot();
        assert_eq!(state.moved.len(), 1, "the tree is renamed in one step");
        assert!(
            state.failed.is_empty(),
            "a rename reports no failures: {:?}",
            state.failed
        );
        assert!(!source.exists(), "the source moved as a whole");
        assert!(
            destination.join("Data").join("link.txt").exists(),
            "a rename carries every entry with it, including links"
        );

        clean_up(&workspace);
    }

    #[test]
    fn a_move_of_a_folder_with_a_nested_collision_still_removes_the_source() {
        let workspace = unique_temp_dir("move-merged");
        let source = workspace.join("Data");
        write_file(&source.join("keep.txt"), 10);
        write_file(&source.join("replace.txt"), 20);
        let destination = workspace.join("out");
        write_file(&destination.join("Data").join("replace.txt"), 2);

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Replace,
        );
        let job = TestJob::new("move-merged");

        execute(&plan, &job).expect("the move completes");

        assert!(
            !source.exists(),
            "every item transferred, so the source goes"
        );
        assert_eq!(
            std::fs::read(destination.join("Data").join("keep.txt"))
                .expect("readable")
                .len(),
            10
        );
        assert_eq!(
            std::fs::read(destination.join("Data").join("replace.txt"))
                .expect("readable")
                .len(),
            20
        );

        clean_up(&workspace);
    }

    #[test]
    fn empty_directory_plans_create_every_folder() {
        let workspace = unique_temp_dir("copy-only-directories");
        let source = workspace.join("Tree");
        std::fs::create_dir_all(source.join("a").join("b")).expect("subdir");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        );
        let job = TestJob::new("copy-only-directories");

        execute(&plan, &job).expect("the transfer completes");

        assert_eq!(job.snapshot().bytes, 0);
        assert!(destination.join("Tree").join("a").join("b").is_dir());

        clean_up(&workspace);
    }

    /// Creates a file symlink, returning `false` when the platform refuses.
    fn try_symlink_file(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, link);
            false
        }
    }
}
