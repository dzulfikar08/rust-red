import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DiffDialog } from "../DiffDialog";
import { useDiffStore } from "../../../store/diff-store";
import type { Node } from "@xyflow/react";
import type { FlowNodeData } from "../../../api/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type FlowNode = Node<FlowNodeData>;

function makeNode(
  id: string,
  overrides: Partial<FlowNode> & { data?: Partial<FlowNodeData> } = {},
): FlowNode {
  return {
    id,
    type: "default",
    position: { x: 0, y: 0 },
    data: { label: `Node ${id}`, type: "inject", ...overrides.data },
    ...overrides,
  };
}

function resetStore() {
  useDiffStore.setState({
    localFlow: null,
    remoteFlow: null,
    diffResult: null,
    isLoading: false,
    filter: "changes",
  });
}

function setupDiff() {
  const local = [
    makeNode("a"), // unchanged
    makeNode("b", { data: { label: "Node b", type: "inject", name: "updated" } }), // modified
    makeNode("c"), // added
  ];
  const remote = [
    makeNode("a"), // unchanged
    makeNode("b", { data: { label: "Node b", type: "inject", name: "original" } }), // modified
    makeNode("d"), // removed
  ];

  useDiffStore.getState().computeDiff(local, remote);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("DiffDialog", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders nothing when open is false", () => {
      render(<DiffDialog open={false} onClose={() => {}} />);
      expect(screen.queryByTestId("diff-dialog")).not.toBeInTheDocument();
    });

    it("renders the dialog when open is true", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByTestId("diff-dialog")).toBeInTheDocument();
    });

    it("renders the title", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("Compare with Deployed")).toBeInTheDocument();
    });

    it("shows loading state when isLoading is true", () => {
      useDiffStore.setState({ isLoading: true });
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("Loading diff...")).toBeInTheDocument();
    });

    it("shows no diff data message when diffResult is null and not loading", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("No diff data available")).toBeInTheDocument();
    });

    it("renders column headers", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("Local (Editor)")).toBeInTheDocument();
      expect(screen.getByText("Remote (Deployed)")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Summary counts
  // -----------------------------------------------------------------------

  describe("summary counts", () => {
    it("shows node count summary for each category", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("1 added")).toBeInTheDocument();
      expect(screen.getByText("1 removed")).toBeInTheDocument();
      expect(screen.getByText("1 modified")).toBeInTheDocument();
    });

    it("shows no changes message when everything is identical", () => {
      const nodes = [makeNode("a")];
      useDiffStore.getState().computeDiff(nodes, nodes);
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByText("No changes detected")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Filter
  // -----------------------------------------------------------------------

  describe("filter toggle", () => {
    it("renders filter buttons", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByTestId("diff-filter-changes")).toBeInTheDocument();
      expect(screen.getByTestId("diff-filter-all")).toBeInTheDocument();
    });

    it("defaults to changes-only filter", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);

      // Unchanged nodes should NOT be shown in changes-only mode
      // "Node a" is unchanged, so it shouldn't appear as a diff-node
      const unchangedNodes = screen.queryAllByTestId("diff-node-unchanged");
      expect(unchangedNodes).toHaveLength(0);
    });

    it("shows unchanged nodes when filter is set to all", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);

      const showAllBtn = screen.getByTestId("diff-filter-all");
      fireEvent.click(showAllBtn);

      // Now unchanged nodes should appear (in both columns)
      const unchangedNodes = screen.queryAllByTestId("diff-node-unchanged");
      expect(unchangedNodes.length).toBeGreaterThanOrEqual(2); // one in each column
    });

    it("highlights active filter button", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      const changesBtn = screen.getByTestId("diff-filter-changes");
      expect(changesBtn.className).toContain("bg-gray-200");
    });
  });

  // -----------------------------------------------------------------------
  // Color-coded node rows
  // -----------------------------------------------------------------------

  describe("node rows", () => {
    it("renders added nodes with green styling", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      const addedNodes = screen.getAllByTestId("diff-node-added");
      expect(addedNodes.length).toBeGreaterThanOrEqual(1);
      // Check it has the "added" badge text
      expect(addedNodes[0].textContent).toContain("added");
    });

    it("renders removed nodes with red styling", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      const removedNodes = screen.getAllByTestId("diff-node-removed");
      expect(removedNodes.length).toBeGreaterThanOrEqual(1);
      expect(removedNodes[0].textContent).toContain("removed");
    });

    it("renders modified nodes with yellow styling", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      const modifiedNodes = screen.getAllByTestId("diff-node-modified");
      expect(modifiedNodes.length).toBeGreaterThanOrEqual(1);
      expect(modifiedNodes[0].textContent).toContain("modified");
    });

    it("shows property-level diff for modified nodes", () => {
      setupDiff();
      render(<DiffDialog open={true} onClose={() => {}} />);
      const propDiff = screen.getAllByTestId("diff-property-name");
      expect(propDiff.length).toBeGreaterThanOrEqual(1);
      // Should show old and new values
      expect(propDiff[0].textContent).toContain("original");
      expect(propDiff[0].textContent).toContain("updated");
    });
  });

  // -----------------------------------------------------------------------
  // Action buttons
  // -----------------------------------------------------------------------

  describe("action buttons", () => {
    it("renders Cancel, Merge, and Overwrite buttons", () => {
      render(<DiffDialog open={true} onClose={() => {}} />);
      expect(screen.getByTestId("diff-cancel")).toBeInTheDocument();
      expect(screen.getByTestId("diff-merge")).toBeInTheDocument();
      expect(screen.getByTestId("diff-overwrite")).toBeInTheDocument();
    });

    it("calls onClose when Cancel is clicked", () => {
      const onClose = vi.fn();
      render(<DiffDialog open={true} onClose={onClose} />);
      fireEvent.click(screen.getByTestId("diff-cancel"));
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("calls onClose when backdrop is clicked", () => {
      const onClose = vi.fn();
      render(<DiffDialog open={true} onClose={onClose} />);
      // Click on the overlay
      const overlay = screen.getByTestId("diff-dialog").parentElement!;
      fireEvent.click(overlay);
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("calls onClose when X button is clicked", () => {
      const onClose = vi.fn();
      render(<DiffDialog open={true} onClose={onClose} />);
      const closeBtn = screen.getByLabelText("Close");
      fireEvent.click(closeBtn);
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  // -----------------------------------------------------------------------
  // Merge action
  // -----------------------------------------------------------------------

  describe("merge action", () => {
    it("merges remote-added nodes into local on Merge click", async () => {
      const local = [makeNode("a")];
      const remote = [makeNode("a"), makeNode("b")];
      useDiffStore.getState().computeDiff(local, remote);

      const { useFlowStore } = await import("../../../store/flow-store");
      useFlowStore.getState().setNodes(local);

      const onClose = vi.fn();
      render(<DiffDialog open={true} onClose={onClose} />);

      fireEvent.click(screen.getByTestId("diff-merge"));

      const mergedNodes = useFlowStore.getState().nodes;
      expect(mergedNodes).toHaveLength(2);
      expect(mergedNodes.map((n) => n.id)).toContain("a");
      expect(mergedNodes.map((n) => n.id)).toContain("b");
    });
  });

  // -----------------------------------------------------------------------
  // Overwrite action
  // -----------------------------------------------------------------------

  describe("overwrite action", () => {
    it("replaces local nodes with remote on Overwrite click", async () => {
      const local = [makeNode("a"), makeNode("c")];
      const remote = [makeNode("a"), makeNode("b")];
      useDiffStore.getState().computeDiff(local, remote);

      const { useFlowStore } = await import("../../../store/flow-store");
      useFlowStore.getState().setNodes(local);

      const onClose = vi.fn();
      render(<DiffDialog open={true} onClose={onClose} />);

      fireEvent.click(screen.getByTestId("diff-overwrite"));

      const overwrittenNodes = useFlowStore.getState().nodes;
      expect(overwrittenNodes).toHaveLength(2);
      expect(overwrittenNodes.map((n) => n.id)).toContain("a");
      expect(overwrittenNodes.map((n) => n.id)).toContain("b");
    });
  });
});
