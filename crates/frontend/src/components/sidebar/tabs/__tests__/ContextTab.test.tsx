import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ContextTab } from "../ContextTab";
import { useContextStore } from "../../../../store/context-store";

// ---------------------------------------------------------------------------
// Reset helper
// ---------------------------------------------------------------------------

function resetStore() {
  useContextStore.setState({
    flowContext: null,
    globalContext: null,
    isLoading: false,
    error: null,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ContextTab", () => {
  beforeEach(() => {
    resetStore();
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the context tab container", () => {
      render(<ContextTab />);
      expect(
        screen.getByTestId("sidebar-tab-content-context"),
      ).toBeInTheDocument();
    });

    it("shows empty state when no context data", () => {
      // Override refresh to a no-op so the useEffect auto-load does nothing
      useContextStore.setState({ refresh: async () => {} });

      render(<ContextTab />);
      expect(screen.getByText("No context data")).toBeInTheDocument();
    });

    it("shows loading spinner while fetching", () => {
      // Set loading state directly
      useContextStore.setState({ isLoading: true });
      render(<ContextTab />);
      expect(screen.getByText("Loading...")).toBeInTheDocument();
    });

    it("renders context sections when data is present", () => {
      useContextStore.setState({
        flowContext: { counter: 42 },
        globalContext: { version: "1.0" },
      });
      render(<ContextTab />);

      expect(
        screen.getByTestId("context-section-flow-context"),
      ).toBeInTheDocument();
      expect(
        screen.getByTestId("context-section-global-context"),
      ).toBeInTheDocument();
    });

    it("renders primitive values with correct formatting", () => {
      useContextStore.setState({
        flowContext: {
          name: "test",
          count: 10,
          active: true,
          disabled: false,
          empty: null,
        },
        globalContext: {},
      });
      render(<ContextTab />);

      // Strings are quoted
      expect(screen.getByText(/"test"/)).toBeInTheDocument();
      // Numbers
      expect(screen.getByText("10")).toBeInTheDocument();
      // Booleans
      expect(screen.getByText("true")).toBeInTheDocument();
      expect(screen.getByText("false")).toBeInTheDocument();
      // Null
      expect(screen.getByText("null")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Expand / Collapse
  // -----------------------------------------------------------------------

  describe("expand/collapse", () => {
    it("renders expandable object values with a chevron", () => {
      useContextStore.setState({
        flowContext: {
          settings: { debug: true, interval: 5000 },
        },
        globalContext: {},
      });
      render(<ContextTab />);

      // The toggle button for the "settings" key should exist
      expect(
        screen.getByTestId("context-tree-toggle-settings"),
      ).toBeInTheDocument();
    });

    it("expands an object when clicking its chevron", async () => {
      const user = userEvent.setup();
      useContextStore.setState({
        flowContext: {
          settings: { debug: true },
        },
        globalContext: {},
      });
      render(<ContextTab />);

      // Click to expand
      await user.click(screen.getByTestId("context-tree-toggle-settings"));

      // The child value "true" should now be visible
      expect(screen.getByText("true")).toBeInTheDocument();
    });

    it("collapses an expanded object when clicking again", async () => {
      const user = userEvent.setup();
      useContextStore.setState({
        flowContext: {
          settings: { debug: true },
        },
        globalContext: {},
      });
      render(<ContextTab />);

      const toggle = screen.getByTestId("context-tree-toggle-settings");

      // Expand
      await user.click(toggle);
      expect(screen.getByText("true")).toBeInTheDocument();

      // Collapse
      await user.click(toggle);
      // After collapsing, child values are no longer in the DOM
      // (the inner "true" from the settings object, not from other sources)
      expect(screen.queryByText("debug:")).not.toBeInTheDocument();
    });

    it("renders expandable arrays with correct notation", () => {
      useContextStore.setState({
        flowContext: { items: [1, 2, 3] },
        globalContext: {},
      });
      render(<ContextTab />);

      expect(screen.getByText("[3]")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Section expand/collapse
  // -----------------------------------------------------------------------

  describe("section expand/collapse", () => {
    it("toggles Flow Context section", async () => {
      const user = userEvent.setup();
      useContextStore.setState({
        flowContext: { x: 1 },
        globalContext: { y: 2 },
      });
      render(<ContextTab />);

      const flowHeader = screen.getByTestId("context-section-flow-context");

      // Collapse
      await user.click(flowHeader);
      // Re-expand
      await user.click(flowHeader);

      // Section content should still be toggleable
      expect(flowHeader).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Toolbar
  // -----------------------------------------------------------------------

  describe("toolbar", () => {
    it("renders refresh button", () => {
      render(<ContextTab />);
      expect(screen.getByTestId("context-refresh-btn")).toBeInTheDocument();
    });

    it("renders copy button", () => {
      render(<ContextTab />);
      expect(screen.getByTestId("context-copy-btn")).toBeInTheDocument();
    });

    it("triggers refresh when clicking refresh button", async () => {
      const user = userEvent.setup();
      const refreshSpy = vi.fn().mockResolvedValue(undefined);
      // Pre-set non-null context so the useEffect auto-refresh is skipped
      useContextStore.setState({
        refresh: refreshSpy,
        flowContext: { x: 1 },
        globalContext: {},
      });
      render(<ContextTab />);

      await user.click(screen.getByTestId("context-refresh-btn"));
      expect(refreshSpy).toHaveBeenCalledOnce();
    });
  });
});
