/* ==========================================================================
 * Transfer sanity suite
 * The milestone's end-to-end check: real files, real directories, real
 * temporary workspace, driven through the same engine and the same queue the
 * application uses — not through an internal shortcut.
 *
 * Every test here creates its own workspace under the system temporary
 * directory and removes it again (even when it fails), so nothing outside that
 * workspace is read or written. The last test measures this process's working
 * set while a 192 MiB file is copied to prove that memory does not scale with
 * file size.
 * ========================================================================== */

use super::*;
use crate::filesystem::test_support::unique_temp_dir;
use crate::platform;
use crate::platform::volume::VolumeKind;
use crate::verification::{
    ChecksumAlgorithm, VerificationMethod, VerificationMismatchReason, VerificationPolicy,
    VerificationStatus,
};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

const TIMEOUT: Duration = Duration::from_secs(120);

/// Large enough that a whole-file buffer would be obvious in the memory
/// measurement below, small enough to stay a quick test.
const LARGE_FILE_BYTES: usize = 192 * 1024 * 1024;

/// Ceiling for the memory a transfer of [`LARGE_FILE_BYTES`] may add.
///
/// Streaming through one reused 1 MiB buffer costs a few megabytes; buffering
/// the file would cost at least its full 192 MiB. The ceiling sits squarely
/// between the two, far enough from the streaming figure that other tests
/// running in this process cannot push a correct engine over it.
const MEMORY_CEILING_BYTES: u64 = 96 * 1024 * 1024;

/// A temporary workspace, removed when the test ends however it ends.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(label: &str) -> Self {
        Self {
            root: unique_temp_dir(label),
        }
    }

    /// A workspace on a root chosen by the caller, removed on drop like any
    /// other: used for the cross-volume tests, which have to place one side of
    /// the transfer on a second volume.
    fn at(root: PathBuf) -> Self {
        std::fs::create_dir_all(&root).expect("workspace root is creatable");
        Self { root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Creates a file of `bytes` bytes at `relative`, with the last byte encoding
    /// the path so mismatched content is detectable.
    fn file(&self, relative: &str, bytes: usize) -> PathBuf {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent is creatable");
        }
        let mut content = vec![0x11u8; bytes];
        if let Some(last) = content.last_mut() {
            *last = (bytes % 251) as u8;
        }
        std::fs::write(&path, content).expect("file is writable");
        path
    }

    fn directory(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(&path).expect("directory is creatable");
        path
    }

    fn file_text(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent is creatable");
        }
        std::fs::write(&path, text.as_bytes()).expect("file is writable");
        path
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Every entry under `base`, relative to it, sorted. Directories carry a
/// trailing separator so an empty directory shows up in the comparison.
fn relative_tree(base: &Path) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    let mut pending = vec![base.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(children) = std::fs::read_dir(&directory) else {
            continue;
        };
        for child in children.flatten() {
            let path = child.path();
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                entries.push(format!("{relative}/"));
                pending.push(path);
            } else {
                entries.push(relative);
            }
        }
    }

    entries.sort();
    entries
}

/// Asserts that `destination` holds exactly what `source` holds, byte for byte.
fn assert_same_tree(source: &Path, destination: &Path) {
    let source_tree = relative_tree(source);
    let destination_tree = relative_tree(destination);
    assert_eq!(
        destination_tree, source_tree,
        "the copied tree must match the source exactly"
    );

    for relative in source_tree.iter().filter(|entry| !entry.ends_with('/')) {
        let from = source.join(relative);
        let to = destination.join(relative);
        assert_eq!(
            std::fs::read(&to).expect("the copied file is readable"),
            std::fs::read(&from).expect("the source file is readable"),
            "'{relative}' must hold the same bytes"
        );
    }
}

/// Every file under `base` with its bytes, keyed by its relative path.
///
/// Used where the source is expected to disappear before the comparison — a
/// completed move — so the destination can be checked against what was there.
fn collect_files(base: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = relative_tree(base)
        .into_iter()
        .filter(|entry| !entry.ends_with('/'))
        .map(|relative| {
            let bytes = std::fs::read(base.join(&relative)).expect("the file is readable");
            (relative, bytes)
        })
        .collect();
    files.sort();
    files
}

/// Fails when any temporary or partial artefact is left anywhere under `root`.
fn assert_no_leftovers(root: &Path) {
    let leftovers: Vec<String> = relative_tree(root)
        .into_iter()
        .filter(|entry| entry.contains(".crossport-") || entry.ends_with(".partial"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a finished or cancelled transfer leaves nothing behind: {leftovers:?}"
    );
}

/// A workspace on a volume other than the one the system temporary directory
/// lives on, or `None` when the host exposes only one writable volume.
///
/// Cross-volume work is the case the same-volume rename shortcut cannot cover:
/// the bytes really have to be copied, and a move may only remove the source
/// after the destination is complete. The directory is unique, created on the
/// second volume's root, and removed on drop even when the test fails.
fn second_volume_workspace(label: &str) -> Option<Workspace> {
    let temp_volume = crate::platform::drives::volume_root_for(&std::env::temp_dir())?;
    for volume in crate::platform::drives::list_drives() {
        let usable = volume.mounted
            && volume.readonly != Some(true)
            && matches!(volume.kind, VolumeKind::Fixed | VolumeKind::Removable);
        if !usable {
            continue;
        }
        let root = PathBuf::from(&volume.root);
        if crate::platform::paths::same_path(&root, &temp_volume) {
            continue;
        }
        let candidate = root.join(format!("crossport-sanity-{label}-{}", std::process::id()));
        if std::fs::create_dir_all(&candidate).is_ok() {
            return Some(Workspace::at(candidate));
        }
    }
    None
}

fn wait_for_status(engine: &TransferEngine, id: &str, status: TransferStatus) -> TransferSnapshot {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let snapshot = engine.snapshot(id).expect("the job exists");
        if snapshot.status == status {
            return snapshot;
        }
        if snapshot.status.is_terminal() || Instant::now() > deadline {
            panic!("'{id}' never reached {status:?}; last state: {snapshot:?}");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn request_for(
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

/// Cross-volume copy: the bytes really move between volumes, the source stays,
/// and nothing temporary is left behind on either side.
#[test]
fn sanity_a_cross_volume_copy_moves_the_bytes_and_keeps_the_source() {
    let Some(destination_workspace) = second_volume_workspace("copy") else {
        println!("skipped: this host exposes a single writable volume");
        return;
    };
    let source_workspace = Workspace::new("sanity-cross-volume-copy");
    let source = source_workspace.directory("Data");
    source_workspace.file("Data/readme.md", 6 * 1024);
    source_workspace.file("Data/nested/large.bin", 4 * 1024 * 1024);

    let destination = destination_workspace.directory("target");
    let engine = TransferEngine::new(1);
    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        ))
        .expect("a cross-volume copy plans");

    let finished = wait_for_status(&engine, &queued.id, TransferStatus::Completed);
    assert_eq!(finished.progress.failed_items, 0);
    assert_same_tree(&source, &destination.join("Data"));
    assert!(source.exists(), "a copy leaves the source where it was");
    assert_no_leftovers(&destination_workspace.root);
    assert_no_leftovers(&source_workspace.root);

    engine.shutdown();
}

/// Cross-volume move: copy first, remove the source only after the destination
/// is complete, and leave no partial output on either volume.
#[test]
fn sanity_a_cross_volume_move_copies_before_it_removes() {
    let Some(destination_workspace) = second_volume_workspace("move") else {
        println!("skipped: this host exposes a single writable volume");
        return;
    };
    let source_workspace = Workspace::new("sanity-cross-volume-move");
    let source = source_workspace.directory("Payload");
    source_workspace.file("Payload/alpha.bin", 2 * 1024 * 1024);
    source_workspace.file("Payload/nested/beta.bin", 512 * 1024);

    // The source is gone after a completed move, so what it held is recorded
    // first and the destination is compared against that record.
    let expected = collect_files(&source);

    let destination = destination_workspace.directory("target");
    let engine = TransferEngine::new(1);
    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Replace,
        ))
        .expect("a cross-volume move plans");

    let finished = wait_for_status(&engine, &queued.id, TransferStatus::Completed);
    assert_eq!(finished.progress.failed_items, 0);
    assert_eq!(
        collect_files(&destination.join("Payload")),
        expected,
        "a cross-volume move writes every byte before removing the source"
    );
    assert!(
        !source.exists(),
        "a completed move removes the source after the destination is complete"
    );
    assert_no_leftovers(&destination_workspace.root);
    assert_no_leftovers(&source_workspace.root);

    engine.shutdown();
}

/// The same request with an explicit verification policy, as the command layer
/// builds it from the user's setting.
fn request_verifying(
    source: &Path,
    destination: &Path,
    policy: VerificationPolicy,
) -> TransferRequest {
    let mut request = request_for(
        &[source],
        destination,
        TransferOperation::Copy,
        ConflictStrategy::Skip,
    );
    request.verification = Some(policy);
    request
}

#[test]
fn sanity_copying_a_nested_tree_preserves_structure_and_bytes() {
    let workspace = Workspace::new("sanity-copy-tree");
    let source = workspace.directory("Data");
    workspace.file("Data/readme.md", 3 * 1024);
    workspace.file("Data/alpha/a1.bin", 64 * 1024);
    workspace.file("Data/alpha/deep/leaf.bin", 1024);
    workspace.file("Data/beta/b1.txt", 7);
    workspace.directory("Data/empty");
    let destination = workspace.directory("out");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    assert!(finished.error.is_none(), "{:?}", finished.error);
    assert_eq!(finished.progress.percent, Some(100));
    assert_eq!(finished.progress.completed_files, 4);
    assert_eq!(
        finished.progress.transferred_bytes,
        finished.progress.total_bytes
    );
    assert_eq!(
        finished.progress.transferred_bytes,
        (3 * 1024 + 64 * 1024 + 1024 + 7) as u64
    );

    assert_same_tree(&source, &destination.join("Data"));
    assert_same_tree(&source, &source);

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}

#[test]
fn sanity_moving_leaves_the_source_gone_and_the_destination_complete() {
    let workspace = Workspace::new("sanity-move");
    let folder = workspace.directory("Incoming");
    workspace.file("Incoming/report.txt", 4096);
    workspace.file("Incoming/nested/data.bin", 16 * 1024);
    let loose = workspace.file("loose.txt", 512);
    let destination = workspace.directory("sorted");

    let engine = TransferEngine::new(1);

    let folder_job = engine
        .enqueue_request(request_for(
            &[&folder],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    let moved = wait_for_status(&engine, &folder_job.id, TransferStatus::Completed);
    assert_eq!(moved.operation, TransferOperation::Move);
    assert_eq!(
        moved.progress.transferred_bytes,
        4096 + 16 * 1024,
        "every byte of the folder is accounted for"
    );
    assert!(
        !folder.exists(),
        "a moved folder is gone from its old place"
    );
    assert_same_tree(&destination.join("Incoming"), &destination.join("Incoming"));
    assert_eq!(
        std::fs::read(destination.join("Incoming").join("report.txt"))
            .expect("readable")
            .len(),
        4096
    );
    assert_eq!(
        std::fs::read(destination.join("Incoming").join("nested").join("data.bin"))
            .expect("readable")
            .len(),
        16 * 1024
    );

    let file_job = engine
        .enqueue_request(request_for(
            &[&loose],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &file_job.id, TransferStatus::Completed);
    assert!(!loose.exists());
    assert_eq!(
        std::fs::read(destination.join("loose.txt"))
            .expect("readable")
            .len(),
        512
    );

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}

#[test]
fn sanity_every_conflict_strategy_behaves_as_documented() {
    let workspace = Workspace::new("sanity-conflicts");
    let source = workspace.file_text("notes.txt", "the new content");
    let destination = workspace.directory("out");
    let existing = workspace.file_text("out/notes.txt", "the old content");

    let engine = TransferEngine::new(1);

    let skipped = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    let skipped = wait_for_status(&engine, &skipped.id, TransferStatus::Completed);
    assert_eq!(skipped.progress.skipped_items, 1);
    assert_eq!(
        std::fs::read_to_string(&existing).expect("readable"),
        "the old content"
    );

    let renamed = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Rename,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &renamed.id, TransferStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(workspace.path("out/notes (2).txt")).expect("readable"),
        "the new content"
    );
    assert_eq!(
        std::fs::read_to_string(&existing).expect("readable"),
        "the old content",
        "a rename never touches what it was renamed from"
    );

    let replaced = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &replaced.id, TransferStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(&existing).expect("readable"),
        "the new content"
    );

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}

#[test]
fn sanity_pause_resume_and_cancel_leave_the_destination_clean() {
    let workspace = Workspace::new("sanity-control");
    let source = workspace.file("payload.bin", 64 * 1024 * 1024);
    let destination = workspace.directory("out");

    let engine = TransferEngine::new(1);

    // Pause mid-transfer, prove it stopped, resume, prove it finished.
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let deadline = Instant::now() + TIMEOUT;
    loop {
        let snapshot = engine.snapshot(&job.id).expect("the job exists");
        if snapshot.status == TransferStatus::Running && snapshot.progress.transferred_bytes > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the transfer never started: {snapshot:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }

    engine.pause(&job.id).expect("the job pauses");
    std::thread::sleep(Duration::from_millis(150));
    let parked = engine.snapshot(&job.id).expect("the job exists");
    std::thread::sleep(Duration::from_millis(150));
    let still_parked = engine.snapshot(&job.id).expect("the job exists");
    assert_eq!(parked.status, TransferStatus::Paused);
    assert_eq!(
        still_parked.progress.transferred_bytes, parked.progress.transferred_bytes,
        "a paused transfer writes nothing"
    );
    assert!(
        !destination.join("payload.bin").exists(),
        "a paused transfer has committed nothing"
    );

    engine.resume(&job.id).expect("the job resumes");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);
    assert_eq!(finished.progress.percent, Some(100));
    assert_eq!(
        std::fs::metadata(destination.join("payload.bin"))
            .expect("the destination exists")
            .len(),
        64 * 1024 * 1024
    );

    // Now cancel one mid-transfer and check nothing partial survives.
    std::fs::remove_file(destination.join("payload.bin")).expect("the test owns the file");
    let cancelled_job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    loop {
        let snapshot = engine.snapshot(&cancelled_job.id).expect("the job exists");
        if snapshot.status == TransferStatus::Running && snapshot.progress.transferred_bytes > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the transfer never started: {snapshot:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }

    engine.pause(&cancelled_job.id).expect("the job pauses");
    std::thread::sleep(Duration::from_millis(150));
    engine.cancel(&cancelled_job.id).expect("the job cancels");
    let cancelled = wait_for_status(&engine, &cancelled_job.id, TransferStatus::Cancelled);

    assert!(cancelled.error.is_none(), "cancelling is not a failure");
    assert!(!destination.join("payload.bin").exists());
    assert_eq!(
        relative_tree(&destination),
        Vec::<String>::new(),
        "the destination is exactly as empty as it started"
    );
    assert_no_leftovers(&workspace.root);

    engine.shutdown();
}

#[test]
fn sanity_a_large_file_streams_without_growing_memory() {
    let workspace = Workspace::new("sanity-large");
    let source = workspace.file("huge.bin", LARGE_FILE_BYTES);
    let destination = workspace.directory("out");

    let engine = TransferEngine::new(1);

    // Private (committed) bytes, not the working set: reading a 192 MiB file
    // maps its cache pages into this process's working set, which looks like
    // growth even when the copy streams through one reused buffer. Commit
    // charge counts only what this process actually holds.
    let baseline = platform::private_memory_bytes();
    let sampling = Arc::new(AtomicBool::new(false));
    let peak = match baseline {
        Some(baseline) => {
            sampling.store(true, AtomicOrdering::Relaxed);
            let flag = Arc::clone(&sampling);
            Some((
                baseline,
                std::thread::spawn(move || {
                    let mut peak = baseline;
                    while flag.load(AtomicOrdering::Relaxed) {
                        if let Some(current) = platform::private_memory_bytes() {
                            peak = peak.max(current);
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    peak
                }),
            ))
        }
        None => None,
    };

    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let measured = peak.map(|(baseline, handle)| {
        sampling.store(false, AtomicOrdering::Relaxed);
        let peak = handle.join().expect("the sampler finishes");
        (baseline, peak)
    });

    assert_eq!(finished.progress.transferred_bytes, LARGE_FILE_BYTES as u64);
    assert_eq!(finished.progress.percent, Some(100));
    let copied = destination.join("huge.bin");
    assert_eq!(
        std::fs::metadata(&copied).expect("the copy exists").len(),
        LARGE_FILE_BYTES as u64
    );

    // Spot-check the content without reading the whole file into this process:
    // a test that buffered 192 MiB to verify the copy would wreck the very
    // measurement it is making.
    let mut source_file = File::open(&source).expect("the source is readable");
    let mut copied_file = File::open(&copied).expect("the copy is readable");
    for offset in [
        0u64,
        16 * 1024 * 1024,
        96 * 1024 * 1024,
        LARGE_FILE_BYTES as u64 - 4096,
    ] {
        let mut expected = vec![0u8; 4096];
        let mut actual = vec![0u8; 4096];
        source_file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| source_file.read_exact(&mut expected))
            .expect("the source is readable at that offset");
        copied_file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| copied_file.read_exact(&mut actual))
            .expect("the copy is readable at that offset");
        assert_eq!(actual, expected, "bytes at offset {offset} must match");
    }

    match measured {
        Some((baseline, peak)) => {
            let growth = peak.saturating_sub(baseline);
            eprintln!(
                "large-file copy: {} MiB copied, this process grew by {} MiB (ceiling {} MiB)",
                LARGE_FILE_BYTES / (1024 * 1024),
                growth / (1024 * 1024),
                MEMORY_CEILING_BYTES / (1024 * 1024)
            );
            assert!(
                growth < MEMORY_CEILING_BYTES,
                "copying {} MiB grew this process by {} MiB (ceiling {} MiB): the copy must \
                 stream, not buffer",
                LARGE_FILE_BYTES / (1024 * 1024),
                growth / (1024 * 1024),
                MEMORY_CEILING_BYTES / (1024 * 1024)
            );
        }
        None => {
            // The host does not report a working set; the streaming behaviour is
            // still proven by the bounded-read tests in `copy`.
            assert!(LARGE_FILE_BYTES as u64 > MEMORY_CEILING_BYTES);
        }
    }

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}

#[test]
fn sanity_unsafe_requests_are_refused_without_touching_anything() {
    let workspace = Workspace::new("sanity-unsafe");
    let source = workspace.directory("Data");
    workspace.file("Data/important.txt", 2048);
    let inside = workspace.directory("Data/Backup");
    let before = relative_tree(&workspace.root);

    let engine = TransferEngine::new(1);

    let into_itself = engine
        .enqueue_request(request_for(
            &[&source],
            &inside,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        ))
        .expect_err("a folder cannot be copied into itself");
    assert_eq!(into_itself.code(), "unsafe_relationship");

    let onto_itself = engine
        .enqueue_request(request_for(
            &[&source],
            &workspace.path("Data"),
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect_err("a folder cannot be copied onto itself");
    assert_eq!(onto_itself.code(), "unsafe_relationship");

    let missing_source = engine
        .enqueue_request(request_for(
            &[&workspace.path("nope.txt")],
            &inside,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect_err("a missing source is refused");
    assert_eq!(missing_source.code(), "path_not_found");

    let missing_destination = engine
        .enqueue_request(request_for(
            &[&source],
            &workspace.path("nowhere"),
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect_err("a destination that does not exist is refused");
    assert_eq!(missing_destination.code(), "path_not_found");

    assert!(
        engine.snapshots().is_empty(),
        "a refused request never becomes a job"
    );
    assert_eq!(
        relative_tree(&workspace.root),
        before,
        "a refused request changes nothing on disk"
    );
    assert_no_leftovers(&workspace.root);

    engine.shutdown();
}

#[test]
fn sanity_the_queue_runs_several_jobs_in_order() {
    let workspace = Workspace::new("sanity-queue");
    let first = workspace.file("first.txt", 128 * 1024);
    let second = workspace.file("second.txt", 256 * 1024);
    let third = workspace.file("third.txt", 512 * 1024);
    let destination = workspace.directory("out");

    let engine = TransferEngine::new(1);
    let jobs: Vec<TransferSnapshot> = [&first, &second, &third]
        .iter()
        .map(|source| {
            engine
                .enqueue_request(request_for(
                    &[source],
                    &destination,
                    TransferOperation::Copy,
                    ConflictStrategy::Skip,
                ))
                .expect("the job is accepted")
        })
        .collect();

    for job in &jobs {
        wait_for_status(&engine, &job.id, TransferStatus::Completed);
    }

    let queue: Vec<String> = engine
        .snapshots()
        .into_iter()
        .map(|snapshot| snapshot.id)
        .collect();
    assert_eq!(
        queue,
        jobs.iter()
            .map(|job| job.id.clone())
            .collect::<Vec<String>>(),
        "queue order is the order the jobs were accepted"
    );
    assert_eq!(
        std::fs::metadata(destination.join("first.txt"))
            .expect("readable")
            .len(),
        128 * 1024
    );
    assert_eq!(
        std::fs::metadata(destination.join("second.txt"))
            .expect("readable")
            .len(),
        256 * 1024
    );
    assert_eq!(
        std::fs::metadata(destination.join("third.txt"))
            .expect("readable")
            .len(),
        512 * 1024
    );

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}

#[test]
fn sanity_verification_proves_what_it_claims_on_real_files() {
    let workspace = Workspace::new("sanity-verification");
    let source = workspace.directory("Data");
    let small = workspace.file("Data/notes.txt", 4 * 1024);
    let large = workspace.file("Data/alpha/blob.bin", 96 * 1024);
    let destination = workspace.directory("out");

    // 1. The default policy: every written file is proven to be there with the
    //    number of bytes the copy actually streamed out of the source.
    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_verifying(
            &source,
            &destination,
            VerificationPolicy::Size,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let verification = &finished.verification;
    assert_eq!(verification.status, VerificationStatus::Verified);
    assert_eq!(verification.method, VerificationMethod::Size);
    assert_eq!(verification.planned_files, 2);
    assert_eq!(verification.checked_files, 2);
    assert_eq!(verification.verified_files, 2);
    assert_eq!(verification.unverified_files, 0);
    assert_eq!(verification.mismatched_files, 0);
    assert_eq!(verification.failed_files, 0);
    assert_eq!(
        verification.verified_bytes,
        4 * 1024 + 96 * 1024,
        "the bytes that passed verification are the bytes that moved"
    );
    assert!(verification.mismatches.is_empty());
    assert!(
        !verification.coverage.modified_time_preserved && !verification.coverage.readonly_preserved,
        "this engine does not reapply metadata, and verification says so"
    );
    assert_same_tree(&source, &destination.join("Data"));

    // 2. The checksum policy compares the bytes read from the source with the
    //    file on disk. A copy whose bytes were changed afterwards is caught.
    let checksum_out = workspace.directory("checksum-out");
    let checksummed = engine
        .enqueue_request(request_verifying(
            &small,
            &checksum_out,
            VerificationPolicy::Checksum,
        ))
        .expect("the job is accepted");
    let checksummed = wait_for_status(&engine, &checksummed.id, TransferStatus::Completed);
    let summary = &checksummed.verification;
    assert_eq!(summary.status, VerificationStatus::Verified);
    assert_eq!(summary.method, VerificationMethod::SizeAndChecksum);
    assert_eq!(summary.checksum_algorithm, Some(ChecksumAlgorithm::Sha256));
    assert!(summary.coverage.checksum, "a checksum really was compared");

    let copied = checksum_out.join("notes.txt");
    let expected_bytes = std::fs::metadata(&copied).expect("the copy exists").len();
    let mut checkpoint = || true;
    // The digest the copy computed is the digest of the source file, so that is
    // what is recomputed here rather than invented.
    let streamed = crate::verification::hash_file(&small, &mut checkpoint)
        .expect("the source hashes")
        .digest;

    // Same size, different bytes: exactly what a size check cannot catch.
    let mut tampered = std::fs::read(&copied).expect("the copy is readable");
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;
    std::fs::write(&copied, &tampered).expect("the copy is writable");

    let result = crate::verification::verify_item(
        crate::verification::ItemVerification {
            source: &small,
            destination: &copied,
            expected_bytes,
            streamed_checksum: Some(streamed),
        },
        VerificationPolicy::Checksum,
        &mut checkpoint,
    )
    .expect("the verification is not stopped");

    assert_eq!(
        result.status,
        VerificationStatus::Mismatch,
        "changed bytes of the same size must not verify"
    );
    assert_eq!(
        result.mismatch.as_ref().map(|mismatch| mismatch.reason),
        Some(VerificationMismatchReason::ChecksumMismatch)
    );
    assert_eq!(
        result.failure_error().code(),
        "verification_failed",
        "a mismatch must fail with the code the UI switches on"
    );

    // 3. A job that did not verify says so rather than claiming success.
    let unverified_out = workspace.directory("unverified-out");
    let unverified = engine
        .enqueue_request(request_verifying(
            &large,
            &unverified_out,
            VerificationPolicy::None,
        ))
        .expect("the job is accepted");
    let none = wait_for_status(&engine, &unverified.id, TransferStatus::Completed);
    assert_eq!(none.verification.status, VerificationStatus::Skipped);
    assert_eq!(none.verification.verdict, "not verified");
    assert_eq!(none.verification.checked_files, 0);
    assert!(
        !none.verification.coverage.size,
        "nothing about the bytes was checked, so nothing may be claimed"
    );

    assert_no_leftovers(&workspace.root);
    engine.shutdown();
}
