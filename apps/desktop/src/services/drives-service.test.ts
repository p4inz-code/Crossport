import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { makeVolume } from "@/test/fixtures";
import { listDrives } from "./drives-service";
import { IpcError } from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

const SYSTEM = makeVolume();
const MEDIA = makeVolume({
  id: "D:",
  root: "D:\\",
  label: "MEDIA",
  name: "MEDIA",
  kind: "removable",
  filesystem: "exFAT",
  totalBytes: 64 * 1024 ** 3,
  freeBytes: 10 * 1024 ** 3,
  usedBytes: 54 * 1024 ** 3,
});

describe("drives service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("returns the volumes reported by the backend", async () => {
    mockedInvoke.mockResolvedValue([SYSTEM, MEDIA]);

    await expect(listDrives()).resolves.toEqual([SYSTEM, MEDIA]);
    expect(mockedInvoke).toHaveBeenCalledWith("list_drives", undefined);
  });

  it("accepts an empty list", async () => {
    mockedInvoke.mockResolvedValue([]);

    await expect(listDrives()).resolves.toEqual([]);
  });

  it("accepts volumes whose metadata the platform cannot report", async () => {
    const optical = makeVolume({
      id: "E:",
      root: "E:\\",
      label: "E:",
      kind: "optical",
      filesystem: null,
      totalBytes: null,
      freeBytes: null,
      usedBytes: null,
      readonly: null,
      mounted: false,
    });

    mockedInvoke.mockResolvedValue([optical]);

    await expect(listDrives()).resolves.toEqual([optical]);
  });

  it("rejects a payload that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue([{ root: "C:\\", label: "C:" }]);

    const error = await listDrives().catch((failure: unknown) => failure);

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("keeps the backend error code on failure", async () => {
    mockedInvoke.mockRejectedValue({
      code: "permission_denied",
      message: "permission denied: volume is locked",
    });

    const error = await listDrives().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("permission_denied");
    expect((error as IpcError).message).toBe(
      "permission denied: volume is locked",
    );
  });

  it("reports the browser runtime as unavailable", async () => {
    mockedIsTauri.mockReturnValue(false);

    const error = await listDrives().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("unavailable");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});
