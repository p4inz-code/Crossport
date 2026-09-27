/* ==========================================================================
 * Recovery types
 * Mirrors `src-tauri/src/recovery/` and the archive's recovery surface in
 * `src-tauri/src/archive.rs`: what an interrupted transfer looks like, what
 * recovery concluded about it, what an action would do, and what the durable
 * documents reported when they loaded.
 *
 * An outcome is a verdict, not a suggestion: `completed_before_crash` is only
 * ever reached from the archive's own record of the job finishing, never from
 * the presence of a destination file.
 * ========================================================================== */

import { z } from "zod";

import { documentStatusSchema } from "./archive";
import { historyVerificationSchema, recoveryActionSchema } from "./history";
import { conflictStrategySchema, transferOperationSchema } from "./transfer";
import { verificationPolicySchema } from "./verification";

/** What recovery decided about one interrupted job. */
export const RECOVERY_OUTCOMES = [
  "completed_before_crash",
  "restart_required",
  "source_missing",
  "destination_unavailable",
  "unsupported",
] as const;
export type RecoveryOutcome = (typeof RECOVERY_OUTCOMES)[number];

/** A job's counters as they were when its state was last written. */
export const persistedProgressSchema = z.object({
  totalBytes: z.number().int().nonnegative(),
  transferredBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
  completedFiles: z.number().int().nonnegative(),
  totalDirectories: z.number().int().nonnegative(),
  completedDirectories: z.number().int().nonnegative(),
  skippedItems: z.number().int().nonnegative(),
  failedItems: z.number().int().nonnegative(),
});

export type PersistedProgress = z.infer<typeof persistedProgressSchema>;

/** A temporary file one job left behind. */
export const partialArtifactSchema = z.object({
  path: z.string().min(1),
  /** Size on disk, which is how much of that file had been written. */
  bytes: z.number().int().nonnegative(),
});

export type PartialArtifact = z.infer<typeof partialArtifactSchema>;

/** What running an interrupted job again would do to the destination. */
export const restartImpactSchema = z.object({
  strategy: conflictStrategySchema,
  conflicts: z.number().int().nonnegative(),
  skippedItems: z.number().int().nonnegative(),
  totalBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
  /** True when a restart would overwrite existing entries. */
  overwrites: z.boolean(),
});

export type RestartImpact = z.infer<typeof restartImpactSchema>;

/** One interrupted job as the frontend sees it. */
export const recoveryCandidateSchema = z.object({
  id: z.string().min(1),
  operation: transferOperationSchema,
  conflict: conflictStrategySchema,
  verification: verificationPolicySchema,
  sources: z.array(z.string().min(1)).min(1),
  destination: z.string().min(1),
  /** What the job was doing when its state was last written. */
  status: z.string().min(1),
  queuedAtMs: z.number().int().nonnegative(),
  startedAtMs: z.number().int().nonnegative().nullable(),
  updatedAtMs: z.number().int().nonnegative(),
  progress: persistedProgressSchema,
  /** Whole percent reported before the interruption, when measurable. */
  percent: z.number().int().min(0).max(100).nullable(),
  verificationSummary: historyVerificationSchema.nullable(),
  outcome: z.enum(RECOVERY_OUTCOMES),
  /** Why the outcome is what it is, in the backend's own words. */
  detail: z.string().nullable(),
  /** Partial files this job left behind. */
  artifacts: z.array(partialArtifactSchema),
  artifactBytes: z.number().int().nonnegative(),
  /** True when the artifact scan hit its bound and may have missed some. */
  artifactsTruncated: z.boolean(),
  artifactDirectories: z.array(z.string()),
  /** What a restart would do, when it can be run again. */
  restartImpact: restartImpactSchema.nullable(),
  /**
   * Evidence, never proof: whether a re-plan found every planned item in place
   * with the expected size. `null` when that could not be determined.
   */
  destinationLooksComplete: z.boolean().nullable(),
  /** True when the archive's own record shows the job finished. */
  confirmedByArchive: z.boolean(),
  canRestart: z.boolean(),
  canDiscard: z.boolean(),
});

export type RecoveryCandidate = z.infer<typeof recoveryCandidateSchema>;

/** What a recovery decision did. */
export const recoveryReportSchema = z.object({
  candidate: recoveryCandidateSchema,
  action: recoveryActionSchema,
  /** Identifier of the job the restart produced, when it was restarted. */
  restartedAs: z.string().min(1).nullable(),
  artifactsRemoved: z.number().int().nonnegative(),
});

export type RecoveryReport = z.infer<typeof recoveryReportSchema>;

/** What recovery currently finds, plus the state document's health. */
export const recoveryListingSchema = z.object({
  candidates: z.array(recoveryCandidateSchema),
  status: documentStatusSchema,
  writable: z.boolean(),
});

export type RecoveryListing = z.infer<typeof recoveryListingSchema>;

/** What an outcome means, and what the user can do about it. */
export function recoveryOutcomeLabel(outcome: RecoveryOutcome): string {
  switch (outcome) {
    case "completed_before_crash":
      return "Finished before the application stopped";
    case "restart_required":
      return "Interrupted — can be run again";
    case "source_missing":
      return "Interrupted — a source is gone";
    case "destination_unavailable":
      return "Interrupted — destination unavailable";
    case "unsupported":
      return "Interrupted — cannot be replanned";
  }
}

/** Whether a candidate is an episode rather than work that can run again. */
export function isRecoverable(outcome: RecoveryOutcome): boolean {
  return outcome === "restart_required";
}

/** Whether a candidate needs the user to make a decision about it. */
export function needsDecision(outcome: RecoveryOutcome): boolean {
  return outcome !== "completed_before_crash";
}
