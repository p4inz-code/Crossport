import { fireEvent, screen } from "@testing-library/react";
import { useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";

import { useNotificationStore } from "@/stores";
import { renderPage } from "@/test/render-page";
import { NotificationStack } from "./NotificationStack";

/** Shows where the application currently is, so navigation can be asserted. */
function LocationProbe() {
  return <span data-testid="location">{useLocation().pathname}</span>;
}

function renderStack() {
  return renderPage(
    <>
      <NotificationStack />
      <LocationProbe />
    </>,
  );
}

describe("NotificationStack", () => {
  beforeEach(() => {
    useNotificationStore.setState({ notifications: [] });
  });

  it("renders nothing when there is nothing to say", () => {
    renderStack();

    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("announces each notification once, in the order it arrived", () => {
    useNotificationStore.getState().notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files — verified.",
      event: "completed",
      jobId: "transfer-1",
      to: "/history",
    });
    useNotificationStore.getState().notify({
      kind: "error",
      title: "Verification failed",
      message: "1 file — the sizes did not match.",
      event: "verification-failed",
      jobId: "transfer-2",
      to: "/transfers",
    });

    const { container } = renderStack();

    // The announcement order is the render order: the freshest thing that
    // happened is the first one read.
    const titles = [...container.querySelectorAll(".notification__title")].map(
      (node) => node.textContent,
    );
    expect(titles).toEqual(["Verification failed", "Transfer completed"]);
  });

  it("keeps a message about a transfer distinct from a verification failure", () => {
    useNotificationStore.getState().notify({
      kind: "error",
      title: "Verification failed",
      message: "1 file — the sizes did not match.",
      event: "verification-failed",
      jobId: "transfer-2",
      to: "/transfers",
    });

    renderStack();

    // A verification failure is an error, so it is announced assertively.
    expect(screen.getByRole("alert")).toHaveTextContent("Verification failed");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "the sizes did not match",
    );
  });

  it("opens the surface holding what it is about", () => {
    useNotificationStore.getState().notify({
      kind: "warning",
      title: "A transfer was interrupted",
      message: "The application stopped while it was running.",
      event: "recovery-required",
      to: "/recovery",
    });

    renderStack();
    fireEvent.click(screen.getByRole("button", { name: "Review recovery" }));

    expect(screen.getByTestId("location")).toHaveTextContent("/recovery");
    // The message has been read, so it does not stay on screen afterwards.
    expect(useNotificationStore.getState().notifications).toEqual([]);
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("offers no action for a message that is about nothing on screen", () => {
    useNotificationStore.getState().notify({
      kind: "info",
      title: "Something happened",
      message: "There is no page for this.",
      event: "elsewhere",
    });

    renderStack();

    expect(screen.queryByRole("button", { name: "Open" })).toBeNull();
    expect(
      screen.getByRole("button", { name: "Dismiss Something happened" }),
    ).toBeInTheDocument();
  });

  it("dismisses one notification without disturbing the others", () => {
    const store = useNotificationStore.getState();
    store.notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files — verified.",
      event: "completed",
      jobId: "transfer-1",
      to: "/history",
    });
    store.notify({
      kind: "info",
      title: "Transfer cancelled",
      message: "1 file — the destination was left as it was.",
      event: "cancelled",
      jobId: "transfer-2",
      to: "/history",
    });

    renderStack();
    fireEvent.click(
      screen.getByRole("button", { name: "Dismiss Transfer cancelled" }),
    );

    expect(screen.getByText("Transfer completed")).toBeInTheDocument();
    expect(screen.queryByText("Transfer cancelled")).toBeNull();
  });
});
