/* ==========================================================================
 * Transfer history types
 * Mirrors `src-tauri/src/history/mod.rs`: one record per finished job, one per
 * interrupted episode recovery accounted for, and the filter the list is
 * selected with.
 *
 * History is the durable account of what happened, so a record stores plain
 * values — paths, byte counts, timestamps, status identifiers — and never a
 * reference to anything live. `verification` is nullable because a record from
 * an older document may predate verification summaries; it is never assumed to
 * have passed.
 * ========================================================================== */

import { z } from "zod";

import { documentStatusSchema } from "./archive";
import {
  conflictStrategySchema,
  transferErrorSchema,
  transferIssueReasonSchema,
  transferOperationSchema,
} from "./transfer";
import {
  checksumAlgorithmSchema,
  verificationMethodSchema,
  verificationPolicySchema,
  verificationStatusSchema,
} from "./verification";

/** How a transfer ended, as history remembers it. */
export const HISTORY_STATUSES = [
  "completed",
  "failed",
  "cancelled",
  "interrupted",
  "recovered",
] as const;
export type HistoryStatus = (typeof HISTORY_STATUSES)[number];

export const historyStatusSchema = z.enum(HISTORY_STATUSES);

/** Which records a caller wants. */
export const HISTORY_FILTERS = [
  "all",
  "completed",
  "failed",
  "cancelled",
  "interrupted",
] as const;
export type HistoryFilter = (typeof HISTORY_FILTERS)[number];

export const historyFilterSchema = z.enum(HISTORY_FILTERS);

/** What was decided about an interrupted transfer. */
export const RECOVERY_ACTIONS = [
  "pending",
  "discard",
  "restart",
  "confirm",
] as const;
export type RecoveryAction = (typeof RECOVERY_ACTIONS)[number];

export const recoveryActionSchema = z.enum(RECOVERY_ACTIONS);

/**
 * Verification as history keeps it: the verdict, without the per-file detail a
 * live job carries while it is still collecting mismatches.
 */
export const historyVerificationSchema = z.object({
  status: verificationStatusSchema,
  method: verificationMethodSchema,
  policy: verificationPolicySchema,
  checksumAlgorithm: checksumAlgorithmSchema.nullable(),
  checkedFiles: z.number().int().nonnegative(),
  verifiedFiles: z.number().int().nonnegative(),
  mismatchedFiles: z.number().int().nonnegative(),
  failedFiles: z.number().int().nonnegative(),
  verifiedBytes: z.number().int().nonnegative(),
  /** One line the backend rendered, so the UI never reinterprets the fields. */
  verdict: z.string(),
});

export type HistoryVerification = z.infer<typeof historyVerificationSchema>;

/** One issue as history keeps it. */
export const historyIssueSchema = z.object({
  path: z.string().min(1),
  reason: transferIssueReasonSchema,
  error: transferErrorSchema.nullable(),
  detail: z.string().nullable(),
});

export type HistoryIssue = z.infer<typeof historyIssueSchema>;

/** One finished transfer, as history keeps it. */
export const historyRecordSchema = z.object({
  /** The job identifier the record belongs to. */
  id: z.string().min(1),
  operation: transferOperationSchema,
  conflict: conflictStrategySchema,
  status: historyStatusSchema,
  sources: z.array(z.string().min(1)).min(1),
  destination: z.string().min(1),

  totalBytes: z.number().int().nonnegative(),
  transferredBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
  completedFiles: z.number().int().nonnegative(),
  totalDirectories: z.number().int().nonnegative(),
  completedDirectories: z.number().int().nonnegative(),
  skippedItems: z.number().int().nonnegative(),
  failedItems: z.number().int().nonnegative(),

  queuedAtMs: z.number().int().nonnegative(),
  startedAtMs: z.number().int().nonnegative().nullable(),
  finishedAtMs: z.number().int().nonnegative(),
  /** Time the job spent working, paused time excluded. */
  durationMs: z.number().int().nonnegative(),

  /** Job-level failure, when there was one. */
  error: transferErrorSchema.nullable(),
  /** True when more issues happened than the record kept. */
  issuesTruncated: z.boolean(),
  /** Bounded sample of the issues the job reported, for the details view. */
  issues: z.array(historyIssueSchema),

  verification: historyVerificationSchema.nullable(),

  /** Set when this record came out of recovery rather than a live finish. */
  recovery: recoveryActionSchema.nullable(),
  /** The interrupted job this record was restarted from, when it was. */
  recoveredFrom: z.string().min(1).nullable(),
});

export type HistoryRecord = z.infer<typeof historyRecordSchema>;

/** A page of history plus the document's own health. */
export const historyListingSchema = z.object({
  records: z.array(historyRecordSchema),
  /** Records kept in total, whatever the filter selects. */
  total: z.number().int().nonnegative(),
  /** The retention bound currently in force. */
  limit: z.number().int().nonnegative(),
  filter: historyFilterSchema,
  /** How the history document loaded. */
  status: documentStatusSchema,
  /** False only while the document belongs to a newer build. */
  writable: z.boolean(),
});

export type HistoryListing = z.infer<typeof historyListingSchema>;

/** Whether a history status means the data arrived. */
export function historyStatusSucceeded(status: HistoryStatus): boolean {
  return status === "completed" || status === "recovered";
}

/** Short label for a history status. */
export function historyStatusLabel(status: HistoryStatus): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "interrupted":
      return "Interrupted";
    case "recovered":
      return "Recovered";
  }
}

/** Label for a history filter. */
export function historyFilterLabel(filter: HistoryFilter): string {
  switch (filter) {
    case "all":
      return "All";
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "interrupted":
      return "Interrupted";
  }
}
