import { fireEvent, screen } from "@testing-library/react";
import { useLocation } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { renderPage } from "@/test/render-page";
import { useAppShortcuts } from "./useAppShortcuts";

/** A page that installs the shortcuts the way the shell does. */
function ShortcutPage() {
  useAppShortcuts();
  const location = useLocation();
  return (
    <>
      <span data-testid="location">{location.pathname}</span>
      <input aria-label="A field the user is typing in" />
    </>
  );
}

function renderShortcuts() {
  return renderPage(<ShortcutPage />);
}

describe("useAppShortcuts", () => {
  it("starts where the router started", () => {
    renderShortcuts();

    expect(screen.getByTestId("location")).toHaveTextContent("/");
  });

  it("moves between the pages in the navigation table", () => {
    renderShortcuts();

    fireEvent.keyDown(document, { key: "3", ctrlKey: true });
    expect(screen.getByTestId("location")).toHaveTextContent("/transfers");

    fireEvent.keyDown(document, { key: "5", metaKey: true });
    expect(screen.getByTestId("location")).toHaveTextContent("/recovery");
  });

  it("leaves a keystroke the user is typing alone", () => {
    renderShortcuts();

    const field = screen.getByLabelText("A field the user is typing in");
    field.focus();
    fireEvent.keyDown(field, { key: "2", ctrlKey: true });

    // Ctrl+2 in a text field is the user's business, not the application's.
    expect(screen.getByTestId("location")).toHaveTextContent("/");
  });

  it("ignores combinations it does not own", () => {
    renderShortcuts();

    fireEvent.keyDown(document, { key: "3", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(document, { key: "3", ctrlKey: true, altKey: true });
    fireEvent.keyDown(document, { key: "3", altKey: true });
    fireEvent.keyDown(document, { key: "9", ctrlKey: true });

    expect(screen.getByTestId("location")).toHaveTextContent("/");
  });

  it("stops listening when the shell goes away", () => {
    const { unmount } = renderPage(<ShortcutPage />);
    unmount();

    // A removed listener cannot navigate a tree that no longer exists.
    expect(() => {
      fireEvent.keyDown(document, { key: "4", ctrlKey: true });
    }).not.toThrow();
  });
});
