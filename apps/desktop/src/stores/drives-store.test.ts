import { beforeEach, describe, expect, it, vi } from "vitest";
import * as service from "@/services/drives-service";
import { IpcError } from "@/services/ipc";
import { useDrivesStore } from "./drives-store";

vi.mock("@/services/drives-service", () => ({
  listDrives: vi.fn(),
}));

const mockedListDrives = vi.mocked(service.listDrives);

describe("drives store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrivesStore.setState({ drives: [], status: "idle", error: null });
  });

  it("starts idle with no drives", () => {
    expect(useDrivesStore.getState().status).toBe("idle");
    expect(useDrivesStore.getState().drives).toEqual([]);
  });

  it("stores the drives reported by the backend", async () => {
    mockedListDrives.mockResolvedValue([
      { root: "C:\\", label: "C:" },
      { root: "D:\\", label: "D:" },
    ]);

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([
      { root: "C:\\", label: "C:" },
      { root: "D:\\", label: "D:" },
    ]);
    expect(useDrivesStore.getState().status).toBe("ready");
    expect(useDrivesStore.getState().error).toBeNull();
  });

  it("reports an empty result as a ready state", async () => {
    mockedListDrives.mockResolvedValue([]);

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([]);
    expect(useDrivesStore.getState().status).toBe("ready");
  });

  it("surfaces a structured error and clears stale drives", async () => {
    useDrivesStore.setState({
      drives: [{ root: "C:\\", label: "C:" }],
      status: "ready",
    });
    mockedListDrives.mockRejectedValue(
      new IpcError("permission_denied", "permission denied: volume is locked"),
    );

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([]);
    expect(useDrivesStore.getState().status).toBe("error");
    expect(useDrivesStore.getState().error?.code).toBe("permission_denied");
  });

  it("normalises non-IpcError failures", async () => {
    mockedListDrives.mockRejectedValue("boom");

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().error).toBeInstanceOf(IpcError);
    expect(useDrivesStore.getState().error?.message).toBe("boom");
  });

  it("recovers once enumeration succeeds again", async () => {
    mockedListDrives.mockRejectedValueOnce(new Error("offline"));
    await useDrivesStore.getState().refresh();
    expect(useDrivesStore.getState().status).toBe("error");

    mockedListDrives.mockResolvedValue([{ root: "/", label: "/" }]);
    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().status).toBe("ready");
    expect(useDrivesStore.getState().error).toBeNull();
    expect(useDrivesStore.getState().drives).toEqual([
      { root: "/", label: "/" },
    ]);
  });
});
