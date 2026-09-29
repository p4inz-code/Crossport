/* ==========================================================================
 * Measurement harness
 * Opt-in Windows performance measurements (`cargo test --lib measure --
 * --ignored --nocapture`). Every scenario builds its own files under a unique
 * temp directory, prints one `MEASURE <name>: …` line, and asserts only the
 * catastrophes a regression would cause — a measurement that fails on a busy
 * machine helps nobody, but a measurement that cannot finish at all does.
 *
 * The scenarios take a shared lock and run one at a time: a wall-clock or
 * process-CPU figure measured while another scenario hammers the same process
 * is not a measurement, and the parallel default would report a genuinely idle
 * engine as spinning.
 *
 * Compiled only for tests, so the release binary carries none of it.
 * ========================================================================== */

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::filesystem::test_support::unique_temp_dir;
use crate::history::{DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
use crate::transfer::model::{ConflictStrategy, TransferOperation, TransferRequest};
use crate::transfer::{plan, TransferEngine};
use crate::verification::VerificationPolicy;

/// Working set the process may add while streaming a large file. The sanity
/// suite proves the same property; here it is a measurement ceiling.
const LARGE_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MEMORY_CEILING_BYTES: u64 = 64 * 1024 * 1024;

/// Serializes the measurement scenarios against each other.
///
/// Every figure here is a wall-clock or process-wide CPU reading, and the test
/// harness runs a binary's tests in parallel by default. Left unguarded, the
/// idle-engine check counts the CPU the other scenarios spend beside it and
/// reports a quiet engine as spinning, and every timing is inflated by whatever
/// else happens to be running. The lock costs nothing on an ordinary
/// `cargo test`, because none of these scenarios run unless they are asked for.
static MEASURE_GUARD: Mutex<()> = Mutex::new(());

/// Takes the measurement lock, tolerating a scenario that panicked while
/// holding it so one failure does not wedge the rest of the suite.
fn measure_guard() -> std::sync::MutexGuard<'static, ()> {
    MEASURE_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn report(name: &str, detail: &str, started: Instant) {
    println!(
        "MEASURE {name}: {}ms ({detail})",
        started.elapsed().as_millis()
    );
}

fn write_file_bytes(path: &Path, bytes: u64) {
    use std::io::Write;
    let mut file = std::fs::File::create(path).expect("the file is created");
    let buffer = vec![0x5au8; 1024 * 1024];
    let mut left = bytes;
    while left > 0 {
        let chunk = left.min(buffer.len() as u64) as usize;
        file.write_all(&buffer[..chunk])
            .expect("the file is written");
        left -= chunk as u64;
    }
    file.sync_all().expect("the file is flushed");
}

fn request(sources: Vec<String>, destination: &Path) -> TransferRequest {
    TransferRequest {
        sources,
        destination: destination.display().to_string(),
        operation: TransferOperation::Copy,
        conflict: ConflictStrategy::Replace,
        verification: Some(VerificationPolicy::Size),
    }
}

/// Runs a request on a fresh engine and waits for a terminal status.
fn run_to_completion(request: TransferRequest) -> (String, u64) {
    let engine = TransferEngine::new(1);
    let started = Instant::now();
    let queued = engine
        .enqueue_request(request)
        .expect("the request is planned and queued");

    loop {
        let snapshot = engine.snapshot(&queued.id).expect("the job exists");
        if snapshot.status.is_terminal() {
            let elapsed = started.elapsed();
            let status = snapshot.status.as_str().to_string();
            engine.shutdown();
            return (status, elapsed.as_millis() as u64);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn populate_directory(directory: &Path, files: usize, bytes_each: u64) {
    std::fs::create_dir_all(directory).expect("the directory is created");
    for index in 0..files {
        write_file_bytes(&directory.join(format!("item-{index:05}.bin")), bytes_each);
    }
}

/// Lists a directory of `count` entries, printing both a cold and a warm run.
#[test]
#[ignore = "opt-in measurement"]
fn measure_directory_listing() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-listing");
    let wanted = workspace.join("many");
    populate_directory(&wanted, 10_000, 0);

    let started = Instant::now();
    let listing = crate::filesystem::directory::list(&wanted).expect("the folder lists");
    report(
        "directory listing (10000 entries, cold)",
        &format!("{} entries", listing.entries.len()),
        started,
    );

    let started = Instant::now();
    let listing = crate::filesystem::directory::list(&wanted).expect("the folder lists");
    report(
        "directory listing (10000 entries, warm)",
        &format!("{} entries", listing.entries.len()),
        started,
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

/// Plans a deep tree: a chain of nested folders, each holding a few files.
#[test]
#[ignore = "opt-in measurement"]
fn measure_deep_tree_plan() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-deep");
    let root = workspace.join("deep");
    let mut current = root.clone();
    for depth in 0..500 {
        current = current.join(format!("level-{depth:03}"));
        std::fs::create_dir_all(&current).expect("the level exists");
        for file in 0..4 {
            write_file_bytes(&current.join(format!("file-{file}.bin")), 1024);
        }
    }
    let destination = workspace.join("target");
    std::fs::create_dir_all(&destination).expect("the destination exists");

    let started = Instant::now();
    let planned = plan::plan_request(&request(vec![root.display().to_string()], &destination))
        .expect("the tree is planned");
    report(
        "deep tree plan (500 levels, 2000 files)",
        &format!(
            "{} files, {} directories",
            planned.total_files, planned.total_directories
        ),
        started,
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

/// Copies many small files: the workload where per-file costs dominate.
#[test]
#[ignore = "opt-in measurement"]
fn measure_many_small_files() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-small");
    let source = workspace.join("small");
    populate_directory(&source, 2_000, 4 * 1024);
    let destination = workspace.join("target");
    std::fs::create_dir_all(&destination).expect("the destination exists");

    let (status, ms) = run_to_completion(request(vec![source.display().to_string()], &destination));
    let bytes = 2_000u64 * 4 * 1024;
    let mb_per_second = if ms > 0 {
        (bytes as f64 / (1024.0 * 1024.0)) / (ms as f64 / 1000.0)
    } else {
        0.0
    };
    println!(
        "MEASURE small files (2000 x 4 KiB): {ms}ms ({status}, {mb_per_second:.1} MiB/s, {:.2}ms/file)",
        ms as f64 / 2_000.0
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

/// Copies one large file and measures throughput and the memory it added.
#[test]
#[ignore = "opt-in measurement"]
fn measure_large_file_streaming() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-large");
    let source = workspace.join("large");
    std::fs::create_dir_all(&source).expect("the source exists");
    let file = source.join("payload.bin");
    write_file_bytes(&file, LARGE_FILE_BYTES);

    let destination = workspace.join("target");
    std::fs::create_dir_all(&destination).expect("the destination exists");

    let baseline = crate::platform::private_memory_bytes();
    let mut peak = baseline;
    let engine = TransferEngine::new(1);
    let started = Instant::now();
    let queued = engine
        .enqueue_request(request(vec![file.display().to_string()], &destination))
        .expect("the file is planned and queued");

    loop {
        if let (Some(before), Some(now)) = (baseline, crate::platform::private_memory_bytes()) {
            if now > before {
                peak = Some(peak.map_or(now, |highest| highest.max(now)));
            }
        }
        let snapshot = engine.snapshot(&queued.id).expect("the job exists");
        if snapshot.status.is_terminal() {
            let ms = started.elapsed().as_millis() as u64;
            let mib = LARGE_FILE_BYTES as f64 / (1024.0 * 1024.0);
            let mib_per_second = if ms > 0 {
                mib / (ms as f64 / 1000.0)
            } else {
                0.0
            };
            let growth = peek_growth(baseline, peak);
            println!(
                "MEASURE large file (512 MiB): {ms}ms ({}, {mib_per_second:.0} MiB/s, private memory +{} MiB)",
                snapshot.status.as_str(),
                growth / (1024 * 1024)
            );
            assert!(
                growth <= MEMORY_CEILING_BYTES,
                "streaming a 512 MiB file grew private memory by {growth} bytes"
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    engine.shutdown();
    let _ = std::fs::remove_dir_all(&workspace);
}

fn peek_growth(baseline: Option<u64>, peak: Option<u64>) -> u64 {
    match (baseline, peak) {
        (Some(before), Some(highest)) => highest.saturating_sub(before),
        _ => 0,
    }
}

/// Writes a full history at two retention bounds, then measures the cost of
/// opening the archive that holds it.
///
/// Every `record()` persists the whole document — serialize, write, flush,
/// rename — because that is the durability contract: a finished job is on disk
/// before its live state is forgotten, and a crash can never expose a partial
/// file. The cost of writing one record therefore grows with how many are
/// retained, which makes the bound the number that matters. Two are measured:
/// the default a fresh install uses, and the largest a user can configure.
/// This is a per-finished-transfer cost, not a startup cost.
#[test]
#[ignore = "opt-in measurement"]
fn measure_archive_with_full_history() {
    let _serial = measure_guard();
    for limit in [DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT] {
        let workspace = unique_temp_dir(&format!("measure-archive-{limit}"));
        let (archive, _, _) = crate::archive::TransferArchive::open(&workspace, limit);

        let started = Instant::now();
        for index in 0..limit as usize {
            archive
                .history()
                .record(sample_record(index))
                .expect("the record is written");
        }
        let counted = archive.history().len();
        let total_ms = started.elapsed().as_millis() as u64;
        println!(
            "MEASURE history record (bound {limit}, {counted} records): {total_ms}ms ({:.1}ms/record)",
            total_ms as f64 / counted as f64
        );

        let bytes = std::fs::metadata(workspace.join(crate::history::HISTORY_FILE))
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        drop(archive);

        let started = Instant::now();
        let (reopened, status, _) = crate::archive::TransferArchive::open(&workspace, limit);
        report(
            &format!("archive open (bound {limit})"),
            &format!(
                "{} records, {} KiB on disk, load={}",
                reopened.history().len(),
                bytes / 1024,
                status.as_str()
            ),
            started,
        );

        let started = Instant::now();
        let listed = reopened.history_records(crate::history::HistoryFilter::All);
        report(
            &format!("history listing (bound {limit})"),
            &format!("{} records", listed.len()),
            started,
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }
}

fn sample_progress() -> crate::transfer::model::TransferProgress {
    crate::transfer::model::TransferCounters {
        total_bytes: 4096,
        transferred_bytes: 4096,
        total_files: 1,
        completed_files: 1,
        total_directories: 0,
        completed_directories: 0,
        skipped_items: 0,
        skipped_bytes: 0,
        failed_items: 0,
        current_file: None,
        current_file_bytes: 0,
        current_file_total_bytes: 0,
    }
    .to_progress(
        crate::transfer::model::TransferTiming {
            elapsed_ms: 400,
            bytes_per_second: 0,
            average_bytes_per_second: 0,
            eta_seconds: None,
        },
        crate::transfer::model::TransferActivity::Transferring,
    )
}

fn sample_record(index: usize) -> crate::history::HistoryRecord {
    let snapshot = crate::transfer::model::TransferSnapshot {
        id: format!("transfer-{index}"),
        operation: TransferOperation::Copy,
        conflict: ConflictStrategy::Replace,
        status: crate::transfer::model::TransferStatus::Completed,
        verification: crate::verification::VerificationLog::default().summary(true),
        sources: vec![format!("C:\\source\\item-{index}.bin")],
        destination: "D:\\Backup".to_string(),
        progress: sample_progress(),
        error: None,
        issues: Vec::new(),
        issues_truncated: false,
        queued_at_ms: 1_700_000_000_000,
        started_at_ms: Some(1_700_000_000_100),
        finished_at_ms: Some(1_700_000_000_500),
    };
    crate::archive::record_from_snapshot(&snapshot)
}

/// How deep a tree this Windows host can actually create and copy.
///
/// Windows still enforces a 260-character path limit unless both the manifest
/// opts in and the machine enables long paths, so "how deep can a transfer go"
/// is a platform fact worth measuring rather than assuming.
#[test]
#[ignore = "opt-in measurement"]
fn measure_path_length_limit() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-depth");
    let mut current = workspace.join("deep");
    std::fs::create_dir_all(&current).expect("the first level is creatable");
    for depth in 0..200 {
        let next = current.join(format!("nested-level-{depth:03}"));
        if std::fs::create_dir_all(&next).is_err() {
            break;
        }
        current = next;
    }
    let deepest = current.display().to_string().len();
    let file = current.join("payload.bin");
    let file_written = write_file_bytes_checked(&file, 1024);
    println!(
        "MEASURE deepest directory created: {deepest} characters, file written: {file_written}"
    );

    let destination = workspace.join("target");
    std::fs::create_dir_all(&destination).expect("the destination exists");
    let source_root = workspace.join("deep");
    let (status, ms) = run_to_completion(request(
        vec![source_root.display().to_string()],
        &destination,
    ));
    println!("MEASURE deep tree copy: {ms}ms ({status})");
    let _ = std::fs::remove_dir_all(&workspace);
}

fn write_file_bytes_checked(path: &Path, bytes: u64) -> bool {
    use std::io::Write;
    let Ok(mut file) = std::fs::File::create(path) else {
        return false;
    };
    file.write_all(&vec![0x11u8; bytes as usize]).is_ok()
}

/// Starts a fresh archive with no documents: what a first-run startup costs.
#[test]
#[ignore = "opt-in measurement"]
fn measure_archive_cold_start() {
    let _serial = measure_guard();
    let workspace = unique_temp_dir("measure-cold");
    let started = Instant::now();
    let (archive, history_status, state_status) =
        crate::archive::TransferArchive::open(&workspace, 200);
    report(
        "archive open (first run, no documents)",
        &format!(
            "history={}, state={}",
            history_status.as_str(),
            state_status.as_str()
        ),
        started,
    );
    drop(archive);
    let _ = std::fs::remove_dir_all(&workspace);
}

/// An idle engine must not spin: its worker sleeps on a condition variable
/// between polls, so a second of idling should cost almost no CPU time.
#[test]
#[ignore = "opt-in measurement"]
fn measure_idle_engine_cpu() {
    // Holds the measurement lock: this figure is only meaningful while no other
    // scenario is running in the same process.
    let _serial = measure_guard();
    let engine = TransferEngine::new(1);
    let Some(start_cpu) = process_cpu_time() else {
        println!("MEASURE idle engine CPU: host did not report process times");
        engine.shutdown();
        return;
    };

    std::thread::sleep(Duration::from_secs(2));

    let Some(end_cpu) = process_cpu_time() else {
        engine.shutdown();
        return;
    };
    let used = end_cpu.saturating_sub(start_cpu);
    println!(
        "MEASURE idle engine CPU: {used}ms of CPU over 2s wall ({:.2}% of one core)",
        (used as f64 / 2000.0) * 100.0
    );
    assert!(
        used < 500,
        "an idle engine used {used}ms of CPU in two seconds: it is spinning"
    );
    engine.shutdown();
}

#[cfg(windows)]
fn process_cpu_time() -> Option<u64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    // SAFETY: the pseudo handle for the current process is always valid and
    // the four FILETIME outputs are valid for the duration of the call.
    unsafe {
        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        let succeeded = GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        );
        if succeeded == 0 {
            return None;
        }
        let to_ms = |time: FILETIME| {
            let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
            ticks / 10_000
        };
        Some(to_ms(kernel) + to_ms(user))
    }
}

#[cfg(not(windows))]
fn process_cpu_time() -> Option<u64> {
    None
}
