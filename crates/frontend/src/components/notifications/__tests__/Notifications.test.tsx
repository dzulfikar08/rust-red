import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Notifications } from "../Notifications";
import { useNotificationStore } from "../../../store/notification-store";

function resetStore() {
  useNotificationStore.setState({ notifications: [] });
}

describe("Notifications", () => {
  beforeEach(() => {
    resetStore();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders nothing when there are no notifications", () => {
      const { container } = render(<Notifications />);
      expect(container.innerHTML).toBe("");
    });

    it("renders a notification with title", () => {
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Test notification",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);
      expect(screen.getByText("Test notification")).toBeInTheDocument();
    });

    it("renders a notification with title and message", () => {
      useNotificationStore.getState().addNotification({
        type: "success",
        title: "Deployed",
        message: "Flow deployed successfully",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);
      expect(screen.getByText("Deployed")).toBeInTheDocument();
      expect(screen.getByText("Flow deployed successfully")).toBeInTheDocument();
    });

    it("renders action buttons when provided", () => {
      const onClick = vi.fn();
      useNotificationStore.getState().addNotification({
        type: "error",
        title: "Error",
        timeout: 0,
        dismissible: true,
        actions: [{ label: "Retry", onClick }],
      });
      render(<Notifications />);
      const btn = screen.getByText("Retry");
      expect(btn).toBeInTheDocument();
      fireEvent.click(btn);
      expect(onClick).toHaveBeenCalledOnce();
    });

    it("renders all four notification types", () => {
      const types: Array<["success" | "warning" | "error" | "info", string]> = [
        ["success", "Success msg"],
        ["warning", "Warning msg"],
        ["error", "Error msg"],
        ["info", "Info msg"],
      ];
      for (const [type, title] of types) {
        useNotificationStore.getState().addNotification({
          type,
          title,
          timeout: 0,
          dismissible: true,
        });
      }
      render(<Notifications />);
      for (const [, title] of types) {
        expect(screen.getByText(title)).toBeInTheDocument();
      }
    });
  });

  // -----------------------------------------------------------------------
  // Close button
  // -----------------------------------------------------------------------

  describe("close button", () => {
    it("shows close button when dismissible", () => {
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Dismissible",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);
      expect(screen.getByLabelText("Dismiss notification")).toBeInTheDocument();
    });

    it("does not show close button when not dismissible", () => {
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Persistent",
        timeout: 0,
        dismissible: false,
      });
      render(<Notifications />);
      expect(screen.queryByLabelText("Dismiss notification")).not.toBeInTheDocument();
    });

    it("removes notification on close button click", async () => {
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Close me",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);
      const btn = screen.getByLabelText("Dismiss notification");
      fireEvent.click(btn);

      // After the slide-out animation (250ms), the notification is removed
      await waitFor(() => {
        expect(useNotificationStore.getState().notifications).toHaveLength(0);
      });
    });
  });

  // -----------------------------------------------------------------------
  // Auto-dismiss
  // -----------------------------------------------------------------------

  describe("auto-dismiss", () => {
    it("auto-dismisses after timeout", async () => {
      vi.useFakeTimers();
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Auto dismiss",
        timeout: 1000,
        dismissible: true,
      });
      render(<Notifications />);
      expect(screen.getByText("Auto dismiss")).toBeInTheDocument();

      // Advance past the auto-dismiss timeout (1000ms) + slide-out animation (250ms)
      vi.advanceTimersByTime(1300);

      // Restore real timers so waitFor can use real setTimeout internally
      vi.useRealTimers();

      await waitFor(() => {
        expect(useNotificationStore.getState().notifications).toHaveLength(0);
      });
    });

    it("does not auto-dismiss when timeout is 0", () => {
      vi.useFakeTimers();
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Persistent",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);

      vi.advanceTimersByTime(10000);

      expect(useNotificationStore.getState().notifications).toHaveLength(1);

      vi.useRealTimers();
    });
  });

  // -----------------------------------------------------------------------
  // Accessibility
  // -----------------------------------------------------------------------

  describe("accessibility", () => {
    it("each notification has role=alert", () => {
      useNotificationStore.getState().addNotification({
        type: "info",
        title: "Alert test",
        timeout: 0,
        dismissible: true,
      });
      render(<Notifications />);
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
  });
});
