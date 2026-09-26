import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { makeTransferPreview, makeTransferSnapshot } from "@/test/fixtures";
import type { TransferRequest } from "@/types";
import { IpcError } from "./ipc";
import {
  cancelTransfer,
  clearFinishedTransfers,
  getTransfer,
  listTransfers,
  pauseTransfer,
  planTransfer,
  removeTransfer,
  resumeTransfer,
  startTransfer,
  subscribeToTransferUpdates,
  TRANSFER_EVENT,
} from "./transfer-service";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);
const mockedIsTauri = vi.mocked(isTauri);

/**
 * Invokes a listener captured by the mocked `listen`. The listener is assigned
 * inside a callback, so it is re-read here instead of being narrowed to its
 * declaration-time value.
 */
function deliver(
  handler: ((event: { payload: unknown }) => void) | null,
  event: { payload: unknown },
): void {
  const listener = handler as unknown as
    | ((event: { payload: unknown }) => void)
    | null;
  listener?.(event);
}

const REQUEST: TransferRequest = {
  sources: ["D:\\Photos"],
  destination: "D:\\Backup",
  operation: "copy",
  conflict: "skip",
};

describe("transfer commands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("plans a request with the backend dry run", async () => {
    const preview = makeTransferPreview();
    mockedInvoke.mockResolvedValue(preview);

    await expect(planTransfer(REQUEST)).resolves.toEqual(preview);
    expect(mockedInvoke).toHaveBeenCalledWith("plan_transfer", {
      request: REQUEST,
    });
  });

  it("starts a request and returns the job it queued", async () => {
    const snapshot = makeTransferSnapshot({ status: "queued" });
    mockedInvoke.mockResolvedValue(snapshot);

    await expect(startTransfer(REQUEST)).resolves.toEqual(snapshot);
    expect(mockedInvoke).toHaveBeenCalledWith("start_transfer", {
      request: REQUEST,
    });
  });

  it("lists the queue", async () => {
    mockedInvoke.mockResolvedValue([makeTransferSnapshot()]);

    await expect(listTransfers()).resolves.toHaveLength(1);
    expect(mockedInvoke).toHaveBeenCalledWith("list_transfers", undefined);
  });

  it("reads, pauses, resumes, and cancels one job by identifier", async () => {
    const snapshot = makeTransferSnapshot();
    mockedInvoke.mockResolvedValue(snapshot);

    await getTransfer(snapshot.id);
    expect(mockedInvoke).toHaveBeenLastCalledWith("get_transfer", {
      id: snapshot.id,
    });

    await pauseTransfer(snapshot.id);
    expect(mockedInvoke).toHaveBeenLastCalledWith("pause_transfer", {
      id: snapshot.id,
    });

    await resumeTransfer(snapshot.id);
    expect(mockedInvoke).toHaveBeenLastCalledWith("resume_transfer", {
      id: snapshot.id,
    });

    await cancelTransfer(snapshot.id);
    expect(mockedInvoke).toHaveBeenLastCalledWith("cancel_transfer", {
      id: snapshot.id,
    });
  });

  it("removes a finished job and clears the finished ones", async () => {
    mockedInvoke.mockResolvedValue(null);
    await removeTransfer("transfer-1");
    expect(mockedInvoke).toHaveBeenLastCalledWith("remove_transfer", {
      id: "transfer-1",
    });

    mockedInvoke.mockResolvedValue(3);
    await expect(clearFinishedTransfers()).resolves.toBe(3);
    expect(mockedInvoke).toHaveBeenLastCalledWith(
      "clear_finished_transfers",
      undefined,
    );
  });

  it("surfaces a structured backend failure as an IpcError", async () => {
    mockedInvoke.mockRejectedValue({
      code: "unsafe_relationship",
      message: "the destination is inside the source",
    });

    const error = await startTransfer(REQUEST).catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("unsafe_relationship");
  });

  it("rejects a snapshot that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue({ id: "transfer-1", status: "running" });

    const error = await getTransfer("transfer-1").catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("refuses to call the backend outside the desktop app", async () => {
    mockedIsTauri.mockReturnValue(false);

    const error = await listTransfers().catch((failure: unknown) => failure);

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("unavailable");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});

describe("subscribeToTransferUpdates", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("listens on the published event and forwards validated snapshots", async () => {
    const stop = vi.fn();
    let handler: ((event: { payload: unknown }) => void) | null = null;
    mockedListen.mockImplementation((_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return Promise.resolve(stop);
    });

    const onUpdate = vi.fn();
    const unsubscribe = await subscribeToTransferUpdates(onUpdate);

    expect(mockedListen).toHaveBeenCalledWith(
      TRANSFER_EVENT,
      expect.any(Function),
    );
    expect(TRANSFER_EVENT).toBe("transfer:update");

    const snapshot = makeTransferSnapshot();
    deliver(handler, { payload: snapshot });
    expect(onUpdate).toHaveBeenCalledWith(snapshot);

    unsubscribe();
    expect(stop).toHaveBeenCalledTimes(1);
  });

  it("skips a payload that is not a snapshot instead of applying it", async () => {
    let handler: ((event: { payload: unknown }) => void) | null = null;
    mockedListen.mockImplementation((_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return Promise.resolve(vi.fn());
    });

    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const onUpdate = vi.fn();
    await subscribeToTransferUpdates(onUpdate);

    deliver(handler, { payload: { id: "transfer-1" } });

    expect(onUpdate).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("resolves with a no-op when there is no desktop backend", async () => {
    mockedIsTauri.mockReturnValue(false);

    const unsubscribe = await subscribeToTransferUpdates(vi.fn());

    expect(mockedListen).not.toHaveBeenCalled();
    expect(unsubscribe()).toBeUndefined();
  });
});
