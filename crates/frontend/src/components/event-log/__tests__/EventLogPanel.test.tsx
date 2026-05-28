import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { EventLogPanel } from "../EventLogPanel";
import { useEventLogStore } from "../../../store/event-log-store";

function resetStore() {
  useEventLogStore.setState({
    entries: [],
    filter: "",
    maxEntries: 200,
  });
}

describe("EventLogPanel", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the event log panel", () => {
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-panel")).toBeInTheDocument();
    });

    it("shows empty state when no events", () => {
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-empty")).toHaveTextContent(
        "No events logged",
      );
    });

    it("renders the filter input", () => {
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-filter-input")).toBeInTheDocument();
    });

    it("renders the clear button", () => {
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-clear-btn")).toBeInTheDocument();
    });

    it("renders entries when present", () => {
      useEventLogStore.getState().addEntry({
        type: "nodes:added",
        message: "Added a debug node",
        level: "info",
      });
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-entry")).toBeInTheDocument();
      expect(screen.getByTestId("event-log-type")).toHaveTextContent(
        "nodes:added",
      );
      expect(screen.getByTestId("event-log-message")).toHaveTextContent(
        "Added a debug node",
      );
    });

    it("renders timestamp for each entry", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "Test entry",
        level: "info",
      });
      render(<EventLogPanel />);
      const ts = screen.getByTestId("event-log-timestamp");
      expect(ts).toBeInTheDocument();
      expect(ts.textContent).toBeTruthy();
    });

    it("renders entries in reverse chronological order (newest first)", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "Old event",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "New event",
        level: "info",
      });
      render(<EventLogPanel />);
      const entries = screen.getAllByTestId("event-log-entry");
      expect(entries).toHaveLength(2);
      // Newest should appear first
      expect(entries[0]).toHaveTextContent("New event");
      expect(entries[1]).toHaveTextContent("Old event");
    });

    it("renders multiple entries", () => {
      for (let i = 0; i < 5; i++) {
        useEventLogStore.getState().addEntry({
          type: "test",
          message: `Event ${i}`,
          level: "info",
        });
      }
      render(<EventLogPanel />);
      expect(screen.getAllByTestId("event-log-entry")).toHaveLength(5);
    });
  });

  // -----------------------------------------------------------------------
  // Level styling
  // -----------------------------------------------------------------------

  describe("level styling", () => {
    it("renders info level with correct icon", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "Info event",
        level: "info",
      });
      render(<EventLogPanel />);
      const icon = screen.getByTestId("event-log-level-icon");
      // SVG elements use getAttribute("class") since className is SVGAnimatedString
      expect(icon.getAttribute("class")).toContain("text-blue");
    });

    it("renders warn level with correct styling", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "Warn event",
        level: "warn",
      });
      render(<EventLogPanel />);
      const icon = screen.getByTestId("event-log-level-icon");
      expect(icon.getAttribute("class")).toContain("text-amber");
    });

    it("renders error level with correct styling", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "Error event",
        level: "error",
      });
      render(<EventLogPanel />);
      const icon = screen.getByTestId("event-log-level-icon");
      expect(icon.getAttribute("class")).toContain("text-red");
    });
  });

  // -----------------------------------------------------------------------
  // Filter
  // -----------------------------------------------------------------------

  describe("filter", () => {
    it("filters entries by typing in filter input", async () => {
      const user = userEvent.setup();
      useEventLogStore.getState().addEntry({
        type: "nodes:added",
        message: "Added debug node",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "deploy:success",
        message: "Deployed successfully",
        level: "info",
      });
      render(<EventLogPanel />);

      expect(screen.getAllByTestId("event-log-entry")).toHaveLength(2);

      await user.type(screen.getByTestId("event-log-filter-input"), "deploy");

      expect(screen.getAllByTestId("event-log-entry")).toHaveLength(1);
      expect(screen.getByTestId("event-log-entry")).toHaveTextContent(
        "Deployed successfully",
      );
    });
  });

  // -----------------------------------------------------------------------
  // Clear
  // -----------------------------------------------------------------------

  describe("clear", () => {
    it("clears all entries when clear button clicked", async () => {
      const user = userEvent.setup();
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "A",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "B",
        level: "info",
      });
      render(<EventLogPanel />);

      expect(screen.getAllByTestId("event-log-entry")).toHaveLength(2);

      await user.click(screen.getByTestId("event-log-clear-btn"));

      expect(screen.getByTestId("event-log-empty")).toBeInTheDocument();
      expect(useEventLogStore.getState().entries).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Count display
  // -----------------------------------------------------------------------

  describe("count display", () => {
    it("shows entry count", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "A",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "B",
        level: "info",
      });
      render(<EventLogPanel />);
      expect(screen.getByTestId("event-log-count")).toHaveTextContent("2");
    });
  });
});
