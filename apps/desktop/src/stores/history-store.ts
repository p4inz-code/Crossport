/* ==========================================================================
 * History store
 * The durable account of finished transfers, kept deliberately separate from
 * the transfer queue: a queue entry is runtime state that disappears when the
 * process does, while a history record is a fact about the past.
 *
 * Two properties this store keeps:
 *
 * - the listing carries the document's own status, so a degraded file shows as
 *   "unusable and set aside" rather than an empty history;
 * - a selection only ever points at a record that is actually loaded, so the
 *   details panel cannot show a record the list no longer has.
 * ========================================================================== */

import { create } from "zustand";
import {
  clearHistory,
  deleteHistoryRecord,
  getArchiveStatus,
  listHistory,
} from "@/services/history-service";
import { type IpcError, toIpcError } from "@/services/ipc";
import type {
  ArchiveStatus,
  DocumentStatus,
  HistoryFilter,
  HistoryRecord,
  LoadStatus,
} from "@/types";

interface HistoryState {
  records: HistoryRecord[];
  /** Filter the current listing was loaded with. */
  filter: HistoryFilter;
  /** Total records kept, whatever the filter selects. */
  total: number;
  /** Retention bound in force. */
  limit: number;
  /** How the history document loaded. */
  documentStatus: DocumentStatus | null;
  /** False only while the document belongs to a newer build. */
  writable: boolean;
  /** The archive's own health, including the interrupted-job count. */
  archive: ArchiveStatus | null;
  /** Loading state of the listing. */
  status: LoadStatus;
  error: IpcError | null;
  /** Identifier of the record shown in the details panel. */
  selectedId: string | null;
  /** Identifiers with an in-flight delete, so buttons can disable. */
  pending: string[];

  /** Loads the listing for a filter and the archive status. */
  load: (filter?: HistoryFilter) => Promise<void>;
  /** Selects a record for the details panel; `null` closes it. */
  select: (id: string | null) => void;
  /** Deletes one record and reloads the listing. */
  remove: (id: string) => Promise<void>;
  /** Clears every record. */
  clear: () => Promise<void>;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  records: [],
  filter: "all",
  total: 0,
  limit: 0,
  documentStatus: null,
  writable: true,
  archive: null,
  status: "idle",
  error: null,
  selectedId: null,
  pending: [],

  load: async (filter) => {
    const next = filter ?? get().filter;
    set({ status: "loading", error: null, filter: next });
    try {
      const [listing, archive] = await Promise.all([
        listHistory(next),
        getArchiveStatus(),
      ]);
      const selectedId = get().selectedId;
      set({
        records: listing.records,
        filter: listing.filter,
        total: listing.total,
        limit: listing.limit,
        documentStatus: listing.status,
        writable: listing.writable,
        archive,
        status: "ready",
        error: null,
        // A record that is no longer listed must not stay selected.
        selectedId: listing.records.some((record) => record.id === selectedId)
          ? selectedId
          : null,
      });
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn("[history] loading the listing failed", ipcError);
      set({
        records: [],
        status: "error",
        error: ipcError,
        selectedId: null,
      });
    }
  },

  select: (id) => set({ selectedId: id }),

  remove: async (id) => {
    set((state) => ({
      pending: [...state.pending.filter((entry) => entry !== id), id],
    }));
    try {
      await deleteHistoryRecord(id);
      await get().load();
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn(`[history] deleting ${id} failed`, ipcError);
      set({ error: ipcError });
    } finally {
      set((state) => ({
        pending: state.pending.filter((entry) => entry !== id),
      }));
    }
  },

  clear: async () => {
    set({ status: "loading", error: null });
    try {
      await clearHistory();
      set({ selectedId: null });
      await get().load();
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn("[history] clearing failed", ipcError);
      set({ status: "error", error: ipcError });
    }
  },
}));
