import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as drivesService from "@/services/drives-service";
import * as filesystemService from "@/services/filesystem-service";
import { IpcError } from "@/services/ipc";
import * as transferService from "@/services/transfer-service";
import { useBrowserStore, useDrivesStore, useTransferStore } from "@/stores";
import {
  makeEntry,
  makeListing,
  makeTransferPreview,
  makeTransferSnapshot,
  makeVolume,
} from "@/test/fixtures";
import type { DirectoryListing } from "@/types";
import { DrivesPage } from "./DrivesPage";

vi.mock("@/services/drives-service", () => ({ listDrives: vi.fn() }));
vi.mock("@/services/filesystem-service", () => ({
  listDirectory: vi.fn(),
  pickDirectory: vi.fn(),
}));
vi.mock("@/services/transfer-service", () => ({
  cancelTransfer: vi.fn(),
  clearFinishedTransfers: vi.fn(),
  getTransfer: vi.fn(),
  listTransfers: vi.fn(),
  pauseTransfer: vi.fn(),
  planTransfer: vi.fn(),
  removeTransfer: vi.fn(),
  resumeTransfer: vi.fn(),
  startTransfer: vi.fn(),
  subscribeToTransferUpdates: vi.fn(),
}));

const mockedListDrives = vi.mocked(drivesService.listDrives);
const mockedListDirectory = vi.mocked(filesystemService.listDirectory);
const mockedPickDirectory = vi.mocked(filesystemService.pickDirectory);
const mockedPlanTransfer = vi.mocked(transferService.planTransfer);
const mockedStartTransfer = vi.mocked(transferService.startTransfer);

const GIB = 1024 ** 3;

const SYSTEM = makeVolume({ label: "Windows", name: "Windows" });
const MEDIA = makeVolume({
  id: "D:",
  root: "D:\\",
  label: "MEDIA",
  name: "MEDIA",
  kind: "removable",
  filesystem: "exFAT",
  totalBytes: 64 * GIB,
  freeBytes: 8 * GIB,
  usedBytes: 56 * GIB,
});

const MEDIA_ROOT = makeListing({
  path: "D:\\",
  name: "D:\\",
  parent: null,
  entries: [
    makeEntry({
      name: "Photos",
      path: "D:\\Photos",
      kind: "directory",
      sizeBytes: null,
    }),
    makeEntry({ name: "readme.txt", path: "D:\\readme.txt", sizeBytes: 2048 }),
  ],
});

const PHOTOS = makeListing({
  path: "D:\\Photos",
  name: "Photos",
  parent: "D:\\",
  entries: [
    makeEntry({
      name: "trip.jpg",
      path: "D:\\Photos\\trip.jpg",
      sizeBytes: 4096,
    }),
  ],
});

describe("DrivesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrivesStore.setState({ drives: [], status: "idle", error: null });
    useBrowserStore.getState().close();
    useTransferStore.setState({
      jobs: [],
      status: "idle",
      error: null,
      pending: [],
      failures: {},
      dismissed: [],
    });
    mockedPickDirectory.mockResolvedValue(null);
  });

  it("lists the volumes the backend detected with their metadata", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);

    render(<DrivesPage />);

    expect(await screen.findByText("Windows")).toBeInTheDocument();
    expect(screen.getByText("C:\\")).toBeInTheDocument();
    expect(screen.getByText("MEDIA")).toBeInTheDocument();
    expect(screen.getByText("D:\\")).toBeInTheDocument();
    expect(screen.getByText("NTFS")).toBeInTheDocument();
    expect(screen.getByText("exFAT")).toBeInTheDocument();
    expect(screen.getByText("Internal disk")).toBeInTheDocument();
    expect(screen.getByText("Removable storage")).toBeInTheDocument();
    expect(screen.getByText("8 GB free of 64 GB")).toBeInTheDocument();
    expect(screen.getByText("No folder open")).toBeInTheDocument();
  });

  it("opens a volume root and lists its folders and files", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    mockedListDirectory.mockResolvedValue(MEDIA_ROOT);

    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));

    expect(
      await screen.findByRole("rowheader", { name: /Photos/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("rowheader", { name: /readme.txt/ }),
    ).toBeInTheDocument();
    expect(mockedListDirectory).toHaveBeenCalledWith("D:\\");
    expect(screen.getByText("2 KB")).toBeInTheDocument();
    expect(screen.getByText("1 folder · 1 file")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Up" })).toBeDisabled();
  });

  it("navigates into a folder, up to the parent, and back again", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    mockedListDirectory.mockResolvedValueOnce(MEDIA_ROOT);
    mockedListDirectory.mockResolvedValueOnce(PHOTOS);
    mockedListDirectory.mockResolvedValueOnce(MEDIA_ROOT);
    mockedListDirectory.mockResolvedValueOnce(PHOTOS);

    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));

    fireEvent.click(await screen.findByRole("button", { name: /Photos/ }));
    expect(
      await screen.findByRole("rowheader", { name: /trip.jpg/ }),
    ).toBeInTheDocument();
    expect(mockedListDirectory).toHaveBeenLastCalledWith("D:\\Photos");
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Up" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Up" }));
    expect(
      await screen.findByRole("rowheader", { name: /readme.txt/ }),
    ).toBeInTheDocument();
    expect(mockedListDirectory).toHaveBeenLastCalledWith("D:\\");
    expect(screen.getByRole("button", { name: "Up" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("rowheader", { name: /trip.jpg/ }),
    ).toBeInTheDocument();
    expect(mockedListDirectory).toHaveBeenLastCalledWith("D:\\Photos");
  });

  it("refreshes the folder without losing the location", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    mockedListDirectory.mockResolvedValue(MEDIA_ROOT);

    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));
    await screen.findByRole("rowheader", { name: /Photos/ });
    expect(mockedListDirectory).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => {
      expect(mockedListDirectory).toHaveBeenCalledTimes(2);
    });
    expect(mockedListDirectory).toHaveBeenLastCalledWith("D:\\");
    expect(screen.getByText("1 folder · 1 file")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
  });

  it("recovers when a volume disappeared before it was opened", async () => {
    const optical = makeVolume({
      id: "E:",
      root: "E:\\",
      label: "E:",
      kind: "optical",
      mounted: false,
      filesystem: null,
      totalBytes: null,
      freeBytes: null,
      usedBytes: null,
      readonly: null,
    });
    mockedListDrives.mockResolvedValue([SYSTEM, optical]);
    mockedListDirectory.mockRejectedValue(
      new IpcError("path_not_found", "path not found: E:\\"),
    );

    render(<DrivesPage />);
    expect(await screen.findByText("Not available")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /E:/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "path not found: E:\\",
    );

    fireEvent.click(screen.getByRole("button", { name: "Back to volumes" }));
    expect(screen.getByText("No folder open")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("retries a failed folder without leaving the location", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    mockedListDirectory.mockRejectedValueOnce(
      new IpcError("permission_denied", "permission denied: D:\\"),
    );
    mockedListDirectory.mockResolvedValueOnce(MEDIA_ROOT);

    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "permission denied: D:\\",
    );

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(
      await screen.findByRole("rowheader", { name: /Photos/ }),
    ).toBeInTheDocument();
  });

  it("shows a loading state while the backend reads a folder", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    let resolveListing: (listing: DirectoryListing) => void = () => undefined;
    mockedListDirectory.mockReturnValue(
      new Promise<DirectoryListing>((resolve) => {
        resolveListing = resolve;
      }),
    );

    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Reading folder…",
    );

    resolveListing(MEDIA_ROOT);
    expect(
      await screen.findByRole("rowheader", { name: /Photos/ }),
    ).toBeInTheDocument();
  });

  it("opens a folder chosen in the native dialog", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM]);
    mockedPickDirectory.mockResolvedValue("D:\\Project");
    mockedListDirectory.mockResolvedValue(
      makeListing({ path: "D:\\Project", name: "Project" }),
    );

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Open folder…" }),
    );

    await waitFor(() => {
      expect(mockedListDirectory).toHaveBeenCalledWith("D:\\Project");
    });
    expect(screen.getByText("D:\\Project")).toBeInTheDocument();
  });

  it("stays quiet when the folder dialog is cancelled", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM]);
    mockedPickDirectory.mockResolvedValue(null);

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Open folder…" }),
    );

    await waitFor(() => {
      expect(mockedPickDirectory).toHaveBeenCalledTimes(1);
    });
    expect(mockedListDirectory).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText("No folder open")).toBeInTheDocument();
  });

  it("reports a picker failure without touching the browser", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM]);
    mockedPickDirectory.mockRejectedValue(
      new IpcError(
        "unavailable",
        "The 'pick_directory' feature is only available in the CrossPort desktop app.",
      ),
    );

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Open folder…" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "only available in the CrossPort desktop app",
    );
    expect(screen.getByText("No folder open")).toBeInTheDocument();
  });

  it("explains an empty volume list", async () => {
    mockedListDrives.mockResolvedValue([]);

    render(<DrivesPage />);

    expect(await screen.findByText("No volumes detected")).toBeInTheDocument();
  });

  it("explains a failed volume enumeration and recovers on retry", async () => {
    mockedListDrives.mockRejectedValue(
      new IpcError("permission_denied", "permission denied: volume is locked"),
    );

    render(<DrivesPage />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "permission denied: volume is locked",
    );

    mockedListDrives.mockResolvedValue([SYSTEM]);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(await screen.findByText("Windows")).toBeInTheDocument();
  });

  it("refreshes the volume list from the page header", async () => {
    mockedListDrives.mockResolvedValue([SYSTEM]);

    render(<DrivesPage />);
    await screen.findByText("Windows");

    mockedListDrives.mockResolvedValue([SYSTEM, MEDIA]);
    fireEvent.click(screen.getByRole("button", { name: "Refresh volumes" }));

    expect(await screen.findByText("MEDIA")).toBeInTheDocument();
  });
});

describe("DrivesPage transfer composition", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrivesStore.setState({ drives: [], status: "idle", error: null });
    useBrowserStore.getState().close();
    useTransferStore.setState({
      jobs: [],
      status: "idle",
      error: null,
      pending: [],
      failures: {},
      dismissed: [],
    });
    mockedListDrives.mockResolvedValue([MEDIA]);
    mockedListDirectory.mockResolvedValue(MEDIA_ROOT);
  });

  /** Opens D:\ and checks the Photos folder. */
  async function checkPhotos(): Promise<void> {
    render(<DrivesPage />);
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select Photos" }),
    );
  }

  it("plans the transfer before offering to start it", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(makeTransferPreview());

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));

    await waitFor(() => {
      expect(mockedPlanTransfer).toHaveBeenCalledWith({
        sources: ["D:\\Photos"],
        destination: "D:\\Backup",
        operation: "copy",
        conflict: "skip",
      });
    });

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Copy 1 item");
    expect(dialog).toHaveTextContent("4 KB");
    expect(mockedStartTransfer).not.toHaveBeenCalled();
  });

  it("does nothing when the destination dialog is cancelled", async () => {
    mockedPickDirectory.mockResolvedValue(null);

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Move to…/ }));

    await waitFor(() => {
      expect(mockedPickDirectory).toHaveBeenCalledTimes(1);
    });
    expect(mockedPlanTransfer).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("asks the backend to plan again for a different conflict strategy", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(makeTransferPreview());

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));
    await screen.findByRole("dialog");

    mockedPlanTransfer.mockResolvedValue(
      makeTransferPreview({ conflict: "rename" }),
    );
    fireEvent.click(screen.getByRole("radio", { name: /Keep both/ }));

    await waitFor(() => {
      expect(mockedPlanTransfer).toHaveBeenCalledTimes(2);
    });
    expect(mockedPlanTransfer).toHaveBeenLastCalledWith({
      sources: ["D:\\Photos"],
      destination: "D:\\Backup",
      operation: "copy",
      conflict: "rename",
    });
    expect(screen.getByRole("radio", { name: /Keep both/ })).toBeChecked();
  });

  it("queues the job on confirmation and adopts its snapshot", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(makeTransferPreview());
    const snapshot = makeTransferSnapshot({ status: "queued" });
    mockedStartTransfer.mockResolvedValue(snapshot);

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: "Start copy" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(mockedStartTransfer).toHaveBeenCalledWith({
      sources: ["D:\\Photos"],
      destination: "D:\\Backup",
      operation: "copy",
      conflict: "skip",
    });
    expect(useTransferStore.getState().jobs.map((job) => job.id)).toEqual([
      snapshot.id,
    ]);
    expect(screen.getByText(/1 item queued for copy/)).toBeInTheDocument();
  });

  it("explains a refused request instead of opening a dialog", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Photos\\Backup");
    mockedPlanTransfer.mockRejectedValue(
      new IpcError(
        "unsafe_relationship",
        "the destination is inside the source",
      ),
    );

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "the destination is inside the source",
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps the dialog open when starting is refused", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(makeTransferPreview());
    mockedStartTransfer.mockRejectedValue(
      new IpcError("not_enough_space", "not enough space: 4 KB needed"),
    );

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "Start copy" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "not enough space: 4 KB needed",
    );
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(useTransferStore.getState().jobs).toEqual([]);
  });

  it("closes the dialog without starting anything", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(makeTransferPreview());

    await checkPhotos();
    fireEvent.click(screen.getByRole("button", { name: /Move to…/ }));
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mockedStartTransfer).not.toHaveBeenCalled();
  });
});
