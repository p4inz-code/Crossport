/* ==========================================================================
 * Test fixtures
 * Backend payloads used by more than one test suite, so a contract change is
 * made in one place instead of drifting between tests.
 * ========================================================================== */

import type {
  DirectoryEntry,
  DirectoryListing,
  DriveInfo,
  HistoryRecord,
  RecoveryCandidate,
  TransferIssue,
  TransferPreview,
  TransferPreviewRoot,
  TransferProgress,
  TransferSnapshot,
  VerificationSummary,
} from "@/types";

const GIB = 1024 ** 3;

/** A mounted NTFS system volume; override anything a test cares about. */
export function makeVolume(overrides: Partial<DriveInfo> = {}): DriveInfo {
  return {
    id: "C:",
    root: "C:\\",
    label: "C:",
    name: null,
    kind: "fixed",
    filesystem: "NTFS",
    totalBytes: 500 * GIB,
    freeBytes: 120 * GIB,
    usedBytes: 380 * GIB,
    readonly: false,
    mounted: true,
    ...overrides,
  };
}

/** A file entry; override `kind`/`sizeBytes` for directories and links. */
export function makeEntry(
  overrides: Partial<DirectoryEntry> = {},
): DirectoryEntry {
  return {
    name: "notes.txt",
    path: "C:\\notes.txt",
    kind: "file",
    sizeBytes: 2048,
    modifiedMs: 1_700_000_000_000,
    readonly: false,
    ...overrides,
  };
}

/** Progress as the backend reports it; override whatever a test cares about. */
export function makeTransferProgress(
  overrides: Partial<TransferProgress> = {},
): TransferProgress {
  return {
    totalBytes: 4096,
    transferredBytes: 1024,
    totalFiles: 2,
    completedFiles: 1,
    totalDirectories: 1,
    completedDirectories: 1,
    skippedItems: 0,
    skippedBytes: 0,
    failedItems: 0,
    currentFile: "D:\\Backup\\Photos\\trip.jpg",
    currentFileBytes: 1024,
    currentFileTotalBytes: 2048,
    percent: 25,
    bytesPerSecond: 1024,
    averageBytesPerSecond: 1024,
    etaSeconds: 3,
    elapsedMs: 1000,
    activity: "transferring",
    ...overrides,
  };
}

/** A verification summary; override whatever a test cares about. */
export function makeVerificationSummary(
  overrides: Partial<VerificationSummary> = {},
): VerificationSummary {
  return {
    status: "pending",
    policy: "size",
    method: "size",
    checksumAlgorithm: null,
    plannedFiles: 2,
    checkedFiles: 0,
    verifiedFiles: 0,
    mismatchedFiles: 0,
    failedFiles: 0,
    skippedFiles: 0,
    unverifiedFiles: 2,
    verifiedBytes: 0,
    durationMs: 0,
    coverage: {
      size: true,
      structure: true,
      checksum: false,
      modifiedTimePreserved: false,
      readonlyPreserved: false,
    },
    mismatches: [],
    mismatchesTruncated: false,
    error: null,
    verdict: "verification has not started",
    ...overrides,
  };
}

/** One issue a job reported. */
export function makeTransferIssue(
  overrides: Partial<TransferIssue> = {},
): TransferIssue {
  return {
    path: "D:\\Backup\\Photos\\notes.txt",
    reason: "skipped",
    error: null,
    detail: "destination already exists",
    ...overrides,
  };
}

/** A queue snapshot; override anything a test cares about. */
export function makeTransferSnapshot(
  overrides: Partial<TransferSnapshot> = {},
): TransferSnapshot {
  return {
    id: "transfer-1700000000000-1",
    operation: "copy",
    conflict: "skip",
    status: "running",
    sources: ["D:\\Photos"],
    destination: "D:\\Backup",
    progress: makeTransferProgress(),
    verification: makeVerificationSummary(),
    error: null,
    issues: [],
    issuesTruncated: false,
    queuedAtMs: 1_700_000_000_000,
    startedAtMs: 1_700_000_000_100,
    finishedAtMs: null,
    ...overrides,
  };
}

/** One root of a transfer preview. */
export function makeTransferPreviewRoot(
  overrides: Partial<TransferPreviewRoot> = {},
): TransferPreviewRoot {
  return {
    source: "D:\\Photos",
    destination: "D:\\Backup\\Photos",
    kind: "directory",
    skipped: false,
    files: 2,
    directories: 1,
    bytes: 4096,
    ...overrides,
  };
}

/** A dry run of a transfer request. */
export function makeTransferPreview(
  overrides: Partial<TransferPreview> = {},
): TransferPreview {
  return {
    sources: ["D:\\Photos"],
    destination: "D:\\Backup",
    operation: "copy",
    conflict: "skip",
    totalBytes: 4096,
    totalFiles: 2,
    totalDirectories: 1,
    conflicts: 0,
    skippedItems: 0,
    skippedBytes: 0,
    availableBytes: 8 * GIB,
    sameVolume: true,
    roots: [makeTransferPreviewRoot()],
    ...overrides,
  };
}

/** One history record; override whatever a test cares about. */
export function makeHistoryRecord(
  overrides: Partial<HistoryRecord> = {},
): HistoryRecord {
  return {
    id: "transfer-1700000000000-1",
    operation: "copy",
    conflict: "skip",
    status: "completed",
    sources: ["D:\\Photos"],
    destination: "D:\\Backup",
    totalBytes: 4096,
    transferredBytes: 4096,
    totalFiles: 2,
    completedFiles: 2,
    totalDirectories: 1,
    completedDirectories: 1,
    skippedItems: 0,
    failedItems: 0,
    queuedAtMs: 1_700_000_000_000,
    startedAtMs: 1_700_000_000_100,
    finishedAtMs: 1_700_000_002_000,
    durationMs: 1900,
    error: null,
    issuesTruncated: false,
    issues: [],
    verification: {
      status: "verified",
      method: "size",
      policy: "size",
      checksumAlgorithm: null,
      checkedFiles: 2,
      verifiedFiles: 2,
      mismatchedFiles: 0,
      failedFiles: 0,
      verifiedBytes: 4096,
      verdict: "verified (size, 2 files, 4096 bytes)",
    },
    recovery: null,
    recoveredFrom: null,
    ...overrides,
  };
}

/** One interrupted transfer recovery reported; override what matters. */
export function makeRecoveryCandidate(
  overrides: Partial<RecoveryCandidate> = {},
): RecoveryCandidate {
  return {
    id: "transfer-1700000000000-2",
    operation: "copy",
    conflict: "skip",
    verification: "size",
    sources: ["D:\\Photos"],
    destination: "D:\\Backup",
    status: "running",
    queuedAtMs: 1_700_000_000_000,
    startedAtMs: 1_700_000_000_100,
    updatedAtMs: 1_700_000_000_900,
    progress: {
      totalBytes: 4096,
      transferredBytes: 1024,
      totalFiles: 2,
      completedFiles: 1,
      totalDirectories: 1,
      completedDirectories: 1,
      skippedItems: 0,
      failedItems: 0,
    },
    percent: 25,
    verificationSummary: null,
    outcome: "restart_required",
    detail:
      "The transfer stopped after reporting 1024 of 4096 bytes. Restarting starts every file from the beginning.",
    artifacts: [
      {
        path: "D:\\Backup\\.crossport-transfer-1700000000000-2-0.partial",
        bytes: 1024,
      },
    ],
    artifactBytes: 1024,
    artifactsTruncated: false,
    artifactDirectories: ["D:\\Backup"],
    restartImpact: {
      strategy: "skip",
      conflicts: 0,
      skippedItems: 0,
      totalBytes: 4096,
      totalFiles: 2,
      overwrites: false,
    },
    destinationLooksComplete: null,
    confirmedByArchive: false,
    canRestart: true,
    canDiscard: true,
    ...overrides,
  };
}

/** A listing with a folder and a file, both on the system volume. */
export function makeListing(
  overrides: Partial<DirectoryListing> = {},
): DirectoryListing {
  return {
    path: "C:\\",
    name: "C:\\",
    parent: null,
    entries: [
      makeEntry({
        name: "Users",
        path: "C:\\Users",
        kind: "directory",
        sizeBytes: null,
      }),
      makeEntry({ name: "notes.txt", path: "C:\\notes.txt" }),
    ],
    truncated: false,
    ...overrides,
  };
}
