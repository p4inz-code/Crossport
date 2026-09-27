/* ==========================================================================
 * History service
 * The typed transport for the archive's read side: transfer history and the
 * health of the durable documents behind it.
 *
 * Every payload is validated before it reaches application state, so a record
 * that breaks the contract is rejected here instead of rendering as a broken
 * row. A document written by a newer build comes back with
 * `state: "unsupported"` and `writable: false`: real information, presented
 * instead of an empty list.
 * ========================================================================== */

import { z } from "zod";

import {
  type ArchiveStatus,
  archiveStatusSchema,
  type HistoryFilter,
  type HistoryListing,
  type HistoryRecord,
  historyListingSchema,
  historyRecordSchema,
} from "@/types";
import { invokeTyped, requireDesktopRuntime } from "./ipc";

/** How many records a clear removed, as the backend reports it. */
const removedCountSchema = z.number().int().nonnegative();

/** Whether the archive removed a record. */
const removedFlagSchema = z.boolean();

/**
 * Lists finished transfers, newest first.
 *
 * `filter` is one of `all`, `completed`, `failed`, `cancelled`, `interrupted`;
 * the backend rejects anything else with `invalid_input` rather than silently
 * showing the wrong records.
 */
export async function listHistory(
  filter: HistoryFilter = "all",
): Promise<HistoryListing> {
  requireDesktopRuntime("list_history");
  return invokeTyped("list_history", historyListingSchema, { filter });
}

/** One transfer's record, with everything history kept about it. */
export async function getHistoryRecord(id: string): Promise<HistoryRecord> {
  requireDesktopRuntime("get_history_record");
  return invokeTyped("get_history_record", historyRecordSchema, { id });
}

/** Removes one record. `false` means there was nothing to remove. */
export async function deleteHistoryRecord(id: string): Promise<boolean> {
  requireDesktopRuntime("delete_history_record");
  return invokeTyped("delete_history_record", removedFlagSchema, { id });
}

/** Removes every record and reports how many were removed. */
export async function clearHistory(): Promise<number> {
  requireDesktopRuntime("clear_history");
  return invokeTyped("clear_history", removedCountSchema);
}

/** The archive's own health: load states, counts, and the retention bound. */
export async function getArchiveStatus(): Promise<ArchiveStatus> {
  requireDesktopRuntime("get_archive_status");
  return invokeTyped("get_archive_status", archiveStatusSchema);
}
