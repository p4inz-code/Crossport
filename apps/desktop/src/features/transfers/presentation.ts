/* ==========================================================================
 * Transfers presentation helpers
 * Framework-free mapping from backend values to what the transfer surface
 * shows: wording for every status, operation, conflict strategy, and issue
 * reason, plus the formatting of bytes, speed, time, and progress. Kept out of
 * the components so it is directly testable and so no component invents its
 * own wording for a backend value.
 *
 * Records are keyed by the backend enum, so a new status or reason cannot be
 * added to the contract without being given wording here.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  ArrowRightLeft,
  Ban,
  CheckCircle2,
  CircleSlash,
  Clock,
  Copy,
  FolderInput,
  Info,
  Loader,
  PauseCircle,
  TriangleAlert,
  XCircle,
} from "lucide-react";

import { formatBytes } from "@/lib";
import { type IpcError, toIpcError } from "@/services/ipc";
import type {
  ConflictStrategy,
  TransferIssue,
  TransferIssueReason,
  TransferOperation,
  TransferProgress,
  TransferSnapshot,
  TransferStatus,
} from "@/types";

/** Wording for each job status. */
export const TRANSFER_STATUS_LABELS: Record<TransferStatus, string> = {
  queued: "Waiting",
  preparing: "Preparing",
  running: "Transferring",
  paused: "Paused",
  cancelling: "Cancelling",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
};

/** How loud each status is, so the surface styles them consistently. */
export type TransferStatusTone =
  | "neutral"
  | "active"
  | "success"
  | "warning"
  | "danger";

export const TRANSFER_STATUS_TONES: Record<TransferStatus, TransferStatusTone> =
  {
    queued: "neutral",
    preparing: "active",
    running: "active",
    paused: "warning",
    cancelling: "warning",
    completed: "success",
    failed: "danger",
    cancelled: "neutral",
  };

/** Icon for each status, keyed the same way. */
export const TRANSFER_STATUS_ICONS: Record<TransferStatus, LucideIcon> = {
  queued: Clock,
  preparing: Loader,
  running: Loader,
  paused: PauseCircle,
  cancelling: Loader,
  completed: CheckCircle2,
  failed: XCircle,
  cancelled: CircleSlash,
};

/** Wording for each operation. */
export const TRANSFER_OPERATION_LABELS: Record<TransferOperation, string> = {
  copy: "Copy",
  move: "Move",
};

/** Icon for each operation, keyed the same way. */
export const TRANSFER_OPERATION_ICONS: Record<TransferOperation, LucideIcon> = {
  copy: Copy,
  move: FolderInput,
};

/** Wording for each conflict strategy, as shown in the composer. */
export const CONFLICT_STRATEGY_LABELS: Record<ConflictStrategy, string> = {
  replace: "Replace what is already there",
  skip: "Skip items that already exist",
  rename: "Keep both and rename the new item",
};

/** One-line explanation of each conflict strategy. */
export const CONFLICT_STRATEGY_DESCRIPTIONS: Record<ConflictStrategy, string> =
  {
    replace: "Existing files are overwritten. Nothing is removed for you.",
    skip: "Existing items are left exactly as they are. Nothing is lost.",
    rename: "The copy is written beside the existing item with a new name.",
  };

/** Wording for each issue reason. */
export const TRANSFER_ISSUE_LABELS: Record<TransferIssueReason, string> = {
  failed: "Failed",
  skipped: "Skipped",
  unsupported: "Not supported",
};

/** Icon for each issue reason, keyed the same way. */
export const TRANSFER_ISSUE_ICONS: Record<TransferIssueReason, LucideIcon> = {
  failed: TriangleAlert,
  skipped: Info,
  unsupported: Ban,
};

/** Whether a job can be parked right now. */
export function canPause(snapshot: TransferSnapshot): boolean {
  return (
    snapshot.status === "queued" ||
    snapshot.status === "preparing" ||
    snapshot.status === "running"
  );
}

/** Whether a job can be continued right now. */
export function canResume(snapshot: TransferSnapshot): boolean {
  return snapshot.status === "paused";
}

/** Whether a job can still be stopped with its partial output discarded. */
export function canCancel(snapshot: TransferSnapshot): boolean {
  return snapshot.status !== "cancelling" && !isFinished(snapshot);
}

/** Whether the job is finished and only removable. */
export function isFinished(snapshot: TransferSnapshot): boolean {
  return (
    snapshot.status === "completed" ||
    snapshot.status === "failed" ||
    snapshot.status === "cancelled"
  );
}

/** Whether data is moving right now. */
export function isMoving(snapshot: TransferSnapshot): boolean {
  return snapshot.status === "running" || snapshot.status === "preparing";
}

/** The last segment of a backend path, for display only. */
export function basename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const separator = Math.max(
    trimmed.lastIndexOf("/"),
    trimmed.lastIndexOf("\\"),
  );
  return separator === -1 ? trimmed : trimmed.slice(separator + 1);
}

/**
 * What a job is about, e.g. `Photos` or `Photos + 2 more`. The first source is
 * named because that is what the user picked; the rest are counted.
 */
export function transferTitle(snapshot: TransferSnapshot): string {
  const first = snapshot.sources.at(0);
  const name = first === undefined ? "Transfer" : basename(first);
  const extra = snapshot.sources.length - 1;
  return extra > 0 ? `${name} + ${extra} more` : name;
}

/** `3 of 10 files · 2 of 4 folders`, omitting what the plan does not have. */
export function transferCountsLabel(progress: TransferProgress): string {
  const parts: string[] = [];
  if (progress.totalFiles > 0) {
    parts.push(`${progress.completedFiles} of ${progress.totalFiles} files`);
  }
  if (progress.totalDirectories > 0) {
    parts.push(
      `${progress.completedDirectories} of ${progress.totalDirectories} folders`,
    );
  }
  return parts.length > 0 ? parts.join(" · ") : "No items to move";
}

/** `12 MB of 40 MB`, or an honest statement that there is nothing to weigh. */
export function transferBytesLabel(progress: TransferProgress): string {
  if (progress.totalBytes === 0) {
    return "No file data";
  }
  return `${formatBytes(progress.transferredBytes)} of ${formatBytes(progress.totalBytes)}`;
}

/** Bytes per second, e.g. `12.3 MB/s`. */
export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** A countdown such as `1m 20s left`, or `Unknown` when it cannot be said. */
export function formatEta(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) {
    return "Unknown";
  }
  if (seconds < 60) {
    return `${Math.round(seconds)}s left`;
  }
  if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const rest = Math.round(seconds % 60);
    return rest === 0 ? `${minutes}m left` : `${minutes}m ${rest}s left`;
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return minutes === 0 ? `${hours}h left` : `${hours}h ${minutes}m left`;
}

/** A duration such as `1m 20s`, used for elapsed time. */
export function formatDuration(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || milliseconds <= 0) {
    return "0s";
  }
  const seconds = Math.floor(milliseconds / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }
  if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const rest = seconds % 60;
    return rest === 0 ? `${minutes}m` : `${minutes}m ${rest}s`;
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}

/**
 * Text for one issue: the backend's message for a failure, the engine's own
 * explanation for skips, and a truthful fallback when neither is present.
 */
export function issueLabel(issue: TransferIssue): string {
  if (issue.reason === "failed") {
    return issue.error === null
      ? "This item could not be transferred."
      : issue.error.message;
  }
  if (issue.detail !== null && issue.detail.length > 0) {
    return issue.detail;
  }
  return issue.reason === "skipped"
    ? "Left alone because something is already there."
    : "This item cannot be copied.";
}

/** The structured error behind a failed item, for code-aware display. */
export function issueError(issue: TransferIssue): IpcError | null {
  return issue.error === null ? null : toIpcError(issue.error);
}

/** One issue paired with a key that identifies it rather than its position. */
export interface KeyedIssue {
  key: string;
  issue: TransferIssue;
}

/**
 * Pairs each issue with a stable key for rendering.
 *
 * Issues carry no identifier of their own, and the same path can legitimately
 * appear more than once — a directory skipped because a file of that name is
 * in the way, then a different item failing underneath it. So a key is built
 * from the issue's own content plus how many identical issues preceded it,
 * which keeps a key attached to the issue instead of to its row. That matters
 * because the visible list is a prefix of the reported one, so row numbers
 * shift as more issues arrive.
 */
export function keyedIssues(issues: readonly TransferIssue[]): KeyedIssue[] {
  const occurrences = new Map<string, number>();

  return issues.map((issue) => {
    const identity = `${issue.path}\u0000${issue.reason}\u0000${issue.error?.code ?? ""}`;
    const seen = occurrences.get(identity) ?? 0;
    occurrences.set(identity, seen + 1);
    return { key: seen === 0 ? identity : `${identity}\u0000${seen}`, issue };
  });
}

/** A job-level failure normalized for display. */
export function jobError(snapshot: TransferSnapshot): IpcError | null {
  return snapshot.error === null ? null : toIpcError(snapshot.error);
}

/** How a completed job ended, e.g. `Completed with 2 skipped`. */
export function transferOutcomeLabel(snapshot: TransferSnapshot): string {
  const { progress } = snapshot;
  const notes: string[] = [];
  if (progress.failedItems > 0) {
    notes.push(`${progress.failedItems} failed`);
  }
  if (progress.skippedItems > 0) {
    notes.push(`${progress.skippedItems} skipped`);
  }
  return notes.length === 0 ? "No problems reported" : notes.join(" · ");
}

/** Icon used when a queue has nothing in it. */
export const EMPTY_QUEUE_ICON = ArrowRightLeft;
