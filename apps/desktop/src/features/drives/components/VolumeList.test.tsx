import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { IpcError } from "@/services/ipc";
import { makeVolume } from "@/test/fixtures";
import { VolumeList } from "./VolumeList";

const GIB = 1024 ** 3;

function renderList(
  overrides: Partial<ComponentProps<typeof VolumeList>> = {},
) {
  return render(
    <VolumeList
      volumes={[]}
      status="ready"
      error={null}
      selectedId={null}
      onSelect={vi.fn()}
      onRetry={vi.fn()}
      {...overrides}
    />,
  );
}

describe("VolumeList", () => {
  it("shows the metadata the backend reported for a volume", () => {
    renderList({
      volumes: [
        makeVolume({
          label: "Windows",
          name: "Windows",
          totalBytes: 500 * GIB,
          freeBytes: 120 * GIB,
          usedBytes: 380 * GIB,
        }),
      ],
    });

    expect(screen.getByText("Windows")).toBeInTheDocument();
    expect(screen.getByText("C:\\")).toBeInTheDocument();
    expect(screen.getByText("Internal disk")).toBeInTheDocument();
    expect(screen.getByText("NTFS")).toBeInTheDocument();
    expect(screen.getByText("120 GB free of 500 GB")).toBeInTheDocument();
  });

  it("draws a usage meter only when capacity is known", () => {
    const { container, rerender } = render(
      <VolumeList
        volumes={[makeVolume({ totalBytes: 500 * GIB, usedBytes: 380 * GIB })]}
        status="ready"
        error={null}
        selectedId={null}
        onSelect={vi.fn()}
        onRetry={vi.fn()}
      />,
    );

    expect(container.querySelector(".volume-card__meter-fill")).not.toBeNull();

    rerender(
      <VolumeList
        volumes={[
          makeVolume({
            totalBytes: null,
            freeBytes: null,
            usedBytes: null,
            filesystem: null,
          }),
        ]}
        status="ready"
        error={null}
        selectedId={null}
        onSelect={vi.fn()}
        onRetry={vi.fn()}
      />,
    );

    expect(container.querySelector(".volume-card__meter-fill")).toBeNull();
    expect(screen.getByText("Capacity unavailable")).toBeInTheDocument();
  });

  it("flags volumes that are unreachable or read-only", () => {
    renderList({
      volumes: [
        makeVolume({
          id: "E:",
          root: "E:\\",
          label: "E:",
          kind: "optical",
          mounted: false,
        }),
        makeVolume({
          id: "F:",
          root: "F:\\",
          label: "Archive",
          readonly: true,
        }),
      ],
    });

    expect(screen.getByText("Not available")).toBeInTheDocument();
    expect(screen.getByText("Optical drive")).toBeInTheDocument();
    expect(screen.getByText("Read-only")).toBeInTheDocument();
  });

  it("marks the volume the current location belongs to", () => {
    renderList({
      volumes: [
        makeVolume(),
        makeVolume({
          id: "D:",
          root: "D:\\",
          label: "MEDIA",
          kind: "removable",
        }),
      ],
      selectedId: "D:",
    });

    expect(screen.getAllByRole("button")).toHaveLength(2);
    expect(screen.getByRole("button", { name: /MEDIA/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(
      screen.getByRole("button", { name: /Internal disk/ }),
    ).not.toHaveAttribute("aria-current");
  });

  it("reports the volume the user picked", () => {
    const onSelect = vi.fn();
    const volume = makeVolume({ id: "D:", root: "D:\\", label: "MEDIA" });
    renderList({ volumes: [volume], onSelect });

    fireEvent.click(screen.getByRole("button", { name: /MEDIA/ }));

    expect(onSelect).toHaveBeenCalledWith(volume);
  });

  it("shows a loading state while enumerating", () => {
    renderList({ status: "loading" });

    expect(screen.getByRole("status")).toHaveTextContent("Detecting volumes…");
  });

  it("explains an empty result", () => {
    renderList({ status: "ready", volumes: [] });

    expect(screen.getByText("No volumes detected")).toBeInTheDocument();
  });

  it("explains a failed enumeration and offers a retry", () => {
    const onRetry = vi.fn();
    renderList({
      status: "error",
      error: new IpcError("permission_denied", "permission denied: locked"),
      onRetry,
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "permission denied: locked",
    );
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
