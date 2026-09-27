import { beforeEach, describe, expect, it, vi } from "vitest";

import * as service from "@/services/history-service";
import { IpcError } from "@/services/ipc";
import { makeHistoryRecord } from "@/test/fixtures";
import type { ArchiveStatus, HistoryFilter } from "@/types";
import { useHistoryStore } from "./history-store";

vi.mock("@/services/history-service", () => ({
  clearHistory: vi.fn(),
  deleteHistoryRecord: vi.fn(),
  getArchiveStatus: vi.fn(),
  listHistory: vi.fn(),
}));

const mockedList = vi.mocked(service.listHistory);
const mockedStatus = vi.mocked(service.getArchiveStatus);
const mockedDelete = vi.mocked(service.deleteHistoryRecord);
const mockedClear = vi.mocked(service.clearHistory);

const ARCHIVE: ArchiveStatus = {
  history: { state: "loaded", detail: null },
  state: { state: "missing", detail: null },
  degraded: false,
  writable: true,
  historyRecords: 2,
  historyLimit: 200,
  interruptedJobs: 0,
};

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

describe("history store", () => {
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

  it("loads the listing and the archive status together", async () => {
    mockedList.mockResolvedValue(listing());
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().records).toHaveLength(1);
    expect(useHistoryStore.getState().total).toBe(1);
    expect(useHistoryStore.getState().archive?.historyLimit).toBe(200);
    expect(useHistoryStore.getState().status).toBe("ready");
    expect(mockedList).toHaveBeenCalledWith("all");
  });

  it("loads a filter the caller asked for", async () => {
    mockedList.mockResolvedValue(listing([], "failed"));
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().load("failed");

    expect(mockedList).toHaveBeenCalledWith("failed");
    expect(useHistoryStore.getState().filter).toBe("failed");
  });

  it("keeps a degraded document's state instead of hiding it", async () => {
    const recovered = {
      state: "recovered" as const,
      detail: "the file is not valid JSON",
    };
    mockedList.mockResolvedValue({
      ...listing([]),
      status: recovered,
      writable: true,
    });
    mockedStatus.mockResolvedValue({ ...ARCHIVE, degraded: true });

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().documentStatus).toEqual(recovered);
    expect(useHistoryStore.getState().writable).toBe(true);
    expect(useHistoryStore.getState().status).toBe("ready");
  });

  it("reports a newer document as read-only", async () => {
    mockedList.mockResolvedValue({
      ...listing([]),
      status: {
        state: "unsupported",
        detail: "schema version 99 is newer than this build understands",
      },
      writable: false,
    });
    mockedStatus.mockResolvedValue({
      ...ARCHIVE,
      writable: false,
      degraded: true,
    });

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().writable).toBe(false);
    expect(useHistoryStore.getState().documentStatus?.state).toBe(
      "unsupported",
    );
  });

  it("drops a selection the new listing no longer holds", async () => {
    useHistoryStore.setState({ selectedId: "transfer-gone" });
    mockedList.mockResolvedValue(listing([makeHistoryRecord()]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().selectedId).toBeNull();
  });

  it("keeps a selection the listing still holds", async () => {
    const record = makeHistoryRecord();
    useHistoryStore.setState({ selectedId: record.id });
    mockedList.mockResolvedValue(listing([record]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().selectedId).toBe(record.id);
  });

  it("deletes a record and reloads the listing", async () => {
    const record = makeHistoryRecord();
    useHistoryStore.setState({ records: [record], selectedId: record.id });
    mockedDelete.mockResolvedValue(true);
    mockedList.mockResolvedValue(listing([]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().remove(record.id);

    expect(mockedDelete).toHaveBeenCalledWith(record.id);
    expect(useHistoryStore.getState().records).toEqual([]);
    expect(useHistoryStore.getState().selectedId).toBeNull();
  });

  it("records a structured failure when a delete is refused", async () => {
    const record = makeHistoryRecord();
    useHistoryStore.setState({ records: [record] });
    mockedDelete.mockRejectedValue(
      new IpcError("invalid_input", "no history entry with that identifier"),
    );

    await useHistoryStore.getState().remove(record.id);

    expect(useHistoryStore.getState().error?.code).toBe("invalid_input");
    expect(useHistoryStore.getState().records).toEqual([record]);
  });

  it("clears every record and reloads", async () => {
    const record = makeHistoryRecord();
    useHistoryStore.setState({ records: [record], selectedId: record.id });
    mockedClear.mockResolvedValue(1);
    mockedList.mockResolvedValue(listing([]));
    mockedStatus.mockResolvedValue(ARCHIVE);

    await useHistoryStore.getState().clear();

    expect(mockedClear).toHaveBeenCalled();
    expect(useHistoryStore.getState().selectedId).toBeNull();
    expect(useHistoryStore.getState().records).toEqual([]);
  });

  it("surfaces a load failure without keeping stale records", async () => {
    useHistoryStore.setState({ records: [makeHistoryRecord()] });
    mockedList.mockRejectedValue(new IpcError("unavailable", "desktop only"));
    mockedStatus.mockRejectedValue(new IpcError("unavailable", "desktop only"));

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().status).toBe("error");
    expect(useHistoryStore.getState().error?.code).toBe("unavailable");
    expect(useHistoryStore.getState().records).toEqual([]);
  });
});
