import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/transfer-service";
import { makeTransferSnapshot } from "@/test/fixtures";
import { useTransferStore } from "./transfer-store";

vi.mock("@/services/transfer-service", () => ({
  cancelTransfer: vi.fn(),
  clearFinishedTransfers: vi.fn(),
  listTransfers: vi.fn(),
  pauseTransfer: vi.fn(),
  removeTransfer: vi.fn(),
  resumeTransfer: vi.fn(),
  subscribeToTransferUpdates: vi.fn(),
}));

const mockedList = vi.mocked(service.listTransfers);
const mockedPause = vi.mocked(service.pauseTransfer);
const mockedResume = vi.mocked(service.resumeTransfer);
const mockedCancel = vi.mocked(service.cancelTransfer);
const mockedRemove = vi.mocked(service.removeTransfer);
const mockedClear = vi.mocked(service.clearFinishedTransfers);
const mockedSubscribe = vi.mocked(service.subscribeToTransferUpdates);

const FIRST = makeTransferSnapshot({
  id: "transfer-1700000000000-1",
  queuedAtMs: 1_700_000_000_000,
});
const SECOND = makeTransferSnapshot({
  id: "transfer-1700000001000-2",
  queuedAtMs: 1_700_000_001_000,
  status: "queued",
  startedAtMs: null,
});

function resetStore(): void {
  useTransferStore.setState({
    jobs: [],
    status: "idle",
    error: null,
    pending: [],
    failures: {},
    dismissed: [],
  });
}

describe("transfer store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
  });

  it("starts idle with an empty queue", () => {
    const state = useTransferStore.getState();

    expect(state.jobs).toEqual([]);
    expect(state.status).toBe("idle");
    expect(state.error).toBeNull();
  });

  it("loads the queue in the order the engine reported", async () => {
    mockedList.mockResolvedValue([FIRST, SECOND]);

    await useTransferStore.getState().refresh();

    const state = useTransferStore.getState();
    expect(state.jobs.map((job) => job.id)).toEqual([FIRST.id, SECOND.id]);
    expect(state.status).toBe("ready");
    expect(state.error).toBeNull();
  });

  it("surfaces a failed queue listing and drops stale jobs", async () => {
    useTransferStore.setState({ jobs: [FIRST] });
    mockedList.mockRejectedValue(
      new IpcError("unavailable", "only available in the desktop app"),
    );

    await useTransferStore.getState().refresh();

    const state = useTransferStore.getState();
    expect(state.jobs).toEqual([]);
    expect(state.status).toBe("error");
    expect(state.error?.code).toBe("unavailable");
  });

  it("merges a snapshot by identifier instead of duplicating the job", () => {
    useTransferStore.getState().apply(FIRST);
    useTransferStore.getState().apply(FIRST);

    expect(useTransferStore.getState().jobs).toHaveLength(1);
  });

  it("adopts progress from an event", () => {
    useTransferStore.getState().apply(FIRST);

    const moved = makeTransferSnapshot({
      id: FIRST.id,
      queuedAtMs: FIRST.queuedAtMs,
      status: "completed",
      progress: { ...FIRST.progress, percent: 100, transferredBytes: 4096 },
    });
    useTransferStore.getState().apply(moved);

    const job = useTransferStore.getState().jobs[0];
    expect(job?.status).toBe("completed");
    expect(job?.progress.percent).toBe(100);
  });

  it("keeps the queue ordered when a snapshot arrives before a listing", () => {
    useTransferStore.getState().apply(SECOND);
    useTransferStore.getState().apply(FIRST);

    expect(useTransferStore.getState().jobs.map((job) => job.id)).toEqual([
      FIRST.id,
      SECOND.id,
    ]);
  });

  it("pauses, resumes, and cancels through the backend", async () => {
    mockedPause.mockResolvedValue({ ...FIRST, status: "paused" });
    mockedResume.mockResolvedValue({ ...FIRST, status: "running" });
    mockedCancel.mockResolvedValue({ ...FIRST, status: "cancelled" });

    await useTransferStore.getState().pause(FIRST.id);
    expect(useTransferStore.getState().jobs[0]?.status).toBe("paused");

    await useTransferStore.getState().resume(FIRST.id);
    expect(useTransferStore.getState().jobs[0]?.status).toBe("running");
    expect(mockedResume).toHaveBeenCalledWith(FIRST.id);

    await useTransferStore.getState().cancel(FIRST.id);
    expect(useTransferStore.getState().jobs[0]?.status).toBe("cancelled");
    expect(mockedCancel).toHaveBeenCalledWith(FIRST.id);
  });

  it("records a control failure next to the job and leaves it in place", async () => {
    useTransferStore.getState().apply(FIRST);
    mockedPause.mockRejectedValue(
      new IpcError("transfer_not_found", "transfer not found: transfer-1"),
    );

    await useTransferStore.getState().pause(FIRST.id);

    const state = useTransferStore.getState();
    expect(state.jobs).toHaveLength(1);
    expect(state.pending).toEqual([]);
    expect(state.failures[FIRST.id]?.code).toBe("transfer_not_found");
  });

  it("clears the previous failure when the same job is controlled again", async () => {
    useTransferStore.getState().apply(FIRST);
    mockedPause.mockRejectedValue(new IpcError("io", "the bus went away"));
    await useTransferStore.getState().pause(FIRST.id);
    expect(useTransferStore.getState().failures[FIRST.id]).toBeDefined();

    mockedPause.mockResolvedValue({ ...FIRST, status: "paused" });
    await useTransferStore.getState().pause(FIRST.id);

    expect(useTransferStore.getState().failures[FIRST.id]).toBeUndefined();
  });

  it("removes a job and ignores a late event for it", async () => {
    useTransferStore.getState().apply(FIRST);
    mockedRemove.mockResolvedValue(undefined);

    await useTransferStore.getState().remove(FIRST.id);

    expect(useTransferStore.getState().jobs).toEqual([]);

    useTransferStore.getState().apply({ ...FIRST, status: "completed" });

    expect(useTransferStore.getState().jobs).toEqual([]);
  });

  it("reports a failed removal without dropping the job", async () => {
    useTransferStore.getState().apply(FIRST);
    mockedRemove.mockRejectedValue(new IpcError("io", "the queue is busy"));

    await useTransferStore.getState().remove(FIRST.id);

    const state = useTransferStore.getState();
    expect(state.jobs).toHaveLength(1);
    expect(state.failures[FIRST.id]?.message).toBe("the queue is busy");
  });

  it("drops every finished job and keeps the live ones", async () => {
    useTransferStore.setState({
      jobs: [
        { ...FIRST, status: "completed" },
        { ...SECOND, status: "running" },
      ],
    });
    mockedClear.mockResolvedValue(1);

    await expect(useTransferStore.getState().clearFinished()).resolves.toBe(1);

    expect(useTransferStore.getState().jobs.map((job) => job.id)).toEqual([
      SECOND.id,
    ]);
  });

  it("explains a failed clear instead of pretending the queue changed", async () => {
    useTransferStore.setState({ jobs: [{ ...FIRST, status: "failed" }] });
    mockedClear.mockRejectedValue(new IpcError("io", "the queue is busy"));

    await expect(useTransferStore.getState().clearFinished()).resolves.toBe(0);

    const state = useTransferStore.getState();
    expect(state.jobs).toHaveLength(1);
    expect(state.error?.message).toBe("the queue is busy");
  });

  it("forgets dismissals when the backend list is reloaded", async () => {
    useTransferStore.getState().apply(FIRST);
    mockedRemove.mockResolvedValue(undefined);
    await useTransferStore.getState().remove(FIRST.id);

    mockedList.mockResolvedValue([FIRST]);
    await useTransferStore.getState().refresh();

    expect(useTransferStore.getState().dismissed).toEqual([]);
    expect(useTransferStore.getState().jobs).toHaveLength(1);
  });

  it("follows the event feed once connected", async () => {
    let forward: ((snapshot: typeof FIRST) => void) | null = null;
    mockedSubscribe.mockImplementation((handler) => {
      forward = handler as (snapshot: typeof FIRST) => void;
      return Promise.resolve(vi.fn());
    });

    const stop = await useTransferStore.getState().connect();
    expect(mockedSubscribe).toHaveBeenCalledTimes(1);

    // The handler is assigned inside a callback, so it is re-read here.
    const listener = forward as unknown as
      | ((snapshot: typeof FIRST) => void)
      | null;
    listener?.(SECOND);

    expect(useTransferStore.getState().jobs.map((job) => job.id)).toEqual([
      SECOND.id,
    ]);
    stop();
  });
});
