import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/system-service";
import { useSystemStore } from "./system-store";

vi.mock("@/services/system-service", () => ({
  getSystemInfo: vi.fn(),
}));

const mockedGetSystemInfo = vi.mocked(service.getSystemInfo);

const INFO = {
  platform: "windows",
  os: "windows",
  arch: "x86_64",
  family: "windows",
} as const;

describe("system store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSystemStore.setState({ info: null, status: "idle", error: null });
  });

  it("holds the platform facts after hydration", async () => {
    mockedGetSystemInfo.mockResolvedValue(INFO);

    await useSystemStore.getState().hydrate();

    expect(useSystemStore.getState().info).toEqual(INFO);
    expect(useSystemStore.getState().status).toBe("ready");
    expect(useSystemStore.getState().error).toBeNull();
  });

  it("reports a browser runtime as an error instead of faking a platform", async () => {
    mockedGetSystemInfo.mockRejectedValue(
      new IpcError("unavailable", "requires the desktop app"),
    );

    await useSystemStore.getState().hydrate();

    expect(useSystemStore.getState().info).toBeNull();
    expect(useSystemStore.getState().status).toBe("error");
    expect(useSystemStore.getState().error?.code).toBe("unavailable");
  });

  it("normalises unexpected failures", async () => {
    mockedGetSystemInfo.mockRejectedValue(new Error("backend exploded"));

    await useSystemStore.getState().hydrate();

    expect(useSystemStore.getState().error).toBeInstanceOf(IpcError);
    expect(useSystemStore.getState().error?.message).toBe("backend exploded");
  });
});
