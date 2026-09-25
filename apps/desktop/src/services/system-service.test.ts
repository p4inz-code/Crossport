import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "./ipc";
import { getSystemInfo } from "./system-service";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

const WINDOWS_INFO = {
  platform: "windows",
  os: "windows",
  arch: "x86_64",
  family: "windows",
};

describe("system service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("returns the platform facts reported by the backend", async () => {
    mockedInvoke.mockResolvedValue(WINDOWS_INFO);

    await expect(getSystemInfo()).resolves.toEqual(WINDOWS_INFO);
    expect(mockedInvoke).toHaveBeenCalledWith("get_system_info", undefined);
  });

  it("accepts every published platform", async () => {
    for (const platform of ["windows", "macos", "linux", "other"]) {
      mockedInvoke.mockResolvedValue({ ...WINDOWS_INFO, platform });
      await expect(getSystemInfo()).resolves.toMatchObject({ platform });
    }
  });

  it("rejects a platform outside the contract", async () => {
    mockedInvoke.mockResolvedValue({ ...WINDOWS_INFO, platform: "plan9" });

    const error = await getSystemInfo().catch((failure: unknown) => failure);

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("rejects an empty host string", async () => {
    mockedInvoke.mockResolvedValue({ ...WINDOWS_INFO, arch: "" });

    const error = await getSystemInfo().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("reports the browser runtime as unavailable", async () => {
    mockedIsTauri.mockReturnValue(false);

    const error = await getSystemInfo().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("unavailable");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});
