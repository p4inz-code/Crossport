/* ==========================================================================
 * Transfer service
 * The typed transport for the transfer engine. Every call goes through
 * `invokeTyped`, so a payload that breaks the contract is rejected here
 * instead of reaching the queue with missing or wrong-typed fields.
 *
 * Progress arrives two ways, and both are supported on purpose:
 *
 * - `transfer:update` events, carrying a `TransferSnapshot` per job, so the
 *   queue stays live without polling;
 * - the commands themselves, which return a snapshot, so a client that missed
 *   an event (or just opened the app) resynchronizes with one call.
 * ========================================================================== */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";

import {
  type TransferPreview,
  type TransferRequest,
  type TransferSnapshot,
  transferPreviewSchema,
  transferSnapshotSchema,
} from "@/types";
import {
  invokeCommand,
  invokeTyped,
  isDesktopRuntime,
  requireDesktopRuntime,
} from "./ipc";

/** Event the backend emits for every job-state change. */
export const TRANSFER_EVENT = "transfer:update";

/** Payload of `clear_finished_transfers`: how many jobs were dropped. */
const removedCountSchema = z.number().int().nonnegative();

/** Every job in queue order, oldest first. */
const transferListSchema = z.array(transferSnapshotSchema);

/**
 * Plans a transfer without starting one.
 *
 * Read-only. Rejects with an `IpcError` for the same reasons `startTransfer`
 * would — `invalid_input`, `path_not_found`, `unsafe_relationship`,
 * `not_enough_space`, `too_many_items`, `permission_denied` — so the UI can
 * warn before the user commits.
 */
export async function planTransfer(
  request: TransferRequest,
): Promise<TransferPreview> {
  requireDesktopRuntime("plan_transfer");
  return invokeTyped("plan_transfer", transferPreviewSchema, { request });
}

/** Plans and queues a request, returning the job it created. */
export async function startTransfer(
  request: TransferRequest,
): Promise<TransferSnapshot> {
  requireDesktopRuntime("start_transfer");
  return invokeTyped("start_transfer", transferSnapshotSchema, { request });
}

/** Every job the engine knows about, including finished ones. */
export async function listTransfers(): Promise<TransferSnapshot[]> {
  requireDesktopRuntime("list_transfers");
  return invokeTyped("list_transfers", transferListSchema);
}

/** One job by identifier. Rejects with `transfer_not_found` when unknown. */
export async function getTransfer(id: string): Promise<TransferSnapshot> {
  requireDesktopRuntime("get_transfer");
  return invokeTyped("get_transfer", transferSnapshotSchema, { id });
}

/** Parks a running job. Progress stops and the destination stays untouched. */
export async function pauseTransfer(id: string): Promise<TransferSnapshot> {
  requireDesktopRuntime("pause_transfer");
  return invokeTyped("pause_transfer", transferSnapshotSchema, { id });
}

/** Continues a paused job from where it stopped. */
export async function resumeTransfer(id: string): Promise<TransferSnapshot> {
  requireDesktopRuntime("resume_transfer");
  return invokeTyped("resume_transfer", transferSnapshotSchema, { id });
}

/** Stops a job and discards whatever partial output it had written. */
export async function cancelTransfer(id: string): Promise<TransferSnapshot> {
  requireDesktopRuntime("cancel_transfer");
  return invokeTyped("cancel_transfer", transferSnapshotSchema, { id });
}

/** Drops one finished job from the queue. */
export async function removeTransfer(id: string): Promise<void> {
  requireDesktopRuntime("remove_transfer");
  await invokeCommand<unknown>("remove_transfer", { id });
}

/** Drops every finished job and reports how many were removed. */
export async function clearFinishedTransfers(): Promise<number> {
  requireDesktopRuntime("clear_finished_transfers");
  return invokeTyped("clear_finished_transfers", removedCountSchema);
}

/**
 * Subscribes to backend progress.
 *
 * Resolves with the function that stops listening. Payloads that do not match
 * the published snapshot contract are skipped rather than applied, so a
 * backend that changes shape shows up as a quiet gap instead of corrupt state.
 *
 * Outside the desktop app there is no backend to listen to, so this resolves
 * with a no-op and the caller needs no special case.
 */
export async function subscribeToTransferUpdates(
  onUpdate: (snapshot: TransferSnapshot) => void,
): Promise<UnlistenFn> {
  if (!isDesktopRuntime()) {
    return () => undefined;
  }

  return listen<unknown>(TRANSFER_EVENT, (event) => {
    const parsed = transferSnapshotSchema.safeParse(event.payload);
    if (!parsed.success) {
      console.warn(
        `[transfer] ignored a '${TRANSFER_EVENT}' payload that broke the contract`,
      );
      return;
    }
    onUpdate(parsed.data);
  });
}
