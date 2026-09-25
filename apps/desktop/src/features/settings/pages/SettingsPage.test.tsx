import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/settings-service";
import { useSettingsStore } from "@/stores";
import { DEFAULT_SETTINGS } from "@/types";
import { SettingsPage } from "./SettingsPage";

vi.mock("@/services/settings-service", () => ({
  getSettings: vi.fn(),
  updateSettings: vi.fn(),
}));

const mockedUpdate = vi.mocked(service.updateSettings);

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedUpdate.mockResolvedValue();
    useSettingsStore.setState({
      ...DEFAULT_SETTINGS,
      status: "ready",
      error: null,
      saveStatus: "idle",
    });
  });

  it("marks the persisted theme as active", () => {
    useSettingsStore.setState({ theme: "dark" });

    render(<SettingsPage />);

    expect(screen.getByRole("button", { name: "Dark" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("persists a theme change through the store", async () => {
    render(<SettingsPage />);

    fireEvent.click(screen.getByRole("button", { name: "Dark" }));

    expect(useSettingsStore.getState().theme).toBe("dark");
    await waitFor(() => {
      expect(mockedUpdate).toHaveBeenCalledWith({
        theme: "dark",
        locale: DEFAULT_SETTINGS.locale,
      });
    });
    expect(
      await screen.findByText("Settings saved by the backend."),
    ).toBeInTheDocument();
  });

  it("applies a locale from the form", async () => {
    render(<SettingsPage />);

    const input = screen.getByLabelText("Locale");
    fireEvent.change(input, { target: { value: "pt-BR" } });
    fireEvent.click(screen.getByRole("button", { name: "Save locale" }));

    expect(useSettingsStore.getState().locale).toBe("pt-BR");
    await waitFor(() => {
      expect(mockedUpdate).toHaveBeenCalledWith({
        theme: DEFAULT_SETTINGS.theme,
        locale: "pt-BR",
      });
    });
  });

  it("shows the structured error when the backend rejects a change", async () => {
    mockedUpdate.mockRejectedValue(
      new IpcError("permission_denied", "permission denied: settings.json"),
    );

    render(<SettingsPage />);
    fireEvent.click(screen.getByRole("button", { name: "Light" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "permission denied: settings.json",
    );
  });
});
