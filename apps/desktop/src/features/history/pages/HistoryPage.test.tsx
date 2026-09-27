import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as service from "@/services/history-service";
import { IpcError } from "@/services/ipc";
import { useHistoryStore } from "@/stores";
import { makeHistoryRecord } from "@/test/fixtures";
import type { ArchiveStatus, HistoryFilter } from "@/types";
import { HistoryPage } from "./HistoryPage";

vi.mock("@/services/history-service", () => ({
  clearHistory: vi.fn(),
  deleteHistoryRecord: vi.fn(),
  getArchiveStatus: vi.fn(),
  listHistory: vi.fn(),
}));

const mockedList = vi.mocked(service.listHistory);
const mockedStatus = vi.mocked(service.getArchiveStatus);
const mockedDelete = vi.mocked(service.deleteHistoryRecord);

const ARCHIVE: ArchiveStatus = {
  history: { state: "loaded", detail: null },
  state: { state: "missing", detail: null },
  degraded: false,
  writable: true,
  historyRecords: 1,
  historyLimit: 200,
  interruptedJobs: 0,
};

/**
 * Finds a row in the record list rather than a filter button.
 *
 * Both use the same status words, so the list is scoped explicitly: a test
 * that clicked the filter instead of the row would pass for the wrong reason.
 */
async function findRow(name: RegExp) {
  const list = await screen.findByRole("list");
  return within(list).getByRole("button", { name });
}

function listing(
  records = [makeHistoryRecord()],
  filter: HistoryFilter = "all",
) {
  return {
    records,
    total: records.length,
    limit: 200,
    filter,
    status: { state: "loaded" as const, detail: null },
    writable: true,
  };
}

describe("HistoryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useHistoryStore.setState({
      records: [],
      filter: "all",
      total: 0,
      limit: 0,
      documentStatus: null,
      writable: true,
      archive: null,
      status: "idle",
      error: null,
      selectedId: null,
      pending: [],
    });
  });

  it("lists finished transfers with their verdict", async () => {
    mockedList.mockResolvedValue(listing());
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);

    expect(await screen.findByText("Completed")).toBeInTheDocument();
    expect(
      screen.getByText(/verified \(size, 2 files, 4096 bytes\)/),
    ).toBeInTheDocument();
    expect(screen.getByText("2 of 2 files · 4 KB of 4 KB")).toBeInTheDocument();
  });

  it("opens the details panel with the structured failure of a failed transfer", async () => {
    const failed = makeHistoryRecord({
      status: "failed",
      error: {
        code: "permission_denied",
        message: "permission denied: D:\\Backup\\report.txt",
      },
    });
    mockedList.mockResolvedValue(listing([failed]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);
    fireEvent.click(await findRow(/Failed/));

    expect(await screen.findByText("Why it failed")).toBeInTheDocument();
    expect(screen.getByText("permission_denied")).toBeInTheDocument();
    expect(
      screen.getByText("permission denied: D:\\Backup\\report.txt"),
    ).toBeInTheDocument();
  });

  it("shows the items a failed transfer reported", async () => {
    const failed = makeHistoryRecord({
      status: "failed",
      error: null,
      issues: [
        {
          path: "D:\\Backup\\report.txt",
          reason: "failed",
          error: { code: "io", message: "the device is not ready" },
          detail: null,
        },
      ],
    });
    mockedList.mockResolvedValue(listing([failed]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);
    fireEvent.click(await findRow(/Failed/));

    expect(await screen.findByText("Items reported")).toBeInTheDocument();
    expect(
      screen.getByText("D:\\Backup\\report.txt — the device is not ready"),
    ).toBeInTheDocument();
  });

  it("filters through the backend rather than the loaded page", async () => {
    mockedList.mockResolvedValue(listing());
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);
    await screen.findByText("Completed");
    fireEvent.click(screen.getByRole("button", { name: /Cancelled/ }));

    await waitFor(() => expect(mockedList).toHaveBeenCalledWith("cancelled"));
  });

  it("says a record predates verification instead of implying one ran", async () => {
    mockedList.mockResolvedValue(
      listing([makeHistoryRecord({ verification: null })]),
    );
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);

    expect(
      await screen.findByText("No verification recorded"),
    ).toBeInTheDocument();
  });

  it("deletes a record and reloads", async () => {
    const record = makeHistoryRecord();
    mockedList.mockResolvedValue(listing([record]));
    mockedStatus.mockResolvedValue(ARCHIVE);
    mockedDelete.mockResolvedValue(true);

    render(<HistoryPage />);
    fireEvent.click(await findRow(/Completed/));
    fireEvent.click(screen.getByRole("button", { name: /Delete record/ }));

    await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(record.id));
  });

  it("clears history only when there is something to clear", async () => {
    mockedList.mockResolvedValue(listing([]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    render(<HistoryPage />);
    await screen.findByText("No transfers match this filter");

    expect(
      screen.getByRole("button", { name: /Clear history/ }),
    ).toBeDisabled();
  });

  it("explains a history document that could not be used", async () => {
    mockedList.mockResolvedValue({
      ...listing([]),
      status: { state: "recovered", detail: "the file was empty" },
    });
    mockedStatus.mockResolvedValue({ ...ARCHIVE, degraded: true });

    render(<HistoryPage />);

    expect(
      await screen.findByText("Unusable and set aside"),
    ).toBeInTheDocument();
    expect(screen.getByText(/the file was empty/)).toBeInTheDocument();
  });

  it("never offers destructive controls for a read-only document", async () => {
    mockedList.mockResolvedValue({
      ...listing([]),
      status: { state: "unsupported", detail: "schema version 99" },
      writable: false,
    });
    mockedStatus.mockResolvedValue({
      ...ARCHIVE,
      writable: false,
      degraded: true,
    });

    render(<HistoryPage />);

    expect(
      await screen.findByText("Written by a newer version"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Clear history/ }),
    ).toBeDisabled();
  });

  it("reports a load failure with a retry", async () => {
    mockedList.mockRejectedValue(new IpcError("unavailable", "desktop only"));
    mockedStatus.mockRejectedValue(new IpcError("unavailable", "desktop only"));

    render(<HistoryPage />);

    expect(
      await screen.findByText("History could not be loaded"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Try again" }),
    ).toBeInTheDocument();
  });
});
