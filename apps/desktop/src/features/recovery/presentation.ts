/* ==========================================================================
 * Recovery presentation
 * Describes an interrupted transfer and what can be done about it.
 *
 * Every line here comes from the backend: the outcome, the reason, the
 * approximate progress, and whether a restart is possible. The UI never
 * upgrades an episode to "finished", and it always states that a restart
 * begins every file from the beginning.
 * ========================================================================== */

import { formatBytes } from "@/lib";
import {
  type RecoveryAction,
  type RecoveryCandidate,
  recoveryOutcomeLabel,
} from "@/types";

/** One line naming what the job was doing. */
export function candidateTitle(candidate: RecoveryCandidate): string {
  const operation = candidate.operation === "move" ? "Move" : "Copy";
  const source = candidate.sources[0];
  const extra =
    candidate.sources.length > 1
      ? ` and ${candidate.sources.length - 1} more`
      : "";
  return `${operation} ${source}${extra} → ${candidate.destination}`;
}

/** How far the job had reported getting, when that is known. */
export function candidateProgress(candidate: RecoveryCandidate): string {
  const { progress, percent } = candidate;
  if (progress.totalBytes > 0) {
    return `${formatBytes(progress.transferredBytes)} of ${formatBytes(progress.totalBytes)}${
      percent === null ? "" : ` (${percent}%)`
    }`;
  }
  const items = progress.totalFiles + progress.totalDirectories;
  if (items > 0) {
    const done = progress.completedFiles + progress.completedDirectories;
    return `${done} of ${items} items${percent === null ? "" : ` (${percent}%)`}`;
  }
  return "No progress was reported before the interruption";
}

/** The outcome, in the backend's words. */
export function candidateOutcome(candidate: RecoveryCandidate): string {
  return recoveryOutcomeLabel(candidate.outcome);
}

/** Why the outcome is what it is. */
export function candidateReason(candidate: RecoveryCandidate): string | null {
  return candidate.detail;
}

/** What the job left on disk, when it left anything. */
export function artifactSummary(candidate: RecoveryCandidate): string | null {
  if (candidate.artifacts.length === 0) {
    return null;
  }
  const directories = candidate.artifactDirectories.length;
  const truncation = candidate.artifactsTruncated
    ? " (the scan hit its bound, so there may be more)"
    : "";
  return `${candidate.artifacts.length} unfinished file(s), ${formatBytes(
    candidate.artifactBytes,
  )}, in ${directories} director${directories === 1 ? "y" : "ies"}${truncation}`;
}

/**
 * What restarting would do to the destination.
 *
 * An overwrite is called out explicitly: with the `replace` strategy a restart
 * overwrites the entries that are already there, and the user must know before
 * they choose it.
 */
export function restartImpactLine(candidate: RecoveryCandidate): string | null {
  const impact = candidate.restartImpact;
  if (impact === null) {
    return null;
  }
  if (impact.conflicts === 0) {
    return `Nothing exists at the destination yet; restarting writes ${formatBytes(impact.totalBytes)} fresh.`;
  }
  if (impact.overwrites) {
    return `Restarting overwrites ${impact.conflicts} existing entr${
      impact.conflicts === 1 ? "y" : "ies"
    } under the '${impact.strategy}' strategy.`;
  }
  return `${impact.conflicts} entr${impact.conflicts === 1 ? "y" : "ies"} already exist; the '${impact.strategy}' strategy leaves them alone.`;
}

/** Whether the archive itself proved the job had finished. */
export function candidateProof(candidate: RecoveryCandidate): string | null {
  if (candidate.confirmedByArchive) {
    return "The archive recorded this transfer reaching a terminal state, so nothing has to run again.";
  }
  if (candidate.destinationLooksComplete === true) {
    return "Every planned item is present at the expected size, but completion was never recorded — this is evidence, not proof, so the transfer is still treated as unfinished.";
  }
  return null;
}

/** Label for a recovery action. */
export function actionLabel(
  action: Exclude<RecoveryAction, "pending">,
): string {
  switch (action) {
    case "restart":
      return "Restart from the beginning";
    case "discard":
      return "Discard partial files";
    case "confirm":
      return "Mark as recovered";
  }
}

/** Short label for a history badge, used by the recovery list. */
export function candidateBadge(candidate: RecoveryCandidate): string {
  return candidate.canRestart ? "Can restart" : "Cannot restart";
}
