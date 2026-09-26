import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as drivesService from "@/services/drives-service";
import * as filesystemService from "@/services/filesystem-service";
import { IpcError } from "@/services/ipc";
import { useBrowserStore, useDrivesStore } from "@/stores";
import { makeEntry, makeListing, makeVolume } from "@/test/fixtures";
import type { DirectoryListing } from "@/types";
import { DrivesPage } from "./DrivesPage";

vi.mock("@/services/drives-service", () => ({ listDrives: vi.fn() }));
vi.mock("@/services/filesystem-service", () => ({
  listDirectory: vi.fn(),
  pickDirectory: vi.fn(),
}));

const mockedListDrives = vi.mocked(drivesService.listDrives);
const mockedListDirectory = vi.mocked(filesystemService.listDirectory);
const mockedPickDirectory = vi.mocked(filesystemService.pickDirectory);

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
