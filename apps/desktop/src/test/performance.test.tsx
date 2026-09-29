/* ==========================================================================
 * Performance regression tests
 * The browser surfaces must stay usable at the largest datasets the backend
 * can hand them: a 10,000-entry listing (the listing cap), a 2,000-record
 * history (the retention cap), and a 500-job queue.
 *
 * Each test asserts a budget far above the measured cost on a development
 * machine, so it is a regression detector, not a benchmark. On CI the budgets
 * are multiplied by `CI_BUDGET_FACTOR`, because a shared runner measures the
 * machine's load as much as the code's cost — the ratio a regression moves is
 * still visible, without a busy host deciding the result. The baseline
 * recorded with the Phase 6 measurements (jsdom, Windows, Sep 2026):
 *
 * - DirectoryBrowser, 10,000 entries: first render 173 ms, one checkbox 6 ms
 *   (before the render window and the memoized rows: 1,672 ms / 649 ms)
 * - zod validation of a 10,000-entry listing: 6 ms
 * - HistoryPage, 2,000 records: 584 ms, one selection 207 ms
 * - TransferQueue, 500 jobs: 366 ms
 * ========================================================================== */

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DirectoryBrowser } from "@/features/drives/components/DirectoryBrowser";
import { HistoryPage } from "@/features/history/pages/HistoryPage";
import { TransferQueue } from "@/features/transfers/components/TransferQueue";
import * as service from "@/services/history-service";
import { useHistoryStore } from "@/stores";
import {
  makeEntry,
  makeHistoryRecord,
  makeListing,
  makeTransferSnapshot,
} from "@/test/fixtures";
import { renderPage } from "@/test/render-page";
import { directoryListingSchema } from "@/types";

vi.mock("@/services/history-service", () => ({
  clearHistory: vi.fn(),
  deleteHistoryRecord: vi.fn(),
  getArchiveStatus: vi.fn(),
  listHistory: vi.fn(),
}));

afterEach(cleanup);

const now = () => globalThis.performance.now();

/** Set by CI runners; absent on a development machine. */
const CI = Boolean(
  (
    globalThis as {
      process?: { env?: Record<string, string | undefined> };
    }
  ).process?.env?.CI,
);

/** A timing budget, relaxed only where the machine is shared with other jobs. */
const budget = (milliseconds: number) => (CI ? milliseconds * 3 : milliseconds);

/** The largest listing the backend will return. */
const LISTING_ENTRIES = 10_000;
/** The largest history the settings allow. */
const HISTORY_RECORDS = 2_000;

function hugeListing() {
  return makeListing({
    path: "C:\\data",
    entries: Array.from({ length: LISTING_ENTRIES }, (_, index) =>
      makeEntry({
        name: `file-${index.toString().padStart(5, "0")}.bin`,
        path: `C:\\data\\file-${index}.bin`,
        sizeBytes: 1024 * index,
      }),
    ),
  });
}

describe("performance budgets", () => {
  it("validates the largest listing at the IPC boundary", () => {
    const listing = hugeListing();

    const started = now();
    directoryListingSchema.parse(listing);
    const took = now() - started;

    expect(took).toBeLessThan(budget(250));
  });

  it("paints the largest listing without rendering every row", () => {
    const listing = hugeListing();

    const started = now();
    const view = render(
      <DirectoryBrowser
        location={listing.path}
        trail={[]}
        listing={listing}
        status="ready"
        error={null}
        canGoBack={false}
        canGoForward={false}
        canGoUp={false}
        pickerBusy={false}
        transferBusy={false}
        alert={null}
        onBack={() => {}}
        onForward={() => {}}
        onUp={() => {}}
        onRefresh={() => {}}
        onOpen={() => {}}
        onOpenFolder={() => {}}
        onTransferRequest={() => {}}
        onLeave={() => {}}
      />,
    );
    const firstRender = now() - started;

    // The window is stated, never silent: the user is told how much is shown.
    expect(
      screen.getByText(/Showing 400 of 10000 entries/),
    ).toBeInTheDocument();

    const boxes = view.container.querySelectorAll("input[type=checkbox]");
    const box = boxes[boxes.length - 1] as HTMLInputElement;
    const clickStarted = now();
    fireEvent.click(box);
    const click = now() - clickStarted;

    expect(firstRender).toBeLessThan(budget(1200));
    expect(click).toBeLessThan(budget(250));
  });

  it("renders and selects in the largest retained history", async () => {
    const records = Array.from({ length: HISTORY_RECORDS }, (_, index) =>
      makeHistoryRecord({
        id: `transfer-${index}`,
        sources: [`C:\\data\\file-${index}.bin`],
      }),
    );
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
    vi.mocked(service.listHistory).mockResolvedValue({
      records,
      total: records.length,
      limit: HISTORY_RECORDS,
      filter: "all",
      status: { state: "loaded", detail: null },
      writable: true,
    });
    vi.mocked(service.getArchiveStatus).mockResolvedValue({
      history: { state: "loaded", detail: null },
      state: { state: "missing", detail: null },
      degraded: false,
      writable: true,
      historyRecords: records.length,
      historyLimit: HISTORY_RECORDS,
      interruptedJobs: 0,
    });

    const started = now();
    renderPage(<HistoryPage />);
    await screen.findByText(/file-1999\.bin/);
    const firstRender = now() - started;

    const list = screen.getByRole("list");
    const row = list.querySelector("button");
    if (row === null) {
      throw new Error("the history list rendered no rows");
    }
    const selectStarted = now();
    fireEvent.click(row);
    const select = now() - selectStarted;

    expect(firstRender).toBeLessThan(budget(2500));
    expect(select).toBeLessThan(budget(1_000));
  });

  it("renders a long queue", () => {
    const jobs = Array.from({ length: 500 }, (_, index) =>
      makeTransferSnapshot({ id: `transfer-${index}` }),
    );

    const started = now();
    render(
      <TransferQueue
        jobs={jobs}
        status="ready"
        error={null}
        pending={[]}
        failures={{}}
        onPause={() => {}}
        onResume={() => {}}
        onCancel={() => {}}
        onRemove={() => {}}
        onRetry={() => {}}
        onBrowse={() => {}}
        onOpenHistory={() => {}}
      />,
    );
    const took = now() - started;

    expect(took).toBeLessThan(budget(2000));
  });
});
