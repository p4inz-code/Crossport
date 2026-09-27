/* ==========================================================================
 * Recovery service
 * The typed transport for interrupted transfers and the decisions about them.
 *
 * Listing is a pure read; it inspects the destination tree for leftovers and
 * re-plans the recorded request, but changes nothing. An action is one explicit
 * call, and the backend validates it against the candidate's own outcome first:
 * restarting a job whose source disappeared fails with `recovery_unavailable`
 * rather than half-running.
 * ========================================================================== */

import {
  type RecoveryAction,
  type RecoveryCandidate,
  type RecoveryListing,
  type RecoveryReport,
  recoveryActionSchema,
  recoveryCandidateSchema,
  recoveryListingSchema,
  recoveryReportSchema,
} from "@/types";
import { IpcError, invokeTyped, requireDesktopRuntime } from "./ipc";

/** Every job that was in flight when the application stopped, with its fate. */
export async function listRecoveryCandidates(): Promise<RecoveryListing> {
  requireDesktopRuntime("list_recovery_candidates");
  return invokeTyped("list_recovery_candidates", recoveryListingSchema);
}

/** One interrupted job, or a structured error naming what is missing. */
export async function getRecoveryCandidate(
  id: string,
): Promise<RecoveryCandidate> {
  requireDesktopRuntime("get_recovery_candidate");
  return invokeTyped("get_recovery_candidate", recoveryCandidateSchema, { id });
}

/**
 * Carries out a recovery decision.
 *
 * The action is validated locally first, so a typo never reaches the backend,
 * and the response is validated like every other payload. `pending` is not a
 * decision and is rejected here as well as there.
 */
export async function recoverTransfer(
  id: string,
  action: RecoveryAction,
): Promise<RecoveryReport> {
  requireDesktopRuntime("recover_transfer");
  const validated = recoveryActionSchema.safeParse(action);
  if (!validated.success || validated.data === "pending") {
    throw new IpcError("invalid_input", `'${action}' is not a recovery action`);
  }
  return invokeTyped("recover_transfer", recoveryReportSchema, {
    id,
    action: validated.data,
  });
}
