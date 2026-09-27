import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import * as service from "@/services/recovery-service";
import { useRecoveryStore } from "@/stores";
import { makeRecoveryCandidate } from "@/test/fixtures";
import { RecoveryPage } from "./RecoveryPage";

vi.mock("@/services/recovery-service", () => ({
  listRecoveryCandidates: vi.fn(),
  getRecoveryCandidate: vi.fn(),
  recoverTransfer: vi.fn(),
}));

const mockedList = vi.mocked(service.listRecoveryCandidates);
const mockedRecover = vi.mocked(service.recoverTransfer);

function listing(candidates = [makeRecoveryCandidate()]) {
  return {
    candidates,
    status: { state: "loaded" as const, detail: null },
    writable: true,
  };
}

describe("RecoveryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
  });

  it("shows nothing interrupted as a clear state rather than an empty list", async () => {
    mockedList.mockResolvedValue(listing([]));

    render(<RecoveryPage />);

    expect(
      await screen.findByText("Nothing needs recovery"),
    ).toBeInTheDocument();
  });

  it("describes what was interrupted and what can be done about it", async () => {
    mockedList.mockResolvedValue(listing());

    render(<RecoveryPage />);

    expect(
      await screen.findByText(/Interrupted — can be run again/),
    ).toBeInTheDocument();
    expect(screen.getByText(/1 unfinished file\(s\)/)).toBeInTheDocument();
    expect(
      screen.getByText(/Every planned item|Nothing exists at the destination/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Restart from the beginning/ }),
    ).toBeEnabled();
  });

  it("restarts a candidate and reloads the list", async () => {
    const candidate = makeRecoveryCandidate();
    mockedList.mockResolvedValue(listing([candidate]));
    mockedRecover.mockResolvedValue({
      candidate,
      action: "restart",
      restartedAs: "transfer-1700000000000-9",
      artifactsRemoved: 1,
    });

    render(<RecoveryPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Restart from the beginning/ }),
    );

    await waitFor(() =>
      expect(mockedRecover).toHaveBeenCalledWith(candidate.id, "restart"),
    );
  });

  it("discards partial files without running anything again", async () => {
    const candidate = makeRecoveryCandidate();
    mockedList.mockResolvedValue(listing([candidate]));
    mockedRecover.mockResolvedValue({
      candidate,
      action: "discard",
      restartedAs: null,
      artifactsRemoved: 1,
    });

    render(<RecoveryPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Discard partial files/ }),
    );

    await waitFor(() =>
      expect(mockedRecover).toHaveBeenCalledWith(candidate.id, "discard"),
    );
  });

  it("disables a restart the backend refused to allow", async () => {
    mockedList.mockResolvedValue(
      listing([
        makeRecoveryCandidate({
          outcome: "source_missing",
          canRestart: false,
          detail: "At least one of the 1 source(s) is no longer there.",
        }),
      ]),
    );

    render(<RecoveryPage />);

    const restart = await screen.findByRole("button", {
      name: /Restart from the beginning/,
    });
    expect(restart).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /Discard partial files/ }),
    ).toBeEnabled();
  });

  it("only enables confirmation when the archive proved the job finished", async () => {
    mockedList.mockResolvedValue(
      listing([
        makeRecoveryCandidate({
          outcome: "completed_before_crash",
          confirmedByArchive: true,
          canRestart: false,
        }),
      ]),
    );

    render(<RecoveryPage />);

    const confirm = await screen.findByRole("button", {
      name: /Mark as recovered/,
    });
    expect(confirm).toBeEnabled();
    expect(
      screen.getByText(
        /archive recorded this transfer reaching a terminal state/,
      ),
    ).toBeInTheDocument();
  });

  it("shows a refusal where the user is looking instead of losing it", async () => {
    const candidate = makeRecoveryCandidate();
    mockedList.mockResolvedValue(listing([candidate]));
    mockedRecover.mockRejectedValue(
      new IpcError("recovery_unavailable", "the destination is gone"),
    );

    render(<RecoveryPage />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Restart from the beginning/ }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "the destination is gone",
    );
  });

  it("explains a state document that could not be used", async () => {
    mockedList.mockResolvedValue({
      candidates: [],
      status: {
        state: "recovered",
        detail: "the file is not valid JSON",
      },
      writable: true,
    });

    render(<RecoveryPage />);

    expect(
      await screen.findByText("Unusable and set aside"),
    ).toBeInTheDocument();
    expect(screen.getByText(/the file is not valid JSON/)).toBeInTheDocument();
  });

  it("explains a newer state document and never offers to overwrite it", async () => {
    mockedList.mockResolvedValue({
      candidates: [],
      status: {
        state: "unsupported",
        detail: "the file uses schema version 99",
      },
      writable: false,
    });

    render(<RecoveryPage />);

    expect(
      await screen.findByText("Written by a newer version"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/will not overwrite a newer document/),
    ).toBeInTheDocument();
  });

  it("reports a listing failure with a retry", async () => {
    mockedList.mockRejectedValue(new IpcError("unavailable", "desktop only"));

    render(<RecoveryPage />);

    expect(
      await screen.findByText("Interrupted transfers could not be listed"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Try again" }),
    ).toBeInTheDocument();
  });
});
