import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { listDrives } from "./drives-service";
import { IpcError } from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

describe("drives service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("returns the drives reported by the backend", async () => {
    mockedInvoke.mockResolvedValue([
      { root: "C:\\", label: "C:" },
      { root: "D:\\", label: "D:" },
    ]);

    await expect(listDrives()).resolves.toEqual([
      { root: "C:\\", label: "C:" },
      { root: "D:\\", label: "D:" },
    ]);
    expect(mockedInvoke).toHaveBeenCalledWith("list_drives", undefined);
  });

  it("accepts an empty list", async () => {
    mockedInvoke.mockResolvedValue([]);

    await expect(listDrives()).resolves.toEqual([]);
  });

  it("rejects a payload that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue([{ root: 1, label: "C:" }]);

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
