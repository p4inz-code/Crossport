import { beforeEach, describe, expect, it } from "vitest";

import { useNotificationStore } from "./notification-store";

describe("notification store", () => {
  beforeEach(() => {
    useNotificationStore.setState({ notifications: [] });
  });

  it("starts empty", () => {
    expect(useNotificationStore.getState().notifications).toEqual([]);
  });

  it("shows a notification newest first", () => {
    const { notify } = useNotificationStore.getState();

    notify({ kind: "success", title: "First", message: "one", event: "a" });
    notify({ kind: "error", title: "Second", message: "two", event: "b" });

    expect(
      useNotificationStore.getState().notifications.map((n) => n.title),
    ).toEqual(["Second", "First"]);
  });

  it("does not repeat the same event for the same job", () => {
    const { notify } = useNotificationStore.getState();

    notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files",
      event: "completed",
      jobId: "transfer-1",
    });
    notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files",
      event: "completed",
      jobId: "transfer-1",
    });

    expect(useNotificationStore.getState().notifications).toHaveLength(1);
  });

  it("keeps events for different jobs apart", () => {
    const { notify } = useNotificationStore.getState();

    notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files",
      event: "completed",
      jobId: "transfer-1",
    });
    notify({
      kind: "success",
      title: "Transfer completed",
      message: "2 files",
      event: "completed",
      jobId: "transfer-2",
    });

    expect(useNotificationStore.getState().notifications).toHaveLength(2);
  });

  it("bounds the list so a long session cannot grow it forever", () => {
    const { notify } = useNotificationStore.getState();

    for (let index = 0; index < 12; index += 1) {
      notify({
        kind: "info",
        title: `Job ${index}`,
        message: "done",
        event: "completed",
        jobId: `transfer-${index}`,
      });
    }

    const notifications = useNotificationStore.getState().notifications;
    expect(notifications).toHaveLength(5);
    expect(notifications[0].jobId).toBe("transfer-11");
  });

  it("dismisses one notification without touching the rest", () => {
    const { notify } = useNotificationStore.getState();
    notify({ kind: "info", title: "One", message: "", event: "a" });
    notify({ kind: "info", title: "Two", message: "", event: "b" });

    const [newest] = useNotificationStore.getState().notifications;
    useNotificationStore.getState().dismiss(newest.id);

    const remaining = useNotificationStore.getState().notifications;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].title).toBe("One");
  });

  it("clears everything on request", () => {
    const { notify } = useNotificationStore.getState();
    notify({ kind: "info", title: "One", message: "", event: "a" });

    useNotificationStore.getState().clear();

    expect(useNotificationStore.getState().notifications).toEqual([]);
  });
});
