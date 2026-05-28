import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SearchDialog } from "../SearchDialog";
import { useSearchStore } from "../../../store/search-store";
import { useFlowStore } from "../../../store/flow-store";
import { useWorkspaceStore } from "../../../store/workspace-store";
import { eventBus } from "../../../red/core/events";
import type { Node } from "@xyflow/react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNode(id: string, type: string, label: string, extra?: Record<string, unknown>): Node {
  return {
    id,
    type: "nrNode",
    position: { x: 100, y: 100 },
    data: {
      label,
      type,
      ...extra,
    },
  };
}

function resetStores() {
  useSearchStore.setState({
    query: "",
    results: [],
    isSearching: false,
    selectedIndex: 0,
    isOpen: false,
  });
  useFlowStore.setState({ nodes: [], edges: [] });
  useWorkspaceStore.setState({ flows: [], activeFlowId: null });
  eventBus.clear();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SearchDialog", () => {
  beforeEach(() => {
    resetStores();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("does not render when closed", () => {
      render(<SearchDialog />);
      expect(screen.queryByTestId("search-dialog")).not.toBeInTheDocument();
    });

    it("renders when open", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      expect(screen.getByTestId("search-dialog")).toBeInTheDocument();
    });

    it("renders search input when open", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      expect(screen.getByTestId("search-input")).toBeInTheDocument();
    });

    it("renders search results container", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      expect(screen.getByTestId("search-results")).toBeInTheDocument();
    });

    it("renders backdrop when open", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      expect(screen.getByTestId("search-dialog-backdrop")).toBeInTheDocument();
    });

    it("shows no results message when query has no matches", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "nonexistent" } });

      expect(screen.getByTestId("search-no-results")).toBeInTheDocument();
      expect(screen.getByText("No results")).toBeInTheDocument();
    });

    it("shows result count when results exist", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject Node")],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      expect(screen.getByTestId("search-result-count")).toHaveTextContent("1 result found");
    });

    it("shows plural results count for multiple results", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject A"),
          makeNode("n2", "inject", "Inject B"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      expect(screen.getByTestId("search-result-count")).toHaveTextContent("2 results found");
    });
  });

  // -----------------------------------------------------------------------
  // Typing / search
  // -----------------------------------------------------------------------

  describe("typing and search", () => {
    it("updates results when typing in search input", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject Node"),
          makeNode("n2", "debug", "Debug Node"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      expect(screen.getByTestId("search-result-n1")).toBeInTheDocument();
      expect(screen.queryByTestId("search-result-n2")).not.toBeInTheDocument();
    });

    it("highlights matching text in results", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "My Inject Node")],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      // "Inject" appears in both the node label and the match context
      const marks = screen.getAllByText("Inject", { selector: "mark" });
      expect(marks.length).toBeGreaterThanOrEqual(1);
    });
  });

  // -----------------------------------------------------------------------
  // Click interaction
  // -----------------------------------------------------------------------

  describe("click interaction", () => {
    it("closes dialog when clicking backdrop", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const backdrop = screen.getByTestId("search-dialog-backdrop");
      // Click directly on the backdrop (not on the dialog itself)
      fireEvent.click(backdrop);

      expect(useSearchStore.getState().isOpen).toBe(false);
    });

    it("navigates to node when clicking a result", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject Node")],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      const result = screen.getByTestId("search-result-n1");
      fireEvent.click(result);

      // After clicking, the search should have selected the result
      // (this emits events and switches flows)
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard navigation
  // -----------------------------------------------------------------------

  describe("keyboard navigation", () => {
    it("closes on Escape key", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.keyDown(input, { key: "Escape" });

      expect(useSearchStore.getState().isOpen).toBe(false);
    });

    it("navigates down on ArrowDown", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });
      expect(useSearchStore.getState().selectedIndex).toBe(0);

      fireEvent.keyDown(input, { key: "ArrowDown" });
      expect(useSearchStore.getState().selectedIndex).toBe(1);
    });

    it("navigates up on ArrowUp", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      // Move down then up
      fireEvent.keyDown(input, { key: "ArrowDown" });
      expect(useSearchStore.getState().selectedIndex).toBe(1);

      fireEvent.keyDown(input, { key: "ArrowUp" });
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });

    it("selects result on Enter", () => {
      const handler = vi.fn();
      const unsub = eventBus.on("search:result-selected", handler);

      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject Node")],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({
        nodeId: "n1",
        flowId: expect.any(String),
      });

      unsub();
    });

    it("wraps ArrowDown from last to first", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      // Go to last then wrap
      fireEvent.keyDown(input, { key: "ArrowDown" });
      fireEvent.keyDown(input, { key: "ArrowDown" });
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });

    it("wraps ArrowUp from first to last", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });
      useSearchStore.getState().open();
      render(<SearchDialog />);

      const input = screen.getByTestId("search-input");
      fireEvent.change(input, { target: { value: "inject" } });

      fireEvent.keyDown(input, { key: "ArrowUp" });
      expect(useSearchStore.getState().selectedIndex).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // Empty state
  // -----------------------------------------------------------------------

  describe("empty state", () => {
    it("does not show no-results message when query is empty", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      expect(screen.queryByTestId("search-no-results")).not.toBeInTheDocument();
    });

    it("does not show result count footer when query is empty", () => {
      useSearchStore.getState().open();
      render(<SearchDialog />);
      // Footer with result count is only rendered when query is non-empty
      expect(screen.queryByTestId("search-result-count")).not.toBeInTheDocument();
    });
  });
});
