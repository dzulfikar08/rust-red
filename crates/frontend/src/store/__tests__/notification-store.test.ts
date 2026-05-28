import { describe, it, expect, beforeEach, vi } from "vitest";
import { useNotificationStore } from "../notification-store";

function resetStore() {
  useNotificationStore.setState({ notifications: [] });
}

describe("useNotificationStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with an empty notification list", () => {
      expect(useNotificationStore.getState().notifications).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // addNotification
  // -----------------------------------------------------------------------

  describe("addNotification()", () => {
    it("adds a notification and returns its id", () => {
      const id = useNotificationStore.getState().addNotification({
        type: "info",
        title: "Hello",
        timeout: 5000,
        dismissible: true,
      });

      expect(id).toBeTruthy();
      const { notifications } = useNotificationStore.getState();
      expect(notifications).toHaveLength(1);
      expect(notifications[0].id).toBe(id);
      expect(notifications[0].title).toBe("Hello");
      expect(notifications[0].type).toBe("info");
      expect(notifications[0].timestamp).toBeTypeOf("number");
    });

    it("preserves optional fields", () => {
      const onClick = vi.fn();
      const id = useNotificationStore.getState().addNotification({
        type: "error",
        title: "Oops",
        message: "Something went wrong",
        timeout: 0,
        dismissible: false,
        actions: [{ label: "Retry", onClick }],
      });

      const n = useNotificationStore.getState().notifications.find((x) => x.id === id)!;
      expect(n.message).toBe("Something went wrong");
      expect(n.timeout).toBe(0);
      expect(n.dismissible).toBe(false);
      expect(n.actions).toHaveLength(1);
    });

    it("appends multiple notifications in order", () => {
      useNotificationStore.getState().addNotification({ type: "info", title: "First", timeout: 5000, dismissible: true });
      useNotificationStore.getState().addNotification({ type: "success", title: "Second", timeout: 5000, dismissible: true });

      const { notifications } = useNotificationStore.getState();
      expect(notifications).toHaveLength(2);
      expect(notifications[0].title).toBe("First");
      expect(notifications[1].title).toBe("Second");
    });
  });

  // -----------------------------------------------------------------------
  // removeNotification
  // -----------------------------------------------------------------------

  describe("removeNotification()", () => {
    it("removes a notification by id", () => {
      const id = useNotificationStore.getState().addNotification({
        type: "info", title: "Bye", timeout: 5000, dismissible: true,
      });
      useNotificationStore.getState().removeNotification(id);
      expect(useNotificationStore.getState().notifications).toHaveLength(0);
    });

    it("does nothing for an unknown id", () => {
      useNotificationStore.getState().addNotification({ type: "info", title: "A", timeout: 5000, dismissible: true });
      useNotificationStore.getState().removeNotification("nonexistent");
      expect(useNotificationStore.getState().notifications).toHaveLength(1);
    });

    it("removes only the targeted notification", () => {
      const id1 = useNotificationStore.getState().addNotification({ type: "info", title: "A", timeout: 5000, dismissible: true });
      const id2 = useNotificationStore.getState().addNotification({ type: "info", title: "B", timeout: 5000, dismissible: true });
      useNotificationStore.getState().removeNotification(id1);
      const remaining = useNotificationStore.getState().notifications;
      expect(remaining).toHaveLength(1);
      expect(remaining[0].id).toBe(id2);
    });
  });

  // -----------------------------------------------------------------------
  // clearAll
  // -----------------------------------------------------------------------

  describe("clearAll()", () => {
    it("removes all notifications", () => {
      useNotificationStore.getState().addNotification({ type: "info", title: "A", timeout: 5000, dismissible: true });
      useNotificationStore.getState().addNotification({ type: "info", title: "B", timeout: 5000, dismissible: true });
      useNotificationStore.getState().clearAll();
      expect(useNotificationStore.getState().notifications).toHaveLength(0);
    });

    it("works on an already-empty list", () => {
      useNotificationStore.getState().clearAll();
      expect(useNotificationStore.getState().notifications).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Convenience helpers
  // -----------------------------------------------------------------------

  describe("convenience helpers", () => {
    it.each([
      ["success", "success"],
      ["warning", "warning"],
      ["error", "error"],
      ["info", "info"],
    ] as const)(".%s() creates a notification of that type", (method, type) => {
      const id = useNotificationStore.getState()[method]("Title");
      const n = useNotificationStore.getState().notifications.find((x) => x.id === id)!;
      expect(n.type).toBe(type);
      expect(n.title).toBe("Title");
      expect(n.timeout).toBe(5000);
      expect(n.dismissible).toBe(true);
    });

    it("passes message through", () => {
      useNotificationStore.getState().success("OK", "All done");
      const n = useNotificationStore.getState().notifications[0];
      expect(n.message).toBe("All done");
    });

    it("works without message", () => {
      useNotificationStore.getState().info("No message needed");
      const n = useNotificationStore.getState().notifications[0];
      expect(n.message).toBeUndefined();
    });
  });
});
