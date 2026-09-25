import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/settings-service";
import { DEFAULT_SETTINGS } from "@/types";
import { useSettingsStore } from "./settings-store";

vi.mock("@/services/settings-service", () => ({
  getSettings: vi.fn(),
  updateSettings: vi.fn(),
}));

const mockedGet = vi.mocked(service.getSettings);
const mockedUpdate = vi.mocked(service.updateSettings);

function resetStore(): void {
  useSettingsStore.setState({
    ...DEFAULT_SETTINGS,
    status: "idle",
    error: null,
    saveStatus: "idle",
  });
}

describe("settings store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
  });

  it("hydrates settings from the service", async () => {
    mockedGet.mockResolvedValue({ theme: "dark", locale: "ja" });

    await useSettingsStore.getState().hydrate();

    expect(useSettingsStore.getState()).toMatchObject({
      theme: "dark",
      locale: "ja",
      status: "ready",
      error: null,
    });
  });

  it("keeps defaults when hydration fails and records the error", async () => {
    mockedGet.mockRejectedValue(
      new IpcError("internal", "backend unreachable"),
    );

    await useSettingsStore.getState().hydrate();

    expect(useSettingsStore.getState()).toMatchObject({
      ...DEFAULT_SETTINGS,
      status: "error",
    });
    expect(useSettingsStore.getState().error?.code).toBe("internal");
  });

  it("setTheme updates state and persists the full settings object", async () => {
    useSettingsStore.setState({ theme: "light", locale: "en" });
    mockedUpdate.mockResolvedValue();

    useSettingsStore.getState().setTheme("dark");

    expect(useSettingsStore.getState().theme).toBe("dark");
    await vi.waitFor(() => {
      expect(mockedUpdate).toHaveBeenCalledWith({
        theme: "dark",
        locale: "en",
      });
      expect(useSettingsStore.getState().saveStatus).toBe("saved");
    });
  });

  it("setLocale trims and persists the full settings object", async () => {
    useSettingsStore.setState({ theme: "system", locale: "en" });
    mockedUpdate.mockResolvedValue();

    useSettingsStore.getState().setLocale("  pt-BR  ");

    expect(useSettingsStore.getState().locale).toBe("pt-BR");
    await vi.waitFor(() =>
      expect(mockedUpdate).toHaveBeenCalledWith({
        theme: "system",
        locale: "pt-BR",
      }),
    );
  });

  it("rejects an invalid locale locally without calling the backend", () => {
    mockedUpdate.mockResolvedValue();

    useSettingsStore.getState().setLocale("x");

    expect(mockedUpdate).not.toHaveBeenCalled();
    expect(useSettingsStore.getState().locale).toBe(DEFAULT_SETTINGS.locale);
    expect(useSettingsStore.getState().saveStatus).toBe("error");
    expect(useSettingsStore.getState().error?.code).toBe("invalid_input");
    expect(useSettingsStore.getState().error?.message).toContain("locale");
  });

  it("does not lose other fields when persisting a single change", async () => {
    useSettingsStore.setState({ theme: "light", locale: "es" });
    mockedUpdate.mockResolvedValue();

    useSettingsStore.getState().setTheme("system");

    await vi.waitFor(() =>
      expect(mockedUpdate).toHaveBeenCalledWith({
        theme: "system",
        locale: "es",
      }),
    );
  });

  it("keeps the applied value but reports a failed write", async () => {
    useSettingsStore.setState({ theme: "light", locale: "en" });
    mockedUpdate.mockRejectedValue(
      new IpcError("permission_denied", "permission denied: settings.json"),
    );

    useSettingsStore.getState().setTheme("dark");

    expect(useSettingsStore.getState().theme).toBe("dark");
    await vi.waitFor(() => {
      expect(useSettingsStore.getState().saveStatus).toBe("error");
      expect(useSettingsStore.getState().error?.code).toBe("permission_denied");
    });
  });
});
