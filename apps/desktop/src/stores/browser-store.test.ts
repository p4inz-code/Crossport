import { beforeEach, describe, expect, it, vi } from "vitest";

import * as service from "@/services/filesystem-service";
import { IpcError } from "@/services/ipc";
import { makeListing } from "@/test/fixtures";
import { useBrowserStore } from "./browser-store";

vi.mock("@/services/filesystem-service", () => ({
  listDirectory: vi.fn(),
}));

const mockedListDirectory = vi.mocked(service.listDirectory);

const ROOT = makeListing({ path: "C:\\", name: "C:\\", parent: null });
const USERS = makeListing({
  path: "C:\\Users",
  name: "Users",
  parent: "C:\\",
});

function resetBrowser(): void {
  useBrowserStore.setState({
    location: null,
    history: [],
    listing: null,
    status: "idle",
    error: null,
  });
}

describe("browser store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetBrowser();
  });

  it("starts idle with nothing open", () => {
    const state = useBrowserStore.getState();

    expect(state.location).toBeNull();
    expect(state.listing).toBeNull();
    expect(state.status).toBe("idle");
    expect(state.history).toEqual([]);
  });

  it("opens a location and adopts the path the backend normalized", async () => {
    mockedListDirectory.mockResolvedValue(ROOT);

    await useBrowserStore.getState().open("C:/");

    expect(mockedListDirectory).toHaveBeenCalledWith("C:/");
    expect(useBrowserStore.getState().location).toBe("C:\\");
    expect(useBrowserStore.getState().listing).toEqual(ROOT);
    expect(useBrowserStore.getState().status).toBe("ready");
    expect(useBrowserStore.getState().error).toBeNull();
  });

  it("drops the previous entries while a new location loads", async () => {
    useBrowserStore.setState({
      listing: ROOT,
      location: "C:\\",
      status: "ready",
    });
    mockedListDirectory.mockResolvedValue(USERS);

    const pending = useBrowserStore.getState().open("C:\\Users");

    expect(useBrowserStore.getState().listing).toBeNull();
    expect(useBrowserStore.getState().status).toBe("loading");
    await pending;
  });

  it("navigates back to the previous location", async () => {
    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().open("C:\\");

    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().open("C:\\Users");
    expect(useBrowserStore.getState().history).toEqual(["C:\\"]);

    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().goBack();

    expect(mockedListDirectory).toHaveBeenLastCalledWith("C:\\");
    expect(useBrowserStore.getState().location).toBe("C:\\");
    expect(useBrowserStore.getState().listing).toEqual(ROOT);
    expect(useBrowserStore.getState().history).toEqual([]);
  });

  it("does nothing when there is no history to go back to", async () => {
    await useBrowserStore.getState().goBack();

    expect(mockedListDirectory).not.toHaveBeenCalled();
    expect(useBrowserStore.getState().location).toBeNull();
  });

  it("reloads the current location without touching history", async () => {
    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().open("C:\\Users");

    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().refresh();

    expect(mockedListDirectory).toHaveBeenLastCalledWith("C:\\Users");
    expect(useBrowserStore.getState().history).toEqual([]);
    expect(useBrowserStore.getState().status).toBe("ready");
  });

  it("does not call the backend when there is nothing to refresh", async () => {
    await useBrowserStore.getState().refresh();

    expect(mockedListDirectory).not.toHaveBeenCalled();
    expect(useBrowserStore.getState().status).toBe("idle");
  });

  it("goes up to the parent the backend reported", async () => {
    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().open("C:\\Users");

    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().goUp();

    expect(mockedListDirectory).toHaveBeenLastCalledWith("C:\\");
    expect(useBrowserStore.getState().location).toBe("C:\\");
    expect(useBrowserStore.getState().history).toEqual(["C:\\Users"]);
  });

  it("cannot go up from a filesystem root", async () => {
    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().open("C:\\");

    await useBrowserStore.getState().goUp();

    expect(mockedListDirectory).toHaveBeenCalledTimes(1);
    expect(useBrowserStore.getState().location).toBe("C:\\");
  });

  it("surfaces a structured error and clears stale entries", async () => {
    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().open("C:\\");

    mockedListDirectory.mockRejectedValueOnce(
      new IpcError("path_not_found", "path not found: D:\\"),
    );
    await useBrowserStore.getState().open("D:\\");

    const state = useBrowserStore.getState();
    expect(state.listing).toBeNull();
    expect(state.status).toBe("error");
    expect(state.error?.code).toBe("path_not_found");
    expect(state.error?.message).toBe("path not found: D:\\");
    expect(state.location).toBe("D:\\");
  });

  it("normalizes a non-IpcError failure", async () => {
    mockedListDirectory.mockRejectedValueOnce("boom");

    await useBrowserStore.getState().open("C:\\");

    expect(useBrowserStore.getState().error).toBeInstanceOf(IpcError);
    expect(useBrowserStore.getState().error?.message).toBe("boom");
  });

  it("recovers when the same location succeeds on retry", async () => {
    mockedListDirectory.mockRejectedValueOnce(
      new IpcError("path_not_found", "path not found: E:\\"),
    );
    await useBrowserStore.getState().open("E:\\");
    expect(useBrowserStore.getState().status).toBe("error");

    mockedListDirectory.mockResolvedValueOnce(
      makeListing({ path: "E:\\", name: "E:\\" }),
    );
    await useBrowserStore.getState().refresh();

    expect(useBrowserStore.getState().status).toBe("ready");
    expect(useBrowserStore.getState().error).toBeNull();
    expect(useBrowserStore.getState().listing?.path).toBe("E:\\");
  });

  it("never lets a slow listing overwrite a newer navigation", async () => {
    let resolveSlow: (listing: typeof ROOT) => void = () => undefined;
    const slow = new Promise<typeof ROOT>((resolve) => {
      resolveSlow = resolve;
    });

    mockedListDirectory.mockReturnValueOnce(slow);
    const slowNavigation = useBrowserStore.getState().open("C:\\");

    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().open("C:\\Users");

    resolveSlow(ROOT);
    await slowNavigation;

    const state = useBrowserStore.getState();
    expect(state.location).toBe("C:\\Users");
    expect(state.listing).toEqual(USERS);
    expect(state.status).toBe("ready");
  });

  it("abandons an in-flight listing when the browser is closed", async () => {
    let resolveSlow: (listing: typeof ROOT) => void = () => undefined;
    mockedListDirectory.mockReturnValueOnce(
      new Promise<typeof ROOT>((resolve) => {
        resolveSlow = resolve;
      }),
    );

    const pending = useBrowserStore.getState().open("C:\\");
    useBrowserStore.getState().close();
    resolveSlow(ROOT);
    await pending;

    const state = useBrowserStore.getState();
    expect(state.location).toBeNull();
    expect(state.listing).toBeNull();
    expect(state.status).toBe("idle");
  });

  it("clears everything when the user leaves the browser", async () => {
    mockedListDirectory.mockResolvedValueOnce(USERS);
    await useBrowserStore.getState().open("C:\\Users");

    useBrowserStore.getState().close();

    const state = useBrowserStore.getState();
    expect(state.location).toBeNull();
    expect(state.history).toEqual([]);
    expect(state.listing).toBeNull();
    expect(state.status).toBe("idle");
    expect(state.error).toBeNull();
  });

  it("keeps history when a navigation fails so the user can go back", async () => {
    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().open("C:\\");

    mockedListDirectory.mockRejectedValueOnce(
      new IpcError("path_not_found", "path not found: D:\\"),
    );
    await useBrowserStore.getState().open("D:\\");

    mockedListDirectory.mockResolvedValueOnce(ROOT);
    await useBrowserStore.getState().goBack();

    expect(useBrowserStore.getState().location).toBe("C:\\");
    expect(useBrowserStore.getState().status).toBe("ready");
  });

  it("does not stack a duplicate history entry when reopening the same location", async () => {
    mockedListDirectory.mockResolvedValue(ROOT);

    await useBrowserStore.getState().open("C:\\");
    await useBrowserStore.getState().open("C:\\");

    expect(useBrowserStore.getState().history).toEqual([]);
  });
});
