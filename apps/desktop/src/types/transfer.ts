/* ==========================================================================
 * Transfer types
 * Mirrors the transfer engine's wire contract in
 * `src-tauri/src/transfer/model.rs`: the request the UI sends, the snapshot
 * and preview it receives, and the closed sets of operation, status, conflict,
 * item-kind, and issue-reason identifiers.
 *
 * The engine's internal planning state (items, roots, counters) does not cross
 * the IPC boundary. Everything here is a published contract, so a new backend
 * identifier must be added in both places.
 * ========================================================================== */

import { z } from "zod";

/** What a transfer does with each source. */
export const TRANSFER_OPERATIONS = ["copy", "move"] as const;
export type TransferOperation = (typeof TRANSFER_OPERATIONS)[number];

/** Lifecycle of one transfer job. */
export const TRANSFER_STATUSES = [
  "queued",
  "preparing",
  "running",
  "paused",
  "cancelling",
  "completed",
  "failed",
  "cancelled",
] as const;
export type TransferStatus = (typeof TRANSFER_STATUSES)[number];

/** How a destination collision is resolved. Applies to every item in a job. */
export const CONFLICT_STRATEGIES = ["replace", "skip", "rename"] as const;
export type ConflictStrategy = (typeof CONFLICT_STRATEGIES)[number];

/** The strategy a transfer uses when the user has not chosen one. */
export const DEFAULT_CONFLICT_STRATEGY: ConflictStrategy = "skip";

/** What a planned entry is. */
export const ITEM_KINDS = ["file", "directory"] as const;
export type ItemKind = (typeof ITEM_KINDS)[number];

/** Why an issue was recorded. Only `failed` fails a job. */
export const TRANSFER_ISSUE_REASONS = [
  "failed",
  "skipped",
  "unsupported",
] as const;
export type TransferIssueReason = (typeof TRANSFER_ISSUE_REASONS)[number];

/**
 * A structured backend failure carried inside a transfer snapshot or issue.
 *
 * The `code` is validated as a free string here and normalized through
 * `toIpcError` where it is displayed, so an unknown code loses no information
 * and a known one stays exhaustively switchable.
 */
export const transferErrorSchema = z.object({
  code: z.string().min(1),
  message: z.string(),
});

export type TransferError = z.infer<typeof transferErrorSchema>;

/**
 * A request to place one or more sources inside a destination directory.
 *
 * Paths are strings because that is what crosses the IPC boundary; the backend
 * validates them before anything is touched and the frontend never builds one
 * itself. `conflict` is required here even though the backend defaults it:
 * the UI always shows the strategy it is about to use.
 */
export const transferRequestSchema = z.object({
  sources: z.array(z.string().min(1)).min(1),
  destination: z.string().min(1),
  operation: z.enum(TRANSFER_OPERATIONS),
  conflict: z.enum(CONFLICT_STRATEGIES),
});

export type TransferRequest = z.infer<typeof transferRequestSchema>;

/** One thing the user should know about a job that is not plain progress. */
export const transferIssueSchema = z.object({
  /** Destination for failed items, source for skipped and unsupported ones. */
  path: z.string().min(1),
  reason: z.enum(TRANSFER_ISSUE_REASONS),
  /** Structured backend error, set for failures only. */
  error: transferErrorSchema.nullable(),
  /** Human-readable detail for skipped and unsupported items. */
  detail: z.string().nullable(),
});

export type TransferIssue = z.infer<typeof transferIssueSchema>;

/**
 * Progress as the backend reports it. Every number comes from work that
 * actually happened; `percent` is `null` only when the plan has nothing to
 * measure, which is not the same as zero.
 */
export const transferProgressSchema = z.object({
  totalBytes: z.number().int().nonnegative(),
  transferredBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
  completedFiles: z.number().int().nonnegative(),
  totalDirectories: z.number().int().nonnegative(),
  completedDirectories: z.number().int().nonnegative(),
  skippedItems: z.number().int().nonnegative(),
  skippedBytes: z.number().int().nonnegative(),
  failedItems: z.number().int().nonnegative(),
  /** File currently being written, or `null` between files. */
  currentFile: z.string().min(1).nullable(),
  currentFileBytes: z.number().int().nonnegative(),
  currentFileTotalBytes: z.number().int().nonnegative(),
  percent: z.number().int().min(0).max(100).nullable(),
  bytesPerSecond: z.number().int().nonnegative(),
  averageBytesPerSecond: z.number().int().nonnegative(),
  etaSeconds: z.number().int().nonnegative().nullable(),
  elapsedMs: z.number().int().nonnegative(),
});

export type TransferProgress = z.infer<typeof transferProgressSchema>;

/** One job as the frontend sees it. */
export const transferSnapshotSchema = z.object({
  /** Stable identifier for the life of the job. */
  id: z.string().min(1),
  operation: z.enum(TRANSFER_OPERATIONS),
  conflict: z.enum(CONFLICT_STRATEGIES),
  status: z.enum(TRANSFER_STATUSES),
  sources: z.array(z.string().min(1)).min(1),
  destination: z.string().min(1),
  progress: transferProgressSchema,
  /** Job-level failure. Per-item failures live in `issues`. */
  error: transferErrorSchema.nullable(),
  issues: z.array(transferIssueSchema),
  /** True when more issues happened than are listed. */
  issuesTruncated: z.boolean(),
  queuedAtMs: z.number().int().nonnegative(),
  startedAtMs: z.number().int().nonnegative().nullable(),
  finishedAtMs: z.number().int().nonnegative().nullable(),
});

export type TransferSnapshot = z.infer<typeof transferSnapshotSchema>;

/** One root of a preview, so the UI can show where each source lands. */
export const transferPreviewRootSchema = z.object({
  source: z.string().min(1),
  destination: z.string().min(1),
  kind: z.enum(ITEM_KINDS),
  /** True when this source will be left alone by the conflict strategy. */
  skipped: z.boolean(),
  files: z.number().int().nonnegative(),
  directories: z.number().int().nonnegative(),
  bytes: z.number().int().nonnegative(),
});

export type TransferPreviewRoot = z.infer<typeof transferPreviewRootSchema>;

/**
 * A dry run of a transfer request: what would be copied, where it would land,
 * and what would collide. Planning creates, moves, and deletes nothing.
 */
export const transferPreviewSchema = z.object({
  sources: z.array(z.string().min(1)).min(1),
  destination: z.string().min(1),
  operation: z.enum(TRANSFER_OPERATIONS),
  conflict: z.enum(CONFLICT_STRATEGIES),
  totalBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
  totalDirectories: z.number().int().nonnegative(),
  /** Files and directories whose destination already exists. */
  conflicts: z.number().int().nonnegative(),
  /** Items the conflict strategy would leave alone. */
  skippedItems: z.number().int().nonnegative(),
  skippedBytes: z.number().int().nonnegative(),
  /** Free space the host reports for the destination, when it reports any. */
  availableBytes: z.number().int().nonnegative().nullable(),
  /** Whether a move of the first source can be a rename instead of a copy. */
  sameVolume: z.boolean(),
  roots: z.array(transferPreviewRootSchema),
});

export type TransferPreview = z.infer<typeof transferPreviewSchema>;

/** Whether a job is finished and will never run again. */
export function isTerminalTransferStatus(status: TransferStatus): boolean {
  return (
    status === "completed" || status === "failed" || status === "cancelled"
  );
}

/** Whether a job is holding (or about to hold) engine resources. */
export function isLiveTransferStatus(status: TransferStatus): boolean {
  return !isTerminalTransferStatus(status);
}

/** Whether a job has left the queue and reached a worker. */
export function hasStarted(snapshot: TransferSnapshot): boolean {
  return snapshot.startedAtMs !== null;
}
