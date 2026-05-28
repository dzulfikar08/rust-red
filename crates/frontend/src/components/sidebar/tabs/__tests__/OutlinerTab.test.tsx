import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { OutlinerTab } from "../OutlinerTab";
import { useFlowStore } from "../../../../store/flow-store";
import { useWorkspaceStore } from "../../../../store/workspace-store";
import { useEditorStore } from "../../../../store/editor-store";
import { nodeRegistry } from "../../../../red/nodes/registry";
import { eventBus } from "../../../../red/core/events";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStores() {
  useFlowStore.setState({ nodes: [], edges: [], revision: "" });
  useWorkspaceStore.setState({ flows: [], activeFlowId: null });
  useEditorStore.setState({
    debugMessages: [],
    statuses: new Map(),
    notifications: [],
    selectedNodeId: null,
  });
  nodeRegistry.clear();
}

/** Register a node type in the registry */
function registerType(
  type: string,
  category: string,
  color = "#a6bbcf",
  paletteLabel?: string,
) {
  nodeRegistry.registerType(type, {
    id: type,
    type,
    name: type,
    category,
    color,
    defaults: {},
    inputs: 1,
    outputs: 1,
    ...(paletteLabel ? { paletteLabel } : {}),
  });
}

/** Create a flow node */
function makeNode(
  id: string,
  type: string,
  label: string,
  flowId: string,
) {
  return {
    id,
    type: "nrNode",
    position: { x: 0, y: 0 },
    data: {
      type,
      label,
      color: "#a6bbcf",
      inputs: 1,
      outputs: 1,
      flowId,
    },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("OutlinerTab", () => {
  beforeEach(() => {
    resetStores();
    eventBus.clear();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the outliner container", () => {
      render(<OutlinerTab />);
      expect(
        screen.getByTestId("sidebar-tab-content-outliner"),
      ).toBeInTheDocument();
    });

    it("renders search input", () => {
      render(<OutlinerTab />);
      expect(
        screen.getByTestId("outliner-search-input"),
      ).toBeInTheDocument();
    });

    it("shows empty state when no nodes", () => {
      render(<OutlinerTab />);
      expect(screen.getByTestId("outliner-empty-state")).toBeInTheDocument();
      expect(screen.getByTestId("outliner-empty-state")).toHaveTextContent(
        "No nodes in this flow",
      );
    });

    it("shows tree when nodes exist", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");
      registerType("function", "function", "#e2d96e");

      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject 1", flowId),
          makeNode("n2", "function", "My Func", flowId),
        ],
      });

      render(<OutlinerTab />);
      expect(screen.getByTestId("outliner-tree")).toBeInTheDocument();
    });

    it("renders categories with correct labels and counts", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");
      registerType("debug", "common", "#a6bbcf");
      registerType("function", "function", "#e2d96e");

      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject 1", flowId),
          makeNode("n2", "debug", "Debug 1", flowId),
          makeNode("n3", "function", "My Func", flowId),
        ],
      });

      render(<OutlinerTab />);
      expect(
        screen.getByTestId("outliner-category-common"),
      ).toBeInTheDocument();
      expect(
        screen.getByTestId("outliner-category-function"),
      ).toBeInTheDocument();

      // Category headers show count
      const commonBtn = screen.getByTestId("outliner-category-toggle-common");
      expect(commonBtn).toHaveTextContent("common (2)");
      const funcBtn = screen.getByTestId("outliner-category-toggle-function");
      expect(funcBtn).toHaveTextContent("function (1)");
    });

    it("shows node items with labels under expanded categories", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "My Inject Node", flowId)],
      });

      render(<OutlinerTab />);
      expect(
        screen.getByTestId("outliner-node-n1"),
      ).toBeInTheDocument();
      expect(
        screen.getByTestId("outliner-node-n1"),
      ).toHaveTextContent("My Inject Node");
    });

    it("shows colored dot for each node", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);
      const dot = screen.getByTestId("outliner-node-dot-n1");
      expect(dot).toBeInTheDocument();
      expect(dot).toHaveStyle({ backgroundColor: "#a6bbcf" });
    });
  });

  // -----------------------------------------------------------------------
  // Expand / Collapse
  // -----------------------------------------------------------------------

  describe("expand/collapse", () => {
    it("categories are expanded by default", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);
      // Node items should be visible since common category is expanded by default
      expect(
        screen.getByTestId("outliner-category-items-common"),
      ).toBeInTheDocument();
    });

    it("collapses a category when clicking its toggle", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);

      // Click to collapse
      await user.click(screen.getByTestId("outliner-category-toggle-common"));

      // Items should be hidden
      expect(
        screen.queryByTestId("outliner-category-items-common"),
      ).not.toBeInTheDocument();
    });

    it("expands a collapsed category when clicking its toggle", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);

      // Collapse then expand
      await user.click(screen.getByTestId("outliner-category-toggle-common"));
      await user.click(screen.getByTestId("outliner-category-toggle-common"));

      expect(
        screen.getByTestId("outliner-category-items-common"),
      ).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Node click -> selection event
  // -----------------------------------------------------------------------

  describe("node click", () => {
    it("emits node:select-requested event when clicking a node", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      const handler = vi.fn();
      eventBus.on("node:select-requested", handler);

      render(<OutlinerTab />);
      await user.click(screen.getByTestId("outliner-node-n1"));

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({ id: "n1" });
    });

    it("updates selectedNodeId in editor store on click", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);
      await user.click(screen.getByTestId("outliner-node-n1"));

      expect(useEditorStore.getState().selectedNodeId).toBe("n1");
    });
  });

  // -----------------------------------------------------------------------
  // Search / Filter
  // -----------------------------------------------------------------------

  describe("search filter", () => {
    it("filters nodes by label", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");
      registerType("debug", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject Timer", flowId),
          makeNode("n2", "debug", "Debug Output", flowId),
        ],
      });

      render(<OutlinerTab />);

      // Both nodes visible
      expect(screen.getByTestId("outliner-node-n1")).toBeInTheDocument();
      expect(screen.getByTestId("outliner-node-n2")).toBeInTheDocument();

      // Type search query
      await user.type(
        screen.getByTestId("outliner-search-input"),
        "Timer",
      );

      // Only matching node visible
      expect(screen.getByTestId("outliner-node-n1")).toBeInTheDocument();
      expect(
        screen.queryByTestId("outliner-node-n2"),
      ).not.toBeInTheDocument();
    });

    it("shows no matching nodes message when search has no results", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);

      await user.type(
        screen.getByTestId("outliner-search-input"),
        "nonexistent",
      );

      expect(screen.getByTestId("outliner-empty-state")).toHaveTextContent(
        "No matching nodes",
      );
    });

    it("restores all nodes when search is cleared", async () => {
      const user = userEvent.setup();
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", flowId)],
      });

      render(<OutlinerTab />);

      // Type and clear
      await user.type(
        screen.getByTestId("outliner-search-input"),
        "xyz",
      );
      expect(screen.getByTestId("outliner-empty-state")).toBeInTheDocument();

      // Clear the input
      await user.clear(screen.getByTestId("outliner-search-input"));

      expect(screen.getByTestId("outliner-node-n1")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Active flow filtering
  // -----------------------------------------------------------------------

  describe("active flow filtering", () => {
    it("only shows nodes for the active flow", () => {
      // Create two flows
      const flowId1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const flowId2 = useWorkspaceStore.getState().addFlow("Flow 2");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Node in Flow 1", flowId1),
          makeNode("n2", "inject", "Node in Flow 2", flowId2),
        ],
      });

      // Set active to flow 1
      useWorkspaceStore.getState().setActiveFlow(flowId1);

      render(<OutlinerTab />);

      expect(screen.getByTestId("outliner-node-n1")).toBeInTheDocument();
      expect(
        screen.queryByTestId("outliner-node-n2"),
      ).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Selected node highlighting
  // -----------------------------------------------------------------------

  describe("selected node highlighting", () => {
    it("highlights the currently selected node", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      registerType("inject", "common", "#a6bbcf");

      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject 1", flowId),
          makeNode("n2", "inject", "Inject 2", flowId),
        ],
      });

      useEditorStore.getState().selectNode("n1");

      render(<OutlinerTab />);

      const node1 = screen.getByTestId("outliner-node-n1");
      expect(node1.className).toContain("bg-blue-50");
    });
  });
});
