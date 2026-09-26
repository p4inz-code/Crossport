import { beforeEach, describe, expect, it, vi } from "vitest";
import * as service from "@/services/drives-service";
import { IpcError } from "@/services/ipc";
import { makeVolume } from "@/test/fixtures";
import { useDrivesStore } from "./drives-store";

vi.mock("@/services/drives-service", () => ({
  listDrives: vi.fn(),
}));

const mockedListDrives = vi.mocked(service.listDrives);

const SYSTEM = makeVolume();
const MEDIA = makeVolume({
  id: "D:",
  root: "D:\\",
  label: "MEDIA",
  name: "MEDIA",
  kind: "removable",
  filesystem: "exFAT",
  totalBytes: 64 * 1024 ** 3,
  freeBytes: 8 * 1024 ** 3,
  usedBytes: 56 * 1024 ** 3,
});

describe("drives store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrivesStore.setState({ drives: [], status: "idle", error: null });
  });

  it("starts idle with no drives", () => {
    expect(useDrivesStore.getState().status).toBe("idle");
    expect(useDrivesStore.getState().drives).toEqual([]);
  });

  it("stores the volumes reported by the backend", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([SYSTEM, MEDIA]);
    expect(useDrivesStore.getState().status).toBe("ready");
    expect(useDrivesStore.getState().error).toBeNull();
  });

  it("keeps volumes the platform could not fully describe", async () => {
    const optical = makeVolume({
      id: "E:",
      root: "E:\\",
      label: "E:",
      kind: "optical",
      filesystem: null,
      totalBytes: null,
      freeBytes: null,
      usedBytes: null,
      mounted: false,
    });
    mockedListDrives.mockResolvedValue([optical]);

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([optical]);
    expect(useDrivesStore.getState().status).toBe("ready");
  });

  it("reports an empty result as a ready state", async () => {
    mockedListDrives.mockResolvedValue([]);

    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().drives).toEqual([]);
    expect(useDrivesStore.getState().status).toBe("ready");
  });

  it("surfaces a structured error and clears stale drives", async () => {
    useDrivesStore.setState({ drives: [SYSTEM], status: "ready" });
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

    mockedListDrives.mockResolvedValue([SYSTEM]);
    await useDrivesStore.getState().refresh();

    expect(useDrivesStore.getState().status).toBe("ready");
    expect(useDrivesStore.getState().error).toBeNull();
    expect(useDrivesStore.getState().drives).toEqual([SYSTEM]);
  });
});
