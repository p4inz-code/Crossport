/* ==========================================================================
 * Application journeys
 * The whole product, wired together: real pages, real stores, real routing,
 * with only the Rust boundary mocked. Each test walks one journey the way a
 * user does — through the surfaces, not around them — so a break in the wiring
 * between them fails here.
 * ========================================================================== */

import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import {
  MemoryRouter,
  NavLink,
  Route,
  Routes,
  useLocation,
} from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DrivesPage } from "@/features/drives";
import { HistoryPage } from "@/features/history";
import { NotificationStack } from "@/features/notifications";
import { RecoveryPage } from "@/features/recovery";
import { TransfersPage } from "@/features/transfers";
import { useRecoveryFeed, useTransferNotifications } from "@/hooks";
import * as appService from "@/services/app-service";
import * as drivesService from "@/services/drives-service";
import * as filesystemService from "@/services/filesystem-service";
import * as historyService from "@/services/history-service";
import { IpcError } from "@/services/ipc";
import * as recoveryService from "@/services/recovery-service";
import * as transferService from "@/services/transfer-service";
import {
  useNotificationStore,
  useRecoveryStore,
  useTransferStore,
} from "@/stores";
import {
  makeEntry,
  makeHistoryRecord,
  makeListing,
  makeRecoveryCandidate,
  makeTransferPreview,
  makeTransferSnapshot,
  makeVolume,
} from "@/test/fixtures";
import type { TransferSnapshot } from "@/types";

vi.mock("@/services/app-service", () => ({
  exitApp: vi.fn(),
  subscribeToCloseRequests: vi.fn(async () => () => {}),
}));
vi.mock("@/services/drives-service", () => ({ listDrives: vi.fn() }));
vi.mock("@/services/filesystem-service", () => ({
  listAncestors: vi.fn(),
  listDirectory: vi.fn(),
  pickDirectory: vi.fn(),
}));
vi.mock("@/services/history-service", () => ({
  clearHistory: vi.fn(),
  deleteHistoryRecord: vi.fn(),
  getArchiveStatus: vi.fn(),
  listHistory: vi.fn(),
}));
vi.mock("@/services/recovery-service", () => ({
  getRecoveryCandidate: vi.fn(),
  listRecoveryCandidates: vi.fn(),
  recoverTransfer: vi.fn(),
}));
vi.mock("@/services/transfer-service", () => ({
  cancelTransfer: vi.fn(),
  clearFinishedTransfers: vi.fn(),
  getTransfer: vi.fn(),
  listTransfers: vi.fn(async () => []),
  pauseTransfer: vi.fn(),
  planTransfer: vi.fn(),
  removeTransfer: vi.fn(),
  resumeTransfer: vi.fn(),
  startTransfer: vi.fn(),
  subscribeToTransferUpdates: vi.fn(async () => () => {}),
}));

const mockedListDrives = vi.mocked(drivesService.listDrives);
const mockedListAncestors = vi.mocked(filesystemService.listAncestors);
const mockedListDirectory = vi.mocked(filesystemService.listDirectory);
const mockedPickDirectory = vi.mocked(filesystemService.pickDirectory);
const mockedListTransfers = vi.mocked(transferService.listTransfers);
const mockedPlanTransfer = vi.mocked(transferService.planTransfer);
const mockedStartTransfer = vi.mocked(transferService.startTransfer);
const mockedListHistory = vi.mocked(historyService.listHistory);
const mockedArchiveStatus = vi.mocked(historyService.getArchiveStatus);
const mockedListCandidates = vi.mocked(recoveryService.listRecoveryCandidates);
const mockedSubscribeToClose = vi.mocked(appService.subscribeToCloseRequests);

const MEDIA = makeVolume({
  id: "D:",
  root: "D:\\",
  label: "MEDIA",
  name: "MEDIA",
  kind: "removable",
});

const MEDIA_ROOT = makeListing({
  path: "D:\\",
  name: "D:\\",
  parent: null,
  entries: [
    makeEntry({ name: "readme.txt", path: "D:\\readme.txt", sizeBytes: 2048 }),
  ],
});

/** The application as the shell builds it: pages, routes, and the toasts. */
function Harness() {
  useTransferNotifications();
  useRecoveryFeed();
  const location = useLocation();

  return (
    <>
      <span data-testid="location">{location.pathname}</span>
      {/* The way between pages, as the sidebar provides it in the shell. */}
      <nav aria-label="Journey">
        <NavLink to="/drives">Journey drives</NavLink>
        <NavLink to="/transfers">Journey transfers</NavLink>
        <NavLink to="/history">Journey history</NavLink>
        <NavLink to="/recovery">Journey recovery</NavLink>
      </nav>
      <Routes>
        <Route path="/drives" element={<DrivesPage />} />
        <Route path="/transfers" element={<TransfersPage />} />
        <Route path="/history" element={<HistoryPage />} />
        <Route path="/recovery" element={<RecoveryPage />} />
      </Routes>
      <NotificationStack />
    </>
  );
}

function renderJourney() {
  return render(
    <MemoryRouter initialEntries={["/drives"]}>
      <Harness />
    </MemoryRouter>,
  );
}

/** The snapshot the engine reports once a job is accepted. */
const QUEUED = makeTransferSnapshot({
  id: "transfer-1",
  status: "queued",
  sources: ["D:\\readme.txt"],
  destination: "D:\\Backup",
  progress: {
    ...makeTransferSnapshot().progress,
    totalFiles: 1,
    totalDirectories: 0,
  },
});

describe("journeys", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useNotificationStore.setState({ notifications: [] });
    useRecoveryStore.setState({
      candidates: [],
      documentStatus: null,
      writable: true,
      status: "idle",
      error: null,
      pending: [],
      failures: {},
      reports: [],
    });
    useTransferStore.setState({
      jobs: [],
      status: "ready",
      error: null,
      pending: [],
      failures: {},
      dismissed: [],
    });
    mockedSubscribeToClose.mockResolvedValue(() => {});
    mockedListCandidates.mockResolvedValue({
      candidates: [],
      status: { state: "loaded", detail: null },
      writable: true,
    });
    mockedListHistory.mockResolvedValue({
      records: [],
      total: 0,
      limit: 200,
      filter: "all",
      status: { state: "loaded", detail: null },
      writable: true,
    });
    mockedArchiveStatus.mockResolvedValue({
      history: { state: "loaded", detail: null },
      state: { state: "missing", detail: null },
      degraded: false,
      writable: true,
      historyRecords: 0,
      historyLimit: 200,
      interruptedJobs: 0,
    });
    mockedListDrives.mockResolvedValue([MEDIA]);
    mockedListDirectory.mockResolvedValue(MEDIA_ROOT);
    mockedListAncestors.mockImplementation(async (path: string) => [
      { path, label: path },
    ]);
  });

  it("carries a copy from browsing to history, and reports what was verified", async () => {
    mockedPickDirectory.mockResolvedValue("D:\\Backup");
    mockedPlanTransfer.mockResolvedValue(
      makeTransferPreview({
        sources: ["D:\\readme.txt"],
        roots: [
          {
            source: "D:\\readme.txt",
            destination: "D:\\Backup\\readme.txt",
            kind: "file",
            skipped: false,
            files: 1,
            directories: 0,
            bytes: 2048,
          },
        ],
        totalFiles: 1,
        totalDirectories: 0,
        totalBytes: 2048,
      }),
    );
    mockedStartTransfer.mockResolvedValue(QUEUED);
    // The queue page reads the engine when it opens: the job the backend
    // accepted is what it finds there.
    mockedListTransfers.mockResolvedValue([QUEUED]);

    renderJourney();

    // Browse: the volume is listed and the file is checked.
    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select readme.txt" }),
    );
    fireEvent.click(screen.getByRole("button", { name: /Copy to…/ }));

    // Review: the composer states the destination and the policy before anything
    // is queued, and nothing has been started yet.
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Into D:\\Backup");
    expect(dialog).toHaveTextContent("Size verification");
    expect(mockedStartTransfer).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Start copy" }));
    await waitFor(() => {
      expect(mockedStartTransfer).toHaveBeenCalledTimes(1);
    });

    // Progress: the queue holds the job the engine accepted.
    fireEvent.click(screen.getByRole("link", { name: "Journey transfers" }));
    expect(await screen.findByTestId("location")).toHaveTextContent(
      "/transfers",
    );
    expect(await screen.findByText("Waiting")).toBeInTheDocument();

    // Completion: the job finishes while the user is looking at the queue, and
    // exactly one notification says so — with the verdict, not a guess.
    const finished: TransferSnapshot = {
      ...QUEUED,
      status: "completed",
      verification: {
        ...QUEUED.verification,
        status: "verified",
        checkedFiles: 1,
        verifiedFiles: 1,
        unverifiedFiles: 0,
        verifiedBytes: 2048,
        verdict: "verified (size, 1 file, 2048 bytes)",
      },
      progress: { ...QUEUED.progress, completedFiles: 1, percent: 100 },
    };
    act(() => {
      useTransferStore.setState({ jobs: [finished] });
    });

    const notification = await screen.findByText("Transfer completed");
    expect(notification.closest("[role]")?.textContent).toContain(
      "verified (size, 1 file, 2048 bytes)",
    );

    // The record the backend kept is what history shows.
    mockedListHistory.mockResolvedValue({
      records: [
        makeHistoryRecord({
          id: "transfer-1",
          sources: ["D:\\readme.txt"],
          totalFiles: 1,
          totalDirectories: 0,
        }),
      ],
      total: 1,
      limit: 200,
      filter: "all",
      status: { state: "loaded", detail: null },
      writable: true,
    });

    fireEvent.click(screen.getByRole("button", { name: "View history" }));

    expect(await screen.findByTestId("location")).toHaveTextContent("/history");
    const list = await screen.findByRole("list");
    expect(list).toHaveTextContent("readme.txt");
    // The verdict shown is the one the record carries, not one inferred here.
    expect(list).toHaveTextContent("verified (size, 2 files, 4096 bytes)");
  });

  it("turns an interrupted job into a decision instead of a completion", async () => {
    const candidate = makeRecoveryCandidate({ canRestart: true });
    mockedListCandidates.mockResolvedValue({
      candidates: [candidate],
      status: { state: "loaded", detail: null },
      writable: true,
    });

    renderJourney();

    // Startup: the interruption is announced once, and it is not a success.
    const notification = await screen.findByText("A transfer was interrupted");
    expect(notification.closest("[role]")?.textContent).toContain(
      "The application stopped while they were running.",
    );

    fireEvent.click(screen.getByRole("button", { name: "Review recovery" }));

    expect(await screen.findByTestId("location")).toHaveTextContent(
      "/recovery",
    );
    // The prompt is gone, and the page holds the decision itself.
    expect(screen.queryByText("A transfer was interrupted")).toBeNull();
    expect(await screen.findByRole("list")).toHaveTextContent(
      "The transfer stopped after reporting 1024 of 4096 bytes.",
    );
    expect(
      screen.getByRole("button", { name: /Restart from the beginning/ }),
    ).toBeEnabled();
  });

  it("reports a destination that disappeared as a structured failure, not silence", async () => {
    mockedPickDirectory.mockRejectedValue(
      new IpcError("path_not_found", "path not found: D:\\Backup"),
    );

    renderJourney();

    fireEvent.click(await screen.findByRole("button", { name: /MEDIA/ }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select readme.txt" }),
    );
    fireEvent.click(screen.getByRole("button", { name: /Move to…/ }));

    // The real error code and message the backend produced, on the page.
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "path not found: D:\\Backup",
    );
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mockedPlanTransfer).not.toHaveBeenCalled();
  });
});
