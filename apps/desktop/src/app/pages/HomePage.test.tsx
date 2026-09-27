import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  useAppStore,
  useRecoveryStore,
  useSystemStore,
  useTransferStore,
} from "@/stores";
import { makeRecoveryCandidate, makeTransferSnapshot } from "@/test/fixtures";
import { renderPage } from "@/test/render-page";
import { HomePage } from "./HomePage";

describe("HomePage", () => {
  beforeEach(() => {
    useAppStore.setState({ name: "CrossPort", version: "0.1.0" });
    useSystemStore.setState({
      info: {
        platform: "windows",
        os: "windows",
        arch: "x86_64",
        family: "windows",
      },
      status: "ready",
      error: null,
    });
    useTransferStore.setState({ jobs: [] });
    useRecoveryStore.setState({ candidates: [] });
  });

  it("reports the platform the way a person writes it", () => {
    renderPage(<HomePage />);

    expect(screen.getByText("Windows · x86_64")).toBeInTheDocument();
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
  });

  it("says nothing is moving when nothing is", () => {
    renderPage(<HomePage />);

    expect(
      screen.getByText("Nothing is moving right now."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "No transfer was left unfinished by a previous session.",
      ),
    ).toBeInTheDocument();
  });

  it("counts the jobs that have not finished", () => {
    useTransferStore.setState({
      jobs: [
        makeTransferSnapshot({ id: "done", status: "completed" }),
        makeTransferSnapshot({ id: "live", status: "running" }),
      ],
    });

    renderPage(<HomePage />);

    expect(screen.getByText("1 job not finished.")).toBeInTheDocument();
  });

  it("sends the user to the queue and to recovery", () => {
    useRecoveryStore.setState({
      candidates: [
        makeRecoveryCandidate({ id: "a" }),
        makeRecoveryCandidate({ id: "b" }),
      ],
    });

    renderPage(<HomePage />);

    expect(screen.getByRole("link", { name: "Choose files" })).toHaveAttribute(
      "href",
      "/drives",
    );
    expect(
      screen.getByRole("link", { name: "Open the queue" }),
    ).toHaveAttribute("href", "/transfers");
    expect(
      screen.getByRole("link", { name: "Review recovery" }),
    ).toHaveAttribute("href", "/recovery");
    expect(
      screen.getByText("2 transfers need a decision."),
    ).toBeInTheDocument();
  });

  it("names the shortcuts the application actually implements", () => {
    renderPage(<HomePage />);

    expect(screen.getByText("Ctrl+1…6")).toBeInTheDocument();
    expect(screen.getByText("Alt+← / Alt+→")).toBeInTheDocument();
    expect(screen.getByText("Esc")).toBeInTheDocument();
  });
});
