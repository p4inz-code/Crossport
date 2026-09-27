/* ==========================================================================
 * Recovery store
 * What was in flight when the application stopped, and the decisions the user
 * makes about it.
 *
 * Nothing here decides on its own. Loading is a read; every action is an
 * explicit call the backend validates against the candidate's own outcome, and
 * a refusal is recorded next to the candidate it belongs to — which is where
 * the user is looking. A candidate is never marked completed by this store, and
 * a restart never claims the original job succeeded: the episode is recorded as
 * interrupted and the restarted job reports itself.
 * ========================================================================== */

import { create } from "zustand";

import { type IpcError, toIpcError } from "@/services/ipc";
import {
  listRecoveryCandidates,
  recoverTransfer,
} from "@/services/recovery-service";
import type {
  DocumentStatus,
  LoadStatus,
  RecoveryAction,
  RecoveryCandidate,
  RecoveryReport,
} from "@/types";

interface RecoveryState {
  candidates: RecoveryCandidate[];
  /** How the interrupted-state document loaded. */
  documentStatus: DocumentStatus | null;
  /** False only while the document belongs to a newer build. */
  writable: boolean;
  /** Loading state of the candidate list. */
  status: LoadStatus;
  error: IpcError | null;
  /** Identifiers with an in-flight action, so buttons can disable. */
  pending: string[];
  /** Action failures, keyed by candidate identifier. */
  failures: Record<string, IpcError>;
  /** Reports of actions that succeeded, newest first. */
  reports: RecoveryReport[];

  /** Reloads the candidate list. */
  load: () => Promise<void>;
  /** Carries out one decision; resolves with the report, or `null` on failure. */
  act: (
    id: string,
    action: Exclude<RecoveryAction, "pending">,
  ) => Promise<RecoveryReport | null>;
}

export const useRecoveryStore = create<RecoveryState>((set, get) => ({
  candidates: [],
  documentStatus: null,
  writable: true,
  status: "idle",
  error: null,
  pending: [],
  failures: {},
  reports: [],

  load: async () => {
    set({ status: "loading", error: null });
    try {
      const listing = await listRecoveryCandidates();
      // A candidate that is no longer there must not keep a stale failure.
      const known = new Set(listing.candidates.map((entry) => entry.id));
      const failures = Object.fromEntries(
        Object.entries(get().failures).filter(([id]) => known.has(id)),
      );
      set({
        candidates: listing.candidates,
        documentStatus: listing.status,
        writable: listing.writable,
        status: "ready",
        error: null,
        failures,
      });
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn("[recovery] loading the candidate list failed", ipcError);
      set({
        candidates: [],
        status: "error",
        error: ipcError,
        failures: {},
      });
    }
  },

  act: async (id, action) => {
    set((state) => ({
      pending: [...state.pending.filter((entry) => entry !== id), id],
      failures: withoutKey(state.failures, id),
    }));

    try {
      const report = await recoverTransfer(id, action);
      set((state) => ({ reports: [report, ...state.reports] }));
      await get().load();
      return report;
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn(`[recovery] '${action}' for ${id} failed`, ipcError);
      set((state) => ({ failures: { ...state.failures, [id]: ipcError } }));
      return null;
    } finally {
      set((state) => ({
        pending: state.pending.filter((entry) => entry !== id),
      }));
    }
  },
}));

/** Copy of a failure map without one key, keeping the rest untouched. */
function withoutKey(
  failures: Record<string, IpcError>,
  key: string,
): Record<string, IpcError> {
  const { [key]: _dropped, ...rest } = failures;
  return rest;
}
