import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/recovery-service";
import { makeRecoveryCandidate } from "@/test/fixtures";
import { useRecoveryStore } from "./recovery-store";

vi.mock("@/services/recovery-service", () => ({
  listRecoveryCandidates: vi.fn(),
  getRecoveryCandidate: vi.fn(),
  recoverTransfer: vi.fn(),
}));

const mockedList = vi.mocked(service.listRecoveryCandidates);
const mockedRecover = vi.mocked(service.recoverTransfer);

function listing(candidates = [makeRecoveryCandidate()]) {
  return {
    candidates,
    status: { state: "loaded" as const, detail: null },
    writable: true,
  };
}

describe("recovery store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useRecoveryStore.setState({
      candidates: [],
      documentStatus: null,
      writable: true,
      status: "idle",
      error: null,
      pending: [],
      failures: {},
      reports: [],
    });
  });

  it("loads what was interrupted", async () => {
    mockedList.mockResolvedValue(listing());

    await useRecoveryStore.getState().load();

    expect(useRecoveryStore.getState().candidates).toHaveLength(1);
    expect(useRecoveryStore.getState().status).toBe("ready");
    expect(useRecoveryStore.getState().documentStatus?.state).toBe("loaded");
  });

  it("reports a degraded state document instead of showing an empty list", async () => {
    mockedList.mockResolvedValue({
      ...listing([]),
      status: { state: "recovered", detail: "the file was empty" },
    });

    await useRecoveryStore.getState().load();

    expect(useRecoveryStore.getState().documentStatus).toEqual({
      state: "recovered",
      detail: "the file was empty",
    });
    expect(useRecoveryStore.getState().status).toBe("ready");
  });

  it("carries out a restart and reloads the list", async () => {
    const candidate = makeRecoveryCandidate();
    useRecoveryStore.setState({ candidates: [candidate] });
    mockedRecover.mockResolvedValue({
      candidate,
      action: "restart",
      restartedAs: "transfer-1700000000000-9",
      artifactsRemoved: 1,
    });
    mockedList.mockResolvedValue(listing([]));

    const report = await useRecoveryStore
      .getState()
      .act(candidate.id, "restart");

    expect(mockedRecover).toHaveBeenCalledWith(candidate.id, "restart");
    expect(report?.restartedAs).toBe("transfer-1700000000000-9");
    expect(useRecoveryStore.getState().reports).toHaveLength(1);
    expect(useRecoveryStore.getState().candidates).toEqual([]);
  });

  it("records a refusal next to the candidate and changes nothing", async () => {
    const candidate = makeRecoveryCandidate({
      outcome: "source_missing",
      canRestart: false,
    });
    useRecoveryStore.setState({ candidates: [candidate] });
    mockedRecover.mockRejectedValue(
      new IpcError(
        "recovery_unavailable",
        "transfer-1 cannot be restarted: the source is gone",
      ),
    );

    const report = await useRecoveryStore
      .getState()
      .act(candidate.id, "restart");

    expect(report).toBeNull();
    expect(
      useRecoveryStore.getState().failures[candidate.id]?.message,
    ).toContain("source is gone");
    expect(useRecoveryStore.getState().candidates).toEqual([candidate]);
    expect(useRecoveryStore.getState().reports).toEqual([]);
  });

  it("clears a stale failure for a candidate that is gone", async () => {
    const candidate = makeRecoveryCandidate();
    useRecoveryStore.setState({
      candidates: [candidate],
      failures: { [candidate.id]: new IpcError("io", "old failure") },
    });
    mockedList.mockResolvedValue(listing([]));

    await useRecoveryStore.getState().load();

    expect(useRecoveryStore.getState().failures).toEqual({});
  });

  it("surfaces a listing failure as a structured error", async () => {
    mockedList.mockRejectedValue(new IpcError("unavailable", "desktop only"));

    await useRecoveryStore.getState().load();

    expect(useRecoveryStore.getState().status).toBe("error");
    expect(useRecoveryStore.getState().error?.code).toBe("unavailable");
    expect(useRecoveryStore.getState().candidates).toEqual([]);
  });
});
