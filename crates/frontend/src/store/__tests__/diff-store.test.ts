import { describe, it, expect, beforeEach, vi } from "vitest";
import { useDiffStore, computeDiff } from "../diff-store";
import type { Node } from "@xyflow/react";
import type { FlowNodeData } from "../../api/types";

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
    data: { label: id, type: "inject", ...overrides.data },
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

// ---------------------------------------------------------------------------
// computeDiff (pure function tests)
// ---------------------------------------------------------------------------

describe("computeDiff", () => {
  it("returns all nodes as unchanged when local and remote are identical", () => {
    const nodes = [makeNode("a"), makeNode("b")];

    const result = computeDiff(nodes, nodes);

    expect(result.unchanged).toHaveLength(2);
    expect(result.added).toHaveLength(0);
    expect(result.removed).toHaveLength(0);
    expect(result.modified).toHaveLength(0);
  });

  it("detects added nodes", () => {
    const local = [makeNode("a"), makeNode("b"), makeNode("c")];
    const remote = [makeNode("a")];

    const result = computeDiff(local, remote);

    expect(result.added).toHaveLength(2);
    expect(result.added.map((n) => n.id)).toContain("b");
    expect(result.added.map((n) => n.id)).toContain("c");
  });

  it("detects removed nodes", () => {
    const local = [makeNode("a")];
    const remote = [makeNode("a"), makeNode("b"), makeNode("c")];

    const result = computeDiff(local, remote);

    expect(result.removed).toHaveLength(2);
    expect(result.removed.map((n) => n.id)).toContain("b");
    expect(result.removed.map((n) => n.id)).toContain("c");
  });

  it("detects modified nodes by data property change", () => {
    const local = [makeNode("a", { data: { label: "a", type: "inject", name: "new-name" } })];
    const remote = [makeNode("a", { data: { label: "a", type: "inject", name: "old-name" } })];

    const result = computeDiff(local, remote);

    expect(result.modified).toHaveLength(1);
    expect(result.modified[0].node.id).toBe("a");
    expect(result.modified[0].changes.name).toEqual({
      old: "old-name",
      new: "new-name",
    });
  });

  it("detects modified nodes by position change", () => {
    const local = [makeNode("a", { position: { x: 100, y: 200 } })];
    const remote = [makeNode("a", { position: { x: 50, y: 100 } })];

    const result = computeDiff(local, remote);

    expect(result.modified).toHaveLength(1);
    expect(result.modified[0].changes.x).toEqual({ old: 50, new: 100 });
    expect(result.modified[0].changes.y).toEqual({ old: 100, new: 200 });
  });

  it("detects multiple property changes on a single node", () => {
    const local = [
      makeNode("a", {
        position: { x: 100, y: 200 },
        data: { label: "a", type: "inject", payload: "hello", topic: "world" },
      }),
    ];
    const remote = [
      makeNode("a", {
        position: { x: 0, y: 0 },
        data: { label: "a", type: "inject", payload: "goodbye", topic: "world" },
      }),
    ];

    const result = computeDiff(local, remote);

    expect(result.modified).toHaveLength(1);
    const changes = result.modified[0].changes;
    expect(changes.x).toEqual({ old: 0, new: 100 });
    expect(changes.y).toEqual({ old: 0, new: 200 });
    expect(changes.payload).toEqual({ old: "goodbye", new: "hello" });
    // topic is the same -- should NOT appear
    expect(changes.topic).toBeUndefined();
  });

  it("handles mixed add/remove/modify/unchanged", () => {
    const local = [
      makeNode("a"), // unchanged
      makeNode("b", { data: { label: "b", type: "inject", name: "updated" } }), // modified
      makeNode("c"), // added
    ];
    const remote = [
      makeNode("a"), // unchanged
      makeNode("b", { data: { label: "b", type: "inject", name: "original" } }), // modified
      makeNode("d"), // removed (not in local)
    ];

    const result = computeDiff(local, remote);

    expect(result.unchanged).toHaveLength(1);
    expect(result.unchanged[0].id).toBe("a");

    expect(result.modified).toHaveLength(1);
    expect(result.modified[0].node.id).toBe("b");

    expect(result.added).toHaveLength(1);
    expect(result.added[0].id).toBe("c");

    expect(result.removed).toHaveLength(1);
    expect(result.removed[0].id).toBe("d");
  });

  it("returns empty arrays when both inputs are empty", () => {
    const result = computeDiff([], []);

    expect(result.added).toHaveLength(0);
    expect(result.removed).toHaveLength(0);
    expect(result.modified).toHaveLength(0);
    expect(result.unchanged).toHaveLength(0);
  });

  it("treats all local nodes as added when remote is empty", () => {
    const local = [makeNode("a"), makeNode("b")];

    const result = computeDiff(local, []);

    expect(result.added).toHaveLength(2);
    expect(result.removed).toHaveLength(0);
    expect(result.modified).toHaveLength(0);
  });

  it("treats all remote nodes as removed when local is empty", () => {
    const remote = [makeNode("a"), makeNode("b")];

    const result = computeDiff([], remote);

    expect(result.removed).toHaveLength(2);
    expect(result.added).toHaveLength(0);
    expect(result.modified).toHaveLength(0);
  });

  it("detects type change as a modification", () => {
    const local = [makeNode("a", { data: { label: "a", type: "debug" } })];
    const remote = [makeNode("a", { data: { label: "a", type: "inject" } })];

    const result = computeDiff(local, remote);

    expect(result.modified).toHaveLength(1);
    expect(result.modified[0].changes.type).toEqual({
      old: "inject",
      new: "debug",
    });
  });
});

// ---------------------------------------------------------------------------
// Store actions
// ---------------------------------------------------------------------------

describe("useDiffStore", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("default state", () => {
    it("starts with null localFlow", () => {
      expect(useDiffStore.getState().localFlow).toBeNull();
    });

    it("starts with null remoteFlow", () => {
      expect(useDiffStore.getState().remoteFlow).toBeNull();
    });

    it("starts with null diffResult", () => {
      expect(useDiffStore.getState().diffResult).toBeNull();
    });

    it("starts with isLoading false", () => {
      expect(useDiffStore.getState().isLoading).toBe(false);
    });

    it("starts with filter 'changes'", () => {
      expect(useDiffStore.getState().filter).toBe("changes");
    });
  });

  describe("computeDiff (store action)", () => {
    it("computes and stores the diff result", () => {
      const local = [makeNode("a")];
      const remote = [makeNode("a"), makeNode("b")];

      const result = useDiffStore.getState().computeDiff(local, remote);

      expect(useDiffStore.getState().diffResult).toBe(result);
      expect(useDiffStore.getState().localFlow).toBe(local);
      expect(useDiffStore.getState().remoteFlow).toBe(remote);
      expect(result.removed).toHaveLength(1);
    });
  });

  describe("setFilter", () => {
    it("sets the filter to 'all'", () => {
      useDiffStore.getState().setFilter("all");
      expect(useDiffStore.getState().filter).toBe("all");
    });

    it("sets the filter to 'changes'", () => {
      useDiffStore.getState().setFilter("all");
      useDiffStore.getState().setFilter("changes");
      expect(useDiffStore.getState().filter).toBe("changes");
    });
  });

  describe("clear", () => {
    it("resets all store state to defaults", () => {
      const local = [makeNode("a")];
      const remote = [makeNode("b")];
      useDiffStore.getState().computeDiff(local, remote);
      useDiffStore.getState().setFilter("all");

      useDiffStore.getState().clear();

      expect(useDiffStore.getState().localFlow).toBeNull();
      expect(useDiffStore.getState().remoteFlow).toBeNull();
      expect(useDiffStore.getState().diffResult).toBeNull();
      expect(useDiffStore.getState().isLoading).toBe(false);
      expect(useDiffStore.getState().filter).toBe("changes");
    });
  });

  describe("fetchRemoteAndDiff", () => {
    it("fetches remote flows and computes diff", async () => {
      const mockNodes = [makeNode("remote-1"), makeNode("remote-2")];

      // Mock the API module
      const { flowsApi } = await import("../../api/flows");
      vi.spyOn(flowsApi, "getFlows").mockResolvedValue({
        rev: "test-rev",
        nodes: mockNodes,
        edges: [],
      });

      // Set local nodes
      const { useFlowStore } = await import("../flow-store");
      useFlowStore.getState().setNodes([makeNode("remote-1")]);

      await useDiffStore.getState().fetchRemoteAndDiff();

      const result = useDiffStore.getState().diffResult;
      expect(result).not.toBeNull();
      expect(result!.added).toHaveLength(0);
      expect(result!.removed).toHaveLength(1);
      expect(result!.removed[0].id).toBe("remote-2");
      expect(useDiffStore.getState().isLoading).toBe(false);

      vi.restoreAllMocks();
    });

    it("handles fetch errors gracefully", async () => {
      const { flowsApi } = await import("../../api/flows");
      vi.spyOn(flowsApi, "getFlows").mockRejectedValue(new Error("Network error"));

      // Mock notification store to avoid side effects
      const { useNotificationStore } = await import("../notification-store");
      vi.spyOn(useNotificationStore.getState(), "error").mockImplementation(() => "");

      await useDiffStore.getState().fetchRemoteAndDiff();

      expect(useDiffStore.getState().isLoading).toBe(false);
      expect(useDiffStore.getState().diffResult).toBeNull();

      vi.restoreAllMocks();
    });
  });
});
