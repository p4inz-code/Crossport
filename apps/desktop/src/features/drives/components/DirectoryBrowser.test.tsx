import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import { makeEntry, makeListing } from "@/test/fixtures";
import { DirectoryBrowser } from "./DirectoryBrowser";

const LISTING = makeListing({
  entries: [
    makeEntry({
      name: "Users",
      path: "C:\\Users",
      kind: "directory",
      sizeBytes: null,
    }),
    makeEntry({ name: "notes.txt", path: "C:\\notes.txt", sizeBytes: 2048 }),
  ],
});

function renderBrowser(
  overrides: Partial<ComponentProps<typeof DirectoryBrowser>> = {},
) {
  return render(
    <DirectoryBrowser
      location={null}
      listing={null}
      status="idle"
      error={null}
      canGoBack={false}
      canGoUp={false}
      pickerBusy={false}
      transferBusy={false}
      alert={null}
      onBack={vi.fn()}
      onUp={vi.fn()}
      onRefresh={vi.fn()}
      onOpen={vi.fn()}
      onOpenFolder={vi.fn()}
      onTransferRequest={vi.fn()}
      onLeave={vi.fn()}
      {...overrides}
    />,
  );
}

describe("DirectoryBrowser", () => {
  it("asks the user to choose a folder when nothing is open", () => {
    renderBrowser();

    expect(screen.getByText("No folder open")).toBeInTheDocument();
    expect(screen.getByText("No folder selected")).toBeInTheDocument();
  });

  it("shows the location and the entries of the listed folder", () => {
    renderBrowser({ location: "C:\\", listing: LISTING, status: "ready" });

    expect(screen.getByText("C:\\")).toBeInTheDocument();
    expect(
      screen.getByRole("rowheader", { name: /Users/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("rowheader", { name: /notes.txt/ }),
    ).toBeInTheDocument();
    expect(screen.getByText("2 KB")).toBeInTheDocument();
    expect(screen.getByText("1 folder · 1 file")).toBeInTheDocument();
  });

  it("opens folders but only selects files and links", () => {
    const onOpen = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: makeListing({
        entries: [
          makeEntry({
            name: "Users",
            path: "C:\\Users",
            kind: "directory",
            sizeBytes: null,
          }),
          makeEntry({ name: "notes.txt", path: "C:\\notes.txt" }),
          makeEntry({
            name: "shortcut",
            path: "C:\\shortcut",
            kind: "symlink",
            sizeBytes: null,
          }),
        ],
      }),
      status: "ready",
      onOpen,
    });

    fireEvent.click(screen.getByRole("button", { name: /Users/ }));
    expect(onOpen).toHaveBeenCalledWith("C:\\Users");

    fireEvent.click(screen.getByRole("button", { name: /notes.txt/ }));
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Selected/)).toHaveTextContent("notes.txt");

    // A link is not followed by a single click; Enter or a double click asks.
    fireEvent.click(screen.getByRole("button", { name: /shortcut/ }));
    expect(onOpen).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(screen.getByRole("button", { name: /shortcut/ }), {
      key: "Enter",
    });
    expect(onOpen).toHaveBeenCalledWith("C:\\shortcut");
  });

  it("reports nothing to open for a folder with no entries", () => {
    renderBrowser({
      location: "C:\\Empty",
      listing: makeListing({ path: "C:\\Empty", name: "Empty", entries: [] }),
      status: "ready",
    });

    expect(screen.getByText("This folder is empty")).toBeInTheDocument();
    // The toolbar and the empty state both name the folder.
    expect(screen.getAllByText("C:\\Empty")).toHaveLength(2);
    expect(screen.getByText("0 folders · 0 files")).toBeInTheDocument();
  });

  it("shows a loading state while the backend reads the folder", () => {
    renderBrowser({ location: "C:\\", status: "loading" });

    expect(screen.getByRole("status")).toHaveTextContent("Reading folder…");
  });

  it("explains a failed listing and offers recovery", () => {
    const onRefresh = vi.fn();
    const onLeave = vi.fn();
    renderBrowser({
      location: "D:\\",
      status: "error",
      error: new IpcError("path_not_found", "path not found: D:\\"),
      onRefresh,
      onLeave,
    });

    expect(screen.getByRole("alert")).toHaveTextContent("path not found: D:\\");

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Back to volumes" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  it("surfaces a picker problem separately from a listing failure", () => {
    renderBrowser({
      alert: "The picker is only available in the desktop app.",
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "The picker is only available in the desktop app.",
    );
    expect(screen.getByText("No folder open")).toBeInTheDocument();
  });

  it("opens a folder with a double click or Enter like a file manager", () => {
    const onOpen = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: makeListing({
        entries: [
          makeEntry({
            name: "Users",
            path: "C:\\Users",
            kind: "directory",
            sizeBytes: null,
          }),
        ],
      }),
      status: "ready",
      onOpen,
    });

    const row = screen.getByRole("button", { name: /Users/ });
    fireEvent.doubleClick(row);
    expect(onOpen).toHaveBeenCalledWith("C:\\Users");

    fireEvent.keyDown(row, { key: "Enter" });
    expect(onOpen).toHaveBeenCalledTimes(2);
  });

  it("warns when a folder is larger than CrossPort lists at once", () => {
    renderBrowser({
      location: "C:\\Huge",
      listing: makeListing({
        path: "C:\\Huge",
        name: "Huge",
        entries: [makeEntry({ name: "one.txt", path: "C:\\Huge\\one.txt" })],
        truncated: true,
      }),
      status: "ready",
    });

    expect(screen.getByRole("status")).toHaveTextContent(
      "Showing the first 1 entries",
    );
  });

  it("only offers navigation the backend supports", () => {
    const onBack = vi.fn();
    const onUp = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      canGoBack: false,
      canGoUp: false,
      onBack,
      onUp,
    });

    const back = screen.getByRole("button", { name: "Back" });
    const up = screen.getByRole("button", { name: "Up" });
    expect(back).toBeDisabled();
    expect(up).toBeDisabled();

    fireEvent.click(back);
    fireEvent.click(up);
    expect(onBack).not.toHaveBeenCalled();
    expect(onUp).not.toHaveBeenCalled();
  });

  it("supports back and up with the keyboard shortcuts", () => {
    const onBack = vi.fn();
    const onUp = vi.fn();
    renderBrowser({
      location: "C:\\Users",
      listing: makeListing({ path: "C:\\Users", parent: "C:\\" }),
      status: "ready",
      canGoBack: true,
      canGoUp: true,
      onBack,
      onUp,
    });

    const browser = screen.getByRole("region", { name: "Directory browser" });
    fireEvent.keyDown(browser, { key: "ArrowLeft", altKey: true });
    fireEvent.keyDown(browser, { key: "ArrowUp", altKey: true });

    expect(onBack).toHaveBeenCalledTimes(1);
    expect(onUp).toHaveBeenCalledTimes(1);
  });

  it("refreshes and opens the native dialog from the toolbar", () => {
    const onRefresh = vi.fn();
    const onOpenFolder = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      canGoBack: true,
      canGoUp: true,
      onRefresh,
      onOpenFolder,
    });

    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    fireEvent.click(screen.getByRole("button", { name: "Open folder…" }));

    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(onOpenFolder).toHaveBeenCalledTimes(1);
  });

  it("waits for the native dialog instead of allowing a second one", () => {
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      pickerBusy: true,
    });

    expect(
      screen.getByRole("button", { name: /Waiting for the dialog/ }),
    ).toBeDisabled();
  });

  it("only offers a transfer once something is checked", () => {
    renderBrowser({ location: "C:\\", listing: LISTING, status: "ready" });

    expect(screen.getByRole("button", { name: /Copy to…/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Move to…/ })).toBeDisabled();
    expect(screen.queryByText(/selected/)).toBeNull();
  });

  it("reports the checked items and the operation the user chose", () => {
    const onTransferRequest = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      onTransferRequest,
    });

    fireEvent.click(screen.getByRole("checkbox", { name: "Select Users" }));

    // The summary names what the engine will actually carry.
    expect(screen.getByText("1 item · size not reported")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Move to…/ }));
    expect(onTransferRequest).toHaveBeenCalledWith("move", ["C:\\Users"]);
  });

  it("reports every checked item and the paths the backend produced", () => {
    const onTransferRequest = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      onTransferRequest,
    });

    fireEvent.click(
      screen.getByRole("checkbox", { name: "Select all entries" }),
    );

    expect(screen.getByText("2 items · at least 2 KB")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));
    expect(onTransferRequest).toHaveBeenCalledWith("copy", [
      "C:\\Users",
      "C:\\notes.txt",
    ]);
  });

  it("unchecks everything from the select-all box and the status bar", () => {
    renderBrowser({ location: "C:\\", listing: LISTING, status: "ready" });

    const selectAll = screen.getByRole("checkbox", {
      name: "Select all entries",
    });
    fireEvent.click(selectAll);
    expect(screen.getByRole("button", { name: /Copy to…/ })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Clear selection" }));

    expect(screen.getByRole("button", { name: /Copy to…/ })).toBeDisabled();
    expect(selectAll).not.toBeChecked();
  });

  it("does not offer to transfer while a transfer is being composed", () => {
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      transferBusy: true,
    });

    fireEvent.click(screen.getByRole("checkbox", { name: "Select Users" }));

    expect(screen.getByRole("button", { name: /Copy to…/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Move to…/ })).toBeDisabled();
  });

  it("names entries the engine will not copy", () => {
    renderBrowser({
      location: "C:\\",
      listing: makeListing({
        entries: [
          makeEntry({ name: "notes.txt", path: "C:\\notes.txt" }),
          makeEntry({
            name: "shortcut",
            path: "C:\\shortcut",
            kind: "symlink",
            sizeBytes: null,
          }),
        ],
      }),
      status: "ready",
    });

    fireEvent.click(
      screen.getByRole("checkbox", { name: "Select all entries" }),
    );

    expect(
      screen.getByText("2 items · at least 2 KB · 1 entry is not copied"),
    ).toBeInTheDocument();
  });

  it("keeps the selection out of the way of opening folders", () => {
    const onOpen = vi.fn();
    renderBrowser({
      location: "C:\\",
      listing: LISTING,
      status: "ready",
      onOpen,
    });

    fireEvent.click(screen.getByRole("checkbox", { name: "Select Users" }));
    fireEvent.click(screen.getByRole("button", { name: /Users/ }));

    expect(onOpen).toHaveBeenCalledWith("C:\\Users");
    expect(screen.getByText("1 item · size not reported")).toBeInTheDocument();
  });

  it("describes entries the platform could not measure", () => {
    renderBrowser({
      location: "C:\\",
      listing: makeListing({
        entries: [
          makeEntry({
            name: "dangling",
            path: "C:\\dangling",
            kind: "symlink",
            sizeBytes: null,
            modifiedMs: null,
            readonly: null,
          }),
        ],
      }),
      status: "ready",
    });

    expect(screen.getByText("Symlink")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(
      screen.getByText("0 folders · 0 files · 1 other"),
    ).toBeInTheDocument();
  });
});
