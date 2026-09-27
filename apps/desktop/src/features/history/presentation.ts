/* ==========================================================================
 * History presentation
 * Turns a history record into the lines a reader needs: what happened, how much
 * moved, how long it took, and what the verification actually proved.
 *
 * The rule this module keeps: a failure is explained with the structured error
 * the backend recorded. A record never renders as "something went wrong" while
 * a code and a message exist.
 * ========================================================================== */

import { formatBytes, formatDuration } from "@/lib";
import {
  type HistoryIssue,
  type HistoryRecord,
  type HistoryVerification,
  historyStatusLabel,
  isVerificationFailure,
  type TransferOperation,
  verificationPolicyLabel,
} from "@/types";

/** Tone a record's status maps to, for colour and iconography. */
export type HistoryTone = "success" | "warning" | "danger" | "neutral";

/** Short label for an operation. */
export function operationLabel(operation: TransferOperation): string {
  return operation === "move" ? "Move" : "Copy";
}

/** How a record's status should read. */
export function historyTone(record: HistoryRecord): HistoryTone {
  switch (record.status) {
    case "completed":
    case "recovered":
      return "success";
    case "interrupted":
      return "warning";
    case "failed":
      return "danger";
    case "cancelled":
      return "neutral";
  }
}

/** One line for a row: what moved, out of what was planned. */
export function historyRowSummary(record: HistoryRecord): string {
  const items = `${record.completedFiles} of ${record.totalFiles} file${record.totalFiles === 1 ? "" : "s"}`;
  return `${items} · ${formatBytes(record.transferredBytes)} of ${formatBytes(record.totalBytes)}`;
}

/**
 * The verification line for a record.
 *
 * A record from before verification existed says so instead of implying a
 * check that never ran.
 */
export function verificationLine(
  verification: HistoryVerification | null,
): string {
  if (verification === null) {
    return "No verification was recorded";
  }
  return `${verificationPolicyLabel(verification.policy)} — ${verification.verdict}`;
}

/** Whether a record's verification did not prove the data arrived. */
export function verificationFailed(
  verification: HistoryVerification | null,
): boolean {
  return verification !== null && isVerificationFailure(verification.status);
}

/**
 * Why a transfer failed, in the backend's own words.
 *
 * Prefers the record's structured error; falls back to the first failed issue,
 * which carries the same shape, and only then to the status label.
 */
export function failureReason(record: HistoryRecord): {
  code: string;
  message: string;
} | null {
  if (record.error !== null) {
    return record.error;
  }
  const failed = record.issues.find((issue) => issue.reason === "failed");
  return failed?.error ?? null;
}

/** One line for an issue, whatever kind it is. */
export function issueLine(issue: HistoryIssue): string {
  if (issue.reason === "failed" && issue.error !== null) {
    return `${issue.path} — ${issue.error.message}`;
  }
  return `${issue.path} — ${issue.detail ?? issue.reason}`;
}

/** The conflict outcome a record carries, when there was one. */
export function conflictSummary(record: HistoryRecord): string | null {
  if (record.skippedItems === 0 && record.failedItems === 0) {
    return null;
  }
  const parts: string[] = [];
  if (record.skippedItems > 0) {
    parts.push(
      `${record.skippedItems} left alone by the '${record.conflict}' strategy`,
    );
  }
  if (record.failedItems > 0) {
    parts.push(`${record.failedItems} failed`);
  }
  return parts.join("; ");
}

/** Everything a details panel shows about one record, in one place. */
export interface HistoryDetailsModel {
  statusLabel: string;
  tone: HistoryTone;
  operation: string;
  sources: string[];
  destination: string;
  items: string;
  bytes: string;
  duration: string;
  queuedAt: number;
  startedAt: number | null;
  finishedAt: number;
  verification: string;
  verificationFailed: boolean;
  conflicts: string | null;
  failure: { code: string; message: string } | null;
  issues: string[];
  issuesTruncated: boolean;
  recovery: string | null;
}

/** Builds the details model for a record. */
export function historyDetails(record: HistoryRecord): HistoryDetailsModel {
  return {
    statusLabel: historyStatusLabel(record.status),
    tone: historyTone(record),
    operation: operationLabel(record.operation),
    sources: record.sources,
    destination: record.destination,
    items: `${record.completedFiles}/${record.totalFiles} files, ${record.completedDirectories}/${record.totalDirectories} directories`,
    bytes: `${formatBytes(record.transferredBytes)} of ${formatBytes(record.totalBytes)}`,
    duration: formatDuration(record.durationMs),
    queuedAt: record.queuedAtMs,
    startedAt: record.startedAtMs,
    finishedAt: record.finishedAtMs,
    verification: verificationLine(record.verification),
    verificationFailed: verificationFailed(record.verification),
    conflicts: conflictSummary(record),
    failure: failureReason(record),
    issues: record.issues.map(issueLine),
    issuesTruncated: record.issuesTruncated,
    recovery: recoveryLine(record),
  };
}

/** What recovery did about an interrupted episode, when it is one. */
function recoveryLine(record: HistoryRecord): string | null {
  switch (record.recovery) {
    case null:
      return null;
    case "restart":
      return "This episode was restarted from the beginning.";
    case "discard":
      return "This episode was discarded; its partial output was removed.";
    case "confirm":
      return "The archive proved this transfer had already finished.";
    case "pending":
      return "No recovery decision was recorded.";
  }
}
