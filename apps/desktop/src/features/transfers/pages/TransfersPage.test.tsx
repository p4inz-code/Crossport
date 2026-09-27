import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/transfer-service";
import { useTransferStore } from "@/stores";
import { makeTransferSnapshot } from "@/test/fixtures";
import { renderPage, withRouter } from "@/test/render-page";
import { TransfersPage } from "./TransfersPage";

vi.mock("@/services/transfer-service", () => ({
  cancelTransfer: vi.fn(),
  clearFinishedTransfers: vi.fn(),
  listTransfers: vi.fn(),
  pauseTransfer: vi.fn(),
  removeTransfer: vi.fn(),
  resumeTransfer: vi.fn(),
  subscribeToTransferUpdates: vi.fn(),
}));

const mockedList = vi.mocked(service.listTransfers);
const mockedPause = vi.mocked(service.pauseTransfer);
const mockedClear = vi.mocked(service.clearFinishedTransfers);
const mockedRemove = vi.mocked(service.removeTransfer);

const ACTIVE = makeTransferSnapshot({ id: "transfer-a", status: "running" });
const DONE = makeTransferSnapshot({
  id: "transfer-b",
  status: "completed",
  queuedAtMs: ACTIVE.queuedAtMs + 1000,
  progress: { ...ACTIVE.progress, percent: 100 },
});

function seed(
  jobs: (typeof ACTIVE)[],
  overrides: Partial<ReturnType<typeof useTransferStore.getState>> = {},
): void {
  useTransferStore.setState({
    jobs,
    status: "ready",
    error: null,
    pending: [],
    failures: {},
    dismissed: [],
    ...overrides,
  });
}

describe("TransfersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    seed([]);
  });

  it("shows the jobs the store holds, in order", () => {
    seed([ACTIVE, DONE]);

    renderPage(<TransfersPage />);

    expect(screen.getByText("Transferring")).toBeInTheDocument();
    expect(screen.getByText("Completed")).toBeInTheDocument();
    // Both jobs report where they are going, one row each.
    expect(screen.getAllByText("Copy into D:\\Backup")).toHaveLength(2);
  });

  it("explains an empty queue", () => {
    renderPage(<TransfersPage />);

    expect(screen.getByText("No transfers yet")).toBeInTheDocument();
  });

  it("offers to clear finished jobs only when there are some", async () => {
    seed([ACTIVE]);
    const { rerender } = renderPage(<TransfersPage />);

    expect(
      screen.getByRole("button", { name: /Clear finished/ }),
    ).toBeDisabled();

    seed([ACTIVE, DONE]);
    rerender(withRouter(<TransfersPage />));

    const clear = screen.getByRole("button", { name: "Clear finished (1)" });
    expect(clear).toBeEnabled();

    mockedClear.mockResolvedValue(1);
    fireEvent.click(clear);

    await waitFor(() => {
      expect(screen.queryByText("Completed")).toBeNull();
    });
    expect(mockedClear).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Transferring")).toBeInTheDocument();
  });

  it("pauses a running job and adopts the snapshot the backend returns", async () => {
    seed([ACTIVE]);
    mockedPause.mockResolvedValue({ ...ACTIVE, status: "paused" });

    renderPage(<TransfersPage />);
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    expect(mockedPause).toHaveBeenCalledWith(ACTIVE.id);
    expect(await screen.findByText("Paused")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Resume/ })).toBeEnabled();
  });

  it("explains a control failure next to the job", async () => {
    seed([ACTIVE]);
    mockedPause.mockRejectedValue(
      new IpcError("transfer_not_found", "transfer not found: transfer-a"),
    );

    renderPage(<TransfersPage />);
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "transfer not found: transfer-a",
    );
    expect(screen.getByText("Transferring")).toBeInTheDocument();
  });

  it("drops a finished job from the list", async () => {
    seed([DONE]);
    mockedRemove.mockResolvedValue(undefined);

    renderPage(<TransfersPage />);
    fireEvent.click(screen.getByRole("button", { name: /Remove from list/ }));

    await waitFor(() => {
      expect(screen.queryByText("Completed")).toBeNull();
    });
    expect(screen.getByText("No transfers yet")).toBeInTheDocument();
  });

  it("explains a queue that could not be read and retries it", async () => {
    seed([], {
      status: "error",
      error: new IpcError("unavailable", "only available in the desktop app"),
    });

    renderPage(<TransfersPage />);
    expect(screen.getByRole("alert")).toHaveTextContent(
      "only available in the desktop app",
    );

    mockedList.mockResolvedValue([ACTIVE]);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(await screen.findByText("Transferring")).toBeInTheDocument();
  });

  it("refreshes the queue from the page header", async () => {
    seed([ACTIVE]);
    mockedList.mockResolvedValue([ACTIVE, DONE]);

    renderPage(<TransfersPage />);
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));

    expect(await screen.findByText("Completed")).toBeInTheDocument();
  });
});
