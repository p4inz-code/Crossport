/* ==========================================================================
 * Transfer store
 * The queue as the application sees it: every job the engine knows about, in
 * queue order, kept live by `transfer:update` events and resynchronized with
 * `list_transfers`.
 *
 * Two properties this store deliberately keeps:
 *
 * - a snapshot received twice is applied once, because jobs are merged by
 *   identifier instead of appended;
 * - a job the user dismissed stays dismissed, so a late event cannot resurrect
 *   a row that is no longer on screen.
 *
 * Control calls never throw at the caller. A failure is recorded next to the
 * job it belongs to and rendered there, which is where the user is looking.
 * ========================================================================== */

import { create } from "zustand";
import { type IpcError, toIpcError } from "@/services/ipc";
import {
  cancelTransfer,
  clearFinishedTransfers,
  listTransfers,
  pauseTransfer,
  removeTransfer,
  resumeTransfer,
  subscribeToTransferUpdates,
} from "@/services/transfer-service";
import {
  isTerminalTransferStatus,
  type LoadStatus,
  type TransferSnapshot,
} from "@/types";

interface TransferState {
  /** Every job, oldest first. */
  jobs: TransferSnapshot[];
  /** Status of the queue listing itself, not of any job. */
  status: LoadStatus;
  /** Why the queue listing failed. */
  error: IpcError | null;
  /** Identifiers with an in-flight control call, so buttons can disable. */
  pending: string[];
  /** Control-call failures, keyed by job identifier. */
  failures: Record<string, IpcError>;
  /** Identifiers the user dismissed; late events for them are ignored. */
  dismissed: string[];

  /** Reloads the queue from the backend. */
  refresh: () => Promise<void>;
  /** Starts listening for progress. Resolves with the unsubscribe function. */
  connect: () => Promise<() => void>;
  /** Merges one snapshot, from a command reply or an event. */
  apply: (snapshot: TransferSnapshot) => void;
  pause: (id: string) => Promise<void>;
  resume: (id: string) => Promise<void>;
  /** Cancels a job and discards its partial output. */
  cancel: (id: string) => Promise<void>;
  /** Drops one finished job. */
  remove: (id: string) => Promise<void>;
  /** Drops every finished job; resolves with how many were removed. */
  clearFinished: () => Promise<number>;
}

/** Merges one snapshot into the queue, keeping it ordered. */
function mergeSnapshot(
  jobs: TransferSnapshot[],
  snapshot: TransferSnapshot,
): TransferSnapshot[] {
  const index = jobs.findIndex((job) => job.id === snapshot.id);
  const merged =
    index === -1
      ? [...jobs, snapshot]
      : jobs.map((job) => (job.id === snapshot.id ? snapshot : job));

  // The engine queues jobs in order, so appends are already ordered; sorting
  // by queue time keeps that true if a reply and an event ever race. The sort
  // is stable, so jobs queued in the same millisecond keep their order.
  return merged.sort((left, right) => left.queuedAtMs - right.queuedAtMs);
}

export const useTransferStore = create<TransferState>((set, get) => {
  /** Runs a control call for one job, recording its state and failure. */
  async function control(
    id: string,
    call: () => Promise<TransferSnapshot>,
  ): Promise<void> {
    set((state) => ({
      pending: [...state.pending.filter((entry) => entry !== id), id],
      failures: withoutKey(state.failures, id),
    }));

    try {
      const snapshot = await call();
      get().apply(snapshot);
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn(`[transfer] control call for ${id} failed`, ipcError);
      set((state) => ({ failures: { ...state.failures, [id]: ipcError } }));
    } finally {
      set((state) => ({
        pending: state.pending.filter((entry) => entry !== id),
      }));
    }
  }

  return {
    jobs: [],
    status: "idle",
    error: null,
    pending: [],
    failures: {},
    dismissed: [],

    refresh: async () => {
      set({ status: "loading", error: null });
      try {
        const jobs = await listTransfers();
        // The backend list is authoritative, so anything dismissed before is
        // no longer reported and the dismissal memory can be dropped.
        set({ jobs, status: "ready", error: null, dismissed: [] });
      } catch (error) {
        const ipcError = toIpcError(error);
        console.warn("[transfer] listing the queue failed", ipcError);
        set({ jobs: [], status: "error", error: ipcError });
      }
    },

    connect: async () =>
      subscribeToTransferUpdates((snapshot) => get().apply(snapshot)),

    apply: (snapshot: TransferSnapshot) => {
      const { jobs, dismissed } = get();
      if (dismissed.includes(snapshot.id)) {
        return;
      }
      set({ jobs: mergeSnapshot(jobs, snapshot) });
    },

    pause: (id: string) => control(id, () => pauseTransfer(id)),

    resume: (id: string) => control(id, () => resumeTransfer(id)),

    cancel: (id: string) => control(id, () => cancelTransfer(id)),

    remove: async (id: string) => {
      set((state) => ({
        pending: [...state.pending.filter((entry) => entry !== id), id],
        failures: withoutKey(state.failures, id),
      }));

      try {
        await removeTransfer(id);
        set((state) => ({
          jobs: state.jobs.filter((job) => job.id !== id),
          dismissed: [...state.dismissed.filter((entry) => entry !== id), id],
        }));
      } catch (error) {
        const ipcError = toIpcError(error);
        console.warn(`[transfer] removing ${id} failed`, ipcError);
        set((state) => ({ failures: { ...state.failures, [id]: ipcError } }));
      } finally {
        set((state) => ({
          pending: state.pending.filter((entry) => entry !== id),
        }));
      }
    },

    clearFinished: async () => {
      try {
        const removed = await clearFinishedTransfers();
        set((state) => ({
          jobs: state.jobs.filter(
            (job) => !isTerminalTransferStatus(job.status),
          ),
          dismissed: [
            ...state.dismissed,
            ...state.jobs
              .filter((job) => isTerminalTransferStatus(job.status))
              .map((job) => job.id),
          ],
        }));
        return removed;
      } catch (error) {
        const ipcError = toIpcError(error);
        console.warn("[transfer] clearing finished jobs failed", ipcError);
        set({ error: ipcError });
        return 0;
      }
    },
  };
});

/** Copy of a failure map without one key, keeping the rest untouched. */
function withoutKey(
  failures: Record<string, IpcError>,
  key: string,
): Record<string, IpcError> {
  const { [key]: _dropped, ...rest } = failures;
  return rest;
}
