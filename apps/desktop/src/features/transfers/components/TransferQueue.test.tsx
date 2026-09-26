import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import {
  makeTransferIssue,
  makeTransferProgress,
  makeTransferSnapshot,
} from "@/test/fixtures";
import type { TransferSnapshot } from "@/types";
import { TransferQueue } from "./TransferQueue";

function renderQueue(
  overrides: Partial<ComponentProps<typeof TransferQueue>> = {},
) {
  return render(
    <TransferQueue
      jobs={[]}
      status="idle"
      error={null}
      pending={[]}
      failures={{}}
      onPause={vi.fn()}
      onResume={vi.fn()}
      onCancel={vi.fn()}
      onRemove={vi.fn()}
      onRetry={vi.fn()}
      {...overrides}
    />,
  );
}

const RUNNING = makeTransferSnapshot({ id: "transfer-1" });

describe("TransferQueue", () => {
  it("asks the user to start a transfer when the queue is empty", () => {
    renderQueue({ status: "ready" });

    expect(screen.getByText("No transfers yet")).toBeInTheDocument();
    expect(screen.queryByRole("list")).toBeNull();
  });

  it("shows a loading state while the queue is read", () => {
    renderQueue({ status: "loading" });

    expect(screen.getByRole("status")).toHaveTextContent(
      "Reading the transfer queue…",
    );
  });

  it("explains a failed listing and offers a retry", () => {
    const onRetry = vi.fn();
    renderQueue({
      status: "error",
      error: new IpcError("unavailable", "only available in the desktop app"),
      onRetry,
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "only available in the desktop app",
    );

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("reports what a running job is doing", () => {
    renderQueue({ jobs: [RUNNING], status: "ready" });

    expect(screen.getByText("Photos")).toBeInTheDocument();
    expect(screen.getByText("Transferring")).toBeInTheDocument();
    expect(screen.getByText("Copy into D:\\Backup")).toBeInTheDocument();
    expect(screen.getByText("1 KB of 4 KB")).toBeInTheDocument();
    expect(
      screen.getByText("1 of 2 files · 1 of 1 folders"),
    ).toBeInTheDocument();
    expect(screen.getByText("1 KB/s · 3s left")).toBeInTheDocument();
    expect(screen.getByText("1s elapsed")).toBeInTheDocument();
    expect(
      screen.getByText("D:\\Backup\\Photos\\trip.jpg"),
    ).toBeInTheDocument();
  });

  it("exposes progress to assistive technology", () => {
    renderQueue({ jobs: [RUNNING], status: "ready" });

    const bar = screen.getByRole("progressbar", { name: "Photos progress" });
    expect(bar).toHaveAttribute("aria-valuenow", "25");
    expect(screen.getByText("25%")).toBeInTheDocument();
  });

  it("admits when it cannot measure progress instead of inventing a number", () => {
    renderQueue({
      jobs: [
        makeTransferSnapshot({
          progress: makeTransferProgress({
            percent: null,
            totalBytes: 0,
            transferredBytes: 0,
          }),
        }),
      ],
      status: "ready",
    });

    const bar = screen.getByRole("progressbar", { name: "Photos progress" });
    expect(bar).not.toHaveAttribute("aria-valuenow");
    expect(screen.getByText("Working…")).toBeInTheDocument();
    expect(screen.getByText("No file data")).toBeInTheDocument();
  });

  it("offers pause, resume, and cancel exactly where they apply", () => {
    const onPause = vi.fn();
    const onResume = vi.fn();
    const onCancel = vi.fn();
    renderQueue({
      jobs: [
        makeTransferSnapshot({ id: "running", status: "running" }),
        makeTransferSnapshot({ id: "paused", status: "paused" }),
        makeTransferSnapshot({ id: "done", status: "completed" }),
      ],
      status: "ready",
      onPause,
      onResume,
      onCancel,
    });

    const pause = screen.getAllByRole("button", { name: "Pause" });
    expect(pause).toHaveLength(1);
    fireEvent.click(pause[0] as HTMLElement);
    expect(onPause).toHaveBeenCalledWith("running");

    const resume = screen.getByRole("button", { name: /Resume/ });
    fireEvent.click(resume);
    expect(onResume).toHaveBeenCalledWith("paused");

    // Two live jobs can be cancelled; the finished one can only be removed.
    expect(screen.getAllByRole("button", { name: /Cancel/ })).toHaveLength(2);
    expect(
      screen.getByRole("button", { name: /Remove from list/ }),
    ).toBeInTheDocument();
  });

  it("asks before a cancel discards partial output", () => {
    const onCancel = vi.fn();
    renderQueue({ jobs: [RUNNING], status: "ready", onCancel });

    fireEvent.click(screen.getByRole("button", { name: /Cancel/ }));

    expect(onCancel).not.toHaveBeenCalled();
    expect(screen.getByText("Discard this transfer?")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Keep it" }));
    expect(onCancel).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /^Cancel/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^Cancel/ }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, cancel it" }));
    expect(onCancel).toHaveBeenCalledWith(RUNNING.id);
  });

  it("disables the controls of a job while its call is in flight", () => {
    renderQueue({
      jobs: [RUNNING],
      status: "ready",
      pending: [RUNNING.id],
    });

    expect(screen.getByRole("button", { name: "Pause" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Cancel/ })).toBeDisabled();
  });

  it("lists skipped and failed items without a failure being fatal to the list", () => {
    const job = makeTransferSnapshot({
      issues: [
        makeTransferIssue({ path: "D:\\Backup\\notes.txt" }),
        makeTransferIssue({
          path: "D:\\Backup\\trip.jpg",
          reason: "failed",
          detail: null,
          error: { code: "disk_full", message: "disk full: D:\\" },
        }),
      ],
      progress: makeTransferProgress({ failedItems: 1, skippedItems: 1 }),
    });

    renderQueue({ jobs: [job], status: "ready" });

    const summary = screen.getByText("1 failed · 1 skipped");
    expect(summary).toBeInTheDocument();
    // The issue list is collapsed until the user opens it.
    expect(summary.closest("details")?.open).toBe(false);

    fireEvent.click(summary);

    expect(screen.getByText("disk full: D:\\")).toBeInTheDocument();
    expect(
      screen.getByText("The destination volume is full."),
    ).toBeInTheDocument();
    expect(screen.getByText("destination already exists")).toBeInTheDocument();
  });

  it("counts the issues it does not list", () => {
    const job = makeTransferSnapshot({
      issues: Array.from({ length: 6 }, (_value, index) =>
        makeTransferIssue({
          path: `D:\\Backup\\file-${index}.txt`,
          detail: "destination already exists",
        }),
      ),
      issuesTruncated: true,
      progress: makeTransferProgress({ skippedItems: 6 }),
    });

    renderQueue({ jobs: [job], status: "ready" });
    fireEvent.click(screen.getByText("6 skipped"));

    expect(screen.getByText("…and 2 more.")).toBeInTheDocument();
    expect(
      screen.getByText("More issues were recorded than the engine keeps."),
    ).toBeInTheDocument();
  });

  it("surfaces a job-level failure and a control failure", () => {
    const job = makeTransferSnapshot({
      id: "transfer-9",
      status: "failed",
      error: { code: "transfer_failed", message: "2 items failed" },
    });

    renderQueue({
      jobs: [job],
      status: "ready",
      failures: { "transfer-9": new IpcError("io", "the queue is busy") },
    });

    // The control failure is the freshest thing that happened, so it wins.
    expect(screen.getByRole("alert")).toHaveTextContent("the queue is busy");
  });

  it("describes an unknown failure code without losing it", () => {
    const job = makeTransferSnapshot({
      issues: [
        makeTransferIssue({
          reason: "failed",
          detail: null,
          error: { code: "quota_exceeded", message: "the share is full" },
        }),
      ],
      progress: makeTransferProgress({ failedItems: 1 }),
    });

    renderQueue({ jobs: [job], status: "ready" });
    fireEvent.click(screen.getByText("1 failed"));

    expect(screen.getByText("the share is full")).toBeInTheDocument();
  });

  it("names every job in the queue in engine order", () => {
    renderQueue({
      jobs: [
        makeTransferSnapshot({ id: "a", sources: ["D:\\Photos"] }),
        makeTransferSnapshot({ id: "b", sources: ["D:\\Music"] }),
      ],
      status: "ready",
    });

    const items = screen.getAllByRole("listitem");
    expect(items.map((item) => item.textContent)).toEqual([
      expect.stringContaining("Photos"),
      expect.stringContaining("Music"),
    ]);
  });

  it("counts the extra sources of a multi-item job", () => {
    renderQueue({
      jobs: [
        makeTransferSnapshot({
          sources: ["D:\\Photos", "D:\\Music", "D:\\Videos"],
        }),
      ],
      status: "ready",
    });

    expect(screen.getByText("Photos + 2 more")).toBeInTheDocument();
  });

  it("keeps showing jobs that arrived even when the listing failed", () => {
    // A refresh failure empties the queue in the store; a job can still arrive
    // afterwards over the event feed, and it belongs on screen.
    const jobs: TransferSnapshot[] = [RUNNING];
    renderQueue({
      jobs,
      status: "error",
      error: new IpcError("io", "the queue is busy"),
    });

    expect(screen.getByRole("alert")).toHaveTextContent("the queue is busy");
    expect(screen.getByText("Photos")).toBeInTheDocument();
  });
});
