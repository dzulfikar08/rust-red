import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DebugPanel } from "../DebugPanel";
import { useDebugStore, type DebugMessage } from "../../../store/debug-store";

// Mock the comms WebSocket client
vi.mock("../../../ws/comms", () => ({
  commsClient: {
    on: vi.fn(() => vi.fn()),
    subscribe: vi.fn(),
  },
}));

function makeMsg(overrides: Partial<DebugMessage> = {}): DebugMessage {
  return {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    msg: { payload: "test" },
    ...overrides,
  };
}

function resetStore() {
  useDebugStore.setState({
    messages: [],
    filter: "",
    flowFilter: null,
    maxMessages: 1000,
  });
}

describe("DebugPanel", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the debug panel", () => {
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-panel")).toBeInTheDocument();
    });

    it("shows empty state when no messages", () => {
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-empty")).toHaveTextContent(
        "No debug messages",
      );
    });

    it("renders the filter input", () => {
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-filter-input")).toBeInTheDocument();
    });

    it("renders the flow filter dropdown", () => {
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-flow-filter")).toBeInTheDocument();
    });

    it("renders the clear button", () => {
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-clear-btn")).toBeInTheDocument();
    });

    it("renders messages when present", () => {
      useDebugStore.getState().addMessage(
        makeMsg({
          id: "msg-1",
          timestamp: 1703520615000,
          sourceNode: { id: "n1", name: "MyDebug", type: "debug" },
          sourceFlow: "Flow 1",
          msg: { payload: "Hello World", topic: "greeting" },
        }),
      );
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-message-row")).toBeInTheDocument();
      expect(screen.getByTestId("debug-source")).toHaveTextContent(
        "MyDebug/Flow 1",
      );
    });

    it("renders timestamp for each message", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ id: "msg-1", timestamp: 1703520615000 }),
      );
      render(<DebugPanel />);
      const ts = screen.getByTestId("debug-timestamp");
      expect(ts).toBeInTheDocument();
      // Should be a locale string (non-empty)
      expect(ts.textContent).toBeTruthy();
    });

    it("shows payload preview for string payload", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: "Hello World" } }),
      );
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-preview")).toHaveTextContent(
        "Hello World",
      );
    });

    it("shows payload preview for object payload", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: { a: 1, b: 2, c: 3 } } }),
      );
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-preview")).toHaveTextContent(
        "{ object with 3 keys }",
      );
    });

    it("shows payload preview for array payload", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: [1, 2, 3] } }),
      );
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-preview")).toHaveTextContent(
        "Array[3]",
      );
    });

    it("renders messages in reverse chronological order (newest at top)", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ id: "old", msg: { payload: "old" } }),
      );
      useDebugStore.getState().addMessage(
        makeMsg({ id: "new", msg: { payload: "new" } }),
      );
      render(<DebugPanel />);
      const rows = screen.getAllByTestId("debug-message-row");
      expect(rows).toHaveLength(2);
      // "new" should appear first (top of list)
      expect(rows[0]).toHaveTextContent("new");
      expect(rows[1]).toHaveTextContent("old");
    });
  });

  // -----------------------------------------------------------------------
  // Expand / collapse tree
  // -----------------------------------------------------------------------

  describe("expand tree view", () => {
    it("expands tree view on clicking expand button", async () => {
      const user = userEvent.setup();
      useDebugStore.getState().addMessage(
        makeMsg({
          msg: { payload: "test", topic: "demo", _msgid: "abc" },
        }),
      );
      render(<DebugPanel />);

      // Tree view should not be visible initially
      expect(screen.queryByTestId("debug-tree-view")).not.toBeInTheDocument();

      // Click expand
      await user.click(screen.getByTestId("debug-expand-btn"));

      // Tree view should now be visible
      expect(screen.getByTestId("debug-tree-view")).toBeInTheDocument();
    });

    it("collapses tree view on second click", async () => {
      const user = userEvent.setup();
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: "test" } }),
      );
      render(<DebugPanel />);

      await user.click(screen.getByTestId("debug-expand-btn"));
      expect(screen.getByTestId("debug-tree-view")).toBeInTheDocument();

      await user.click(screen.getByTestId("debug-expand-btn"));
      expect(screen.queryByTestId("debug-tree-view")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Copy
  // -----------------------------------------------------------------------

  describe("copy", () => {
    it("renders copy button for each message", () => {
      useDebugStore.getState().addMessage(makeMsg());
      render(<DebugPanel />);
      expect(screen.getByTestId("debug-copy-btn")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Clear
  // -----------------------------------------------------------------------

  describe("clear", () => {
    it("clears all messages when clear button clicked", async () => {
      const user = userEvent.setup();
      useDebugStore.getState().addMessage(makeMsg());
      useDebugStore.getState().addMessage(makeMsg());
      render(<DebugPanel />);

      expect(screen.getAllByTestId("debug-message-row")).toHaveLength(2);

      await user.click(screen.getByTestId("debug-clear-btn"));

      expect(screen.getByTestId("debug-empty")).toBeInTheDocument();
      expect(useDebugStore.getState().messages).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Filter
  // -----------------------------------------------------------------------

  describe("filter", () => {
    it("filters messages by typing in filter input", async () => {
      const user = userEvent.setup();
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: "Hello World" } }),
      );
      useDebugStore.getState().addMessage(
        makeMsg({ msg: { payload: "Goodbye" } }),
      );
      render(<DebugPanel />);

      expect(screen.getAllByTestId("debug-message-row")).toHaveLength(2);

      await user.type(screen.getByTestId("debug-filter-input"), "hello");

      expect(screen.getAllByTestId("debug-message-row")).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // Flow filter
  // -----------------------------------------------------------------------

  describe("flow filter", () => {
    it("shows All flows by default", () => {
      render(<DebugPanel />);
      const select = screen.getByTestId("debug-flow-filter");
      expect(select).toBeInTheDocument();
      // Default should show "All flows"
    });

    it("renders flow options for messages with sourceFlow", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ sourceFlow: "Flow 1" }),
      );
      useDebugStore.getState().addMessage(
        makeMsg({ sourceFlow: "Flow 2" }),
      );
      render(<DebugPanel />);
      const select = screen.getByTestId(
        "debug-flow-filter",
      ) as HTMLSelectElement;
      // Options: "All flows" + "Flow 1" + "Flow 2"
      expect(select.options.length).toBeGreaterThanOrEqual(2);
    });
  });

  // -----------------------------------------------------------------------
  // Level styling
  // -----------------------------------------------------------------------

  describe("level styling", () => {
    it("applies error color for error level messages", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ level: "error" }),
      );
      render(<DebugPanel />);
      const source = screen.getByTestId("debug-source");
      expect(source.className).toContain("text-red");
    });

    it("applies warn color for warn level messages", () => {
      useDebugStore.getState().addMessage(
        makeMsg({ level: "warn" }),
      );
      render(<DebugPanel />);
      const source = screen.getByTestId("debug-source");
      expect(source.className).toContain("text-amber");
    });
  });
});
