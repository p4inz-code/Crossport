/* ==========================================================================
 * Notification presentation
 * Turns a backend verdict into one line of feedback. Every message comes from
 * what the backend reported — a status, a verdict, a structured error — and no
 * message claims more than that.
 *
 * A verification failure is never presented as a plain transfer failure: the
 * user needs to know that the bytes did not prove out, which is a different
 * problem from a transfer that could not run at all.
 * ========================================================================== */

import type { NotificationKind } from "@/stores/notification-store";
import {
  isTerminalTransferStatus,
  isVerificationFailure,
  type RecoveryCandidate,
  type TransferSnapshot,
  verificationStatusLabel,
} from "@/types";

/** The notification a finished transfer deserves, or `null` while it runs. */
export function notificationForTransfer(snapshot: TransferSnapshot): {
  kind: NotificationKind;
  title: string;
  message: string;
  event: string;
  /** Where the user can see what the notification is about. */
  to: string | null;
} | null {
  if (!isTerminalTransferStatus(snapshot.status)) {
    return null;
  }

  const items = formatCount(snapshot);
  const verification = snapshot.verification;

  if (snapshot.status === "cancelled") {
    return {
      kind: "info",
      title: "Transfer cancelled",
      message: `${items} — the destination was left as it was.`,
      event: "cancelled",
      to: "/history",
    };
  }

  if (snapshot.status === "failed") {
    if (isVerificationFailure(verification.status)) {
      return {
        kind: "error",
        title: verificationStatusLabel(verification.status),
        message: `${items} — ${verification.verdict}.`,
        event: "verification-failed",
        to: "/transfers",
      };
    }
    return {
      kind: "error",
      title: "Transfer failed",
      message: `${items} — ${snapshot.error?.message ?? verification.verdict}.`,
      event: "failed",
      to: "/transfers",
    };
  }

  // Completed. Skipped items are not a failure, but they are not nothing
  // either, and the user should hear about them exactly once.
  if (snapshot.progress.skippedItems > 0) {
    return {
      kind: "warning",
      title: "Transfer completed with skipped items",
      message: `${items}, ${snapshot.progress.skippedItems} item(s) left alone by the conflict strategy.`,
      event: "completed-with-skips",
      to: "/history",
    };
  }

  return {
    kind: "success",
    title: "Transfer completed",
    message: `${items} — ${verification.verdict}.`,
    event: "completed",
    to: "/history",
  };
}

/**
 * The notification recovery deserves, or `null` when nothing is interrupted.
 *
 * This is the startup prompt: it says what happened and that a decision is
 * waiting, without running anything.
 */
export function notificationForRecovery(candidates: RecoveryCandidate[]): {
  kind: NotificationKind;
  title: string;
  message: string;
  event: string;
  to: string | null;
} | null {
  if (candidates.length === 0) {
    return null;
  }

  const restartable = candidates.filter(
    (candidate) => candidate.canRestart,
  ).length;
  const detail =
    restartable > 0
      ? `${restartable} can be run again; the rest need a decision.`
      : "They cannot be run again, but they can still be inspected or discarded.";

  return {
    kind: "warning",
    title:
      candidates.length === 1
        ? "A transfer was interrupted"
        : `${candidates.length} transfers were interrupted`,
    message: `The application stopped while they were running. ${detail}`,
    event: "recovery-required",
    to: "/recovery",
  };
}

/** "3 files and 1 directory", for a notification line. */
function formatCount(snapshot: TransferSnapshot): string {
  const files = snapshot.progress.totalFiles;
  const directories = snapshot.progress.totalDirectories;
  const parts: string[] = [];
  if (files > 0) {
    parts.push(`${files} file${files === 1 ? "" : "s"}`);
  }
  if (directories > 0) {
    parts.push(`${directories} director${directories === 1 ? "y" : "ies"}`);
  }
  if (parts.length === 0) {
    return "No items";
  }
  return parts.join(" and ");
}
