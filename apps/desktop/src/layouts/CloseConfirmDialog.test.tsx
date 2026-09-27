import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as appService from "@/services/app-service";
import { renderPage } from "@/test/render-page";
import { CloseConfirmDialog } from "./CloseConfirmDialog";

vi.mock("@/services/app-service", () => ({
  exitApp: vi.fn(),
  subscribeToCloseRequests: vi.fn(),
}));

const mockedSubscribe = vi.mocked(appService.subscribeToCloseRequests);
const mockedExit = vi.mocked(appService.exitApp);

/** Hands the dialog a warning, as Rust does when a close is held. */
async function holdClose(warning: {
  liveJobs: number;
  interruptedJobs: number;
  detail: string;
}): Promise<void> {
  let emit: (value: unknown) => void = () => {};
  mockedSubscribe.mockImplementation(async (handler) => {
    emit = handler as (value: unknown) => void;
    return () => {};
  });

  renderPage(<CloseConfirmDialog />);
  await waitFor(() => {
    expect(mockedSubscribe).toHaveBeenCalledTimes(1);
  });

  emit(warning);
  await screen.findByRole("dialog");
}

describe("CloseConfirmDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedExit.mockResolvedValue(undefined);
  });

  it("asks nothing when nothing is in flight", () => {
    mockedSubscribe.mockResolvedValue(() => {});

    renderPage(<CloseConfirmDialog />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("says what closing would set aside, in the backend's words", async () => {
    await holdClose({
      liveJobs: 2,
      interruptedJobs: 1,
      detail: "2 transfers are running and 1 is waiting for a decision.",
    });

    const dialog = screen.getByRole("dialog", { name: "Close CrossPort?" });
    expect(dialog).toHaveTextContent(
      "2 transfers are running and 1 is waiting for a decision.",
    );
    expect(dialog).toHaveTextContent("Running now");
    expect(dialog).toHaveTextContent("Waiting for a decision");
    // The safe answer is the one that has focus.
    expect(screen.getByRole("button", { name: "Keep working" })).toHaveFocus();
  });

  it("keeps working when the user dismisses the question", async () => {
    await holdClose({
      liveJobs: 1,
      interruptedJobs: 0,
      detail: "1 transfer is running.",
    });

    fireEvent.click(screen.getByRole("button", { name: "Keep working" }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mockedExit).not.toHaveBeenCalled();
  });

  it("dismisses with Escape, because the close can be asked for again", async () => {
    await holdClose({
      liveJobs: 1,
      interruptedJobs: 0,
      detail: "1 transfer is running.",
    });

    fireEvent.keyDown(document, { key: "Escape" });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mockedExit).not.toHaveBeenCalled();
  });

  it("closes only when the user says so", async () => {
    await holdClose({
      liveJobs: 1,
      interruptedJobs: 0,
      detail: "1 transfer is running.",
    });

    fireEvent.click(screen.getByRole("button", { name: "Close anyway" }));

    await waitFor(() => {
      expect(mockedExit).toHaveBeenCalledTimes(1);
    });
  });

  it("survives a close the backend refuses", async () => {
    mockedExit.mockRejectedValue(new Error("the window is not available"));
    await holdClose({
      liveJobs: 1,
      interruptedJobs: 0,
      detail: "1 transfer is running.",
    });

    fireEvent.click(screen.getByRole("button", { name: "Close anyway" }));

    // The application is still here, and the question is still on screen.
    await waitFor(() => {
      expect(mockedExit).toHaveBeenCalledTimes(1);
    });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
