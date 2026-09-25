import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as drivesService from "@/services/drives-service";
import * as filesystemService from "@/services/filesystem-service";
import { IpcError } from "@/services/ipc";
import { useDrivesStore } from "@/stores";
import { DrivesPage } from "./DrivesPage";

vi.mock("@/services/drives-service", () => ({ listDrives: vi.fn() }));
vi.mock("@/services/filesystem-service", () => ({
  inspectPath: vi.fn(),
  pickDirectory: vi.fn(),
}));

const mockedListDrives = vi.mocked(drivesService.listDrives);
const mockedPickDirectory = vi.mocked(filesystemService.pickDirectory);
const mockedInspectPath = vi.mocked(filesystemService.inspectPath);

const METADATA = {
  path: "D:\\Media",
  name: "Media",
  isDir: true,
  isFile: false,
  isSymlink: false,
  sizeBytes: 2048,
  modifiedMs: 1_700_000_000_000,
  readonly: false,
};

describe("DrivesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrivesStore.setState({ drives: [], status: "idle", error: null });
    mockedPickDirectory.mockResolvedValue(null);
  });

  it("lists the drives reported by the backend", async () => {
    mockedListDrives.mockResolvedValue([
      { root: "C:\\", label: "C:" },
      { root: "D:\\", label: "D:" },
    ]);

    render(<DrivesPage />);

    expect(await screen.findByText("C:")).toBeInTheDocument();
    expect(screen.getByText("D:\\")).toBeInTheDocument();
    expect(screen.getByText("C:\\")).toBeInTheDocument();
  });

  it("explains a failed enumeration and offers a retry", async () => {
    mockedListDrives.mockRejectedValue(
      new IpcError("permission_denied", "permission denied: volume is locked"),
    );

    render(<DrivesPage />);

    expect(
      await screen.findByText("Drive enumeration failed"),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "permission denied: volume is locked",
    );

    mockedListDrives.mockResolvedValue([{ root: "C:\\", label: "C:" }]);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(await screen.findByText("C:")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.queryByText("Drive enumeration failed")).toBeNull();
    });
  });

  it("shows an empty state when no drives are readable", async () => {
    mockedListDrives.mockResolvedValue([]);

    render(<DrivesPage />);

    expect(await screen.findByText("No drives detected")).toBeInTheDocument();
  });

  it("inspects the folder returned by the native picker", async () => {
    mockedListDrives.mockResolvedValue([]);
    mockedPickDirectory.mockResolvedValue("D:\\Media");
    mockedInspectPath.mockResolvedValue(METADATA);

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Browse for a folder/ }),
    );

    expect(await screen.findByText("D:\\Media")).toBeInTheDocument();
    expect(mockedInspectPath).toHaveBeenCalledWith("D:\\Media");
    expect(screen.getByText("Directory")).toBeInTheDocument();
    expect(screen.getByText("2 KB")).toBeInTheDocument();
  });

  it("stays quiet when the picker is cancelled", async () => {
    mockedListDrives.mockResolvedValue([]);
    mockedPickDirectory.mockResolvedValue(null);

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Browse for a folder/ }),
    );

    await waitFor(() => {
      expect(mockedPickDirectory).toHaveBeenCalledTimes(1);
    });
    expect(mockedInspectPath).not.toHaveBeenCalled();
    expect(screen.getByText(/No folder selected yet/)).toBeInTheDocument();
  });

  it("reports a structured path failure from the backend", async () => {
    mockedListDrives.mockResolvedValue([]);
    mockedPickDirectory.mockResolvedValue("D:\\Gone");
    mockedInspectPath.mockRejectedValue(
      new IpcError("path_not_found", "path not found: D:\\Gone"),
    );

    render(<DrivesPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Browse for a folder/ }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "path not found: D:\\Gone",
    );
  });
});
