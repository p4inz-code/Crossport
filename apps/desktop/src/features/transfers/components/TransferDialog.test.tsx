import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { makeTransferPreview, makeTransferPreviewRoot } from "@/test/fixtures";
import type { TransferRequest } from "@/types";
import { TransferDialog } from "./TransferDialog";

const REQUEST: TransferRequest = {
  sources: ["D:\\Photos"],
  destination: "D:\\Backup",
  operation: "copy",
  conflict: "skip",
};

function renderDialog(
  overrides: Partial<ComponentProps<typeof TransferDialog>> = {},
) {
  return render(
    <TransferDialog
      request={REQUEST}
      preview={makeTransferPreview()}
      busy={false}
      error={null}
      onConflictChange={vi.fn()}
      onConfirm={vi.fn()}
      onCancel={vi.fn()}
      {...overrides}
    />,
  );
}

describe("TransferDialog", () => {
  it("states what would happen before anything is queued", () => {
    renderDialog();

    expect(
      screen.getByRole("dialog", { name: "Copy 1 item to Backup" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Into D:\\Backup")).toBeInTheDocument();
    expect(screen.getByText("4 KB")).toBeInTheDocument();
    expect(screen.getByText("D:\\Photos")).toBeInTheDocument();
    expect(screen.getByText("D:\\Backup\\Photos")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start copy" })).toBeEnabled();
  });

  it("counts the items of a multi-source request", () => {
    renderDialog({
      request: {
        ...REQUEST,
        sources: ["D:\\Photos", "D:\\Music"],
        operation: "move",
      },
      preview: makeTransferPreview({ operation: "move", sameVolume: false }),
    });

    expect(
      screen.getByRole("dialog", { name: "Move 2 items to Backup" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start move" })).toBeEnabled();
  });

  it("waits for the dry run instead of allowing an unplanned start", () => {
    renderDialog({ preview: null });

    expect(screen.getByRole("status")).toHaveTextContent(
      "Checking the destination…",
    );
    expect(screen.getByRole("button", { name: "Start copy" })).toBeDisabled();
  });

  it("asks the backend to plan again when the conflict strategy changes", () => {
    const onConflictChange = vi.fn();
    renderDialog({ onConflictChange });

    fireEvent.click(screen.getByRole("radio", { name: /Keep both/ }));

    expect(onConflictChange).toHaveBeenCalledWith("rename");
  });

  it("shows every conflict strategy with its consequence", () => {
    renderDialog();

    expect(screen.getByRole("radio", { name: /Skip items/ })).toBeChecked();
    expect(
      screen.getByText(/Existing items are left exactly as they are/),
    ).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Replace what/ })).toBeEnabled();
  });

  it("warns about collisions and how much they would leave behind", () => {
    renderDialog({
      preview: makeTransferPreview({
        conflicts: 2,
        skippedItems: 2,
        skippedBytes: 2048,
      }),
    });

    expect(
      screen.getByText(/2 of these items already exist/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/2 would be left alone \(2 KB\)/),
    ).toBeInTheDocument();
  });

  it("explains a same-volume move and a cross-volume one differently", () => {
    renderDialog({
      request: { ...REQUEST, operation: "move" },
      preview: makeTransferPreview({ operation: "move", sameVolume: true }),
    });
    expect(screen.getByText(/this move is a rename/)).toBeInTheDocument();

    renderDialog({
      request: { ...REQUEST, operation: "move" },
      preview: makeTransferPreview({ operation: "move", sameVolume: false }),
    });
    expect(
      screen.getByText(/copied and then removed from the source/),
    ).toBeInTheDocument();
  });

  it("names a source the conflict strategy would leave alone", () => {
    renderDialog({
      preview: makeTransferPreview({
        roots: [makeTransferPreviewRoot({ skipped: true, files: 0, bytes: 0 })],
      }),
    });

    expect(
      screen.getByText("Will be left alone by the conflict strategy"),
    ).toBeInTheDocument();
  });

  it("says so when the destination free space is unknown", () => {
    renderDialog({ preview: makeTransferPreview({ availableBytes: null }) });

    expect(screen.getByText("Not reported")).toBeInTheDocument();
  });

  it("reports a planning or starting failure and keeps the request", () => {
    renderDialog({ error: "not enough space: 40 GB needed, 8 GB free" });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "not enough space: 40 GB needed, 8 GB free",
    );
    // The user can retry once the problem is dealt with.
    expect(screen.getByRole("button", { name: "Start copy" })).toBeEnabled();
  });

  it("disables both actions while a call is in flight", () => {
    renderDialog({ busy: true });

    expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    expect(screen.getByRole("radio", { name: /Skip items/ })).toBeDisabled();
  });

  it("confirms and cancels", () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    renderDialog({ onConfirm, onCancel });

    fireEvent.click(screen.getByRole("button", { name: "Start copy" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
