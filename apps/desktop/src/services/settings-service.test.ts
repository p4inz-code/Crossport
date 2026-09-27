import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_HISTORY_LIMIT, DEFAULT_SETTINGS } from "@/types";
import { IpcError } from "./ipc";
import { getSettings, updateSettings } from "./settings-service";

/** The settings every write in this suite sends, spelled out once. */
const BASE = {
  verification: DEFAULT_SETTINGS.verification,
  historyLimit: DEFAULT_HISTORY_LIMIT,
} as const;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

const STORAGE_KEY = "crossport:settings";

describe("settings service in a browser", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(false);
    window.localStorage.clear();
  });

  it("returns defaults when nothing is stored", async () => {
    await expect(getSettings()).resolves.toEqual(DEFAULT_SETTINGS);
  });

  it("reads and validates stored settings", async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ theme: "dark", locale: "fr", ...BASE }),
    );

    await expect(getSettings()).resolves.toEqual({
      theme: "dark",
      locale: "fr",
      ...BASE,
    });
  });

  it("falls back to defaults for invalid stored data", async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ theme: "neon" }));

    await expect(getSettings()).resolves.toEqual(DEFAULT_SETTINGS);
  });

  it("falls back to defaults for unparseable stored data", async () => {
    window.localStorage.setItem(STORAGE_KEY, "{not json");

    await expect(getSettings()).resolves.toEqual(DEFAULT_SETTINGS);
  });

  it("persists validated settings without touching IPC", async () => {
    await updateSettings({ theme: "light", locale: "de", ...BASE });

    expect(
      JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null"),
    ).toEqual({ theme: "light", locale: "de", ...BASE });
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("rejects invalid settings before persisting them", async () => {
    const invalid = { theme: "neon", locale: "de" } as unknown as Parameters<
      typeof updateSettings
    >[0];

    const error = await updateSettings(invalid).catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_input");
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});

describe("settings service inside Tauri", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
    window.localStorage.clear();
  });

  it("loads settings from the backend", async () => {
    mockedInvoke.mockResolvedValue({ theme: "dark", locale: "pt-BR", ...BASE });

    await expect(getSettings()).resolves.toEqual({
      theme: "dark",
      locale: "pt-BR",
      ...BASE,
    });
    expect(mockedInvoke).toHaveBeenCalledWith("get_settings", undefined);
  });

  it("rejects a backend payload that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue({ theme: "neon", locale: "en" });

    const error = await getSettings().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("sends validated settings to the backend and does not write local state", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await updateSettings({ theme: "light", locale: "de", ...BASE });

    expect(mockedInvoke).toHaveBeenCalledWith("update_settings", {
      settings: { theme: "light", locale: "de", ...BASE },
    });
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("keeps the backend error code when persistence fails", async () => {
    mockedInvoke.mockRejectedValue({
      code: "permission_denied",
      message: "permission denied: settings.json",
    });

    const error = await updateSettings({
      theme: "light",
      locale: "de",
      ...BASE,
    }).catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("permission_denied");
    expect((error as IpcError).message).toBe(
      "permission denied: settings.json",
    );
  });

  it("validates locally before calling the backend", async () => {
    const invalid = { theme: "light", locale: "x" } as unknown as Parameters<
      typeof updateSettings
    >[0];

    const error = await updateSettings(invalid).catch(
      (failure: unknown) => failure,
    );

    expect((error as IpcError).code).toBe("invalid_input");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});
