import { describe, it, expect, beforeEach } from "vitest";
import type { Node as RFNode, Edge as RFEdge } from "@xyflow/react";
import { useClipboardStore } from "../clipboard-store";
import { useFlowStore } from "../flow-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeRFNode(overrides: Partial<RFNode> = {}): RFNode {
  return {
    id: "node-1",
    type: "nrNode",
    position: { x: 100, y: 200 },
    data: {
      label: "inject",
      type: "inject",
      category: "common",
      color: "#a6bbcf",
      z: "tab-1",
    },
    ...overrides,
  };
}

function makeEdge(
  source: string,
  target: string,
  sourceHandle = "output-0",
): RFEdge {
  return {
    id: `${source}:0->${target}`,
    source,
    sourceHandle,
    target,
    targetHandle: "input-0",
    type: "nrWire",
  };
}

function resetStores() {
  useFlowStore.setState({ nodes: [], edges: [], revision: "" });
  useClipboardStore.setState({
    clipboard: null,
    clipboardEdges: null,
    hasClipboard: false,
  });
}

function setupFlow() {
  const node1 = makeRFNode({ id: "n1", position: { x: 100, y: 100 } });
  const node2 = makeRFNode({
    id: "n2",
    position: { x: 300, y: 100 },
    data: {
      label: "debug",
      type: "debug",
      category: "common",
      color: "#a6bbcf",
      z: "tab-1",
    },
  });
  const node3 = makeRFNode({ id: "n3", position: { x: 500, y: 100 } });
  const edge1 = makeEdge("n1", "n2");
  const edge2 = makeEdge("n2", "n3");

  useFlowStore.setState({
    nodes: [node1, node2, node3],
    edges: [edge1, edge2],
  });

  return { node1, node2, node3, edge1, edge2 };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useClipboardStore", () => {
  beforeEach(() => {
    resetStores();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty clipboard", () => {
      const state = useClipboardStore.getState();
      expect(state.clipboard).toBeNull();
      expect(state.clipboardEdges).toBeNull();
      expect(state.hasClipboard).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // copy
  // -----------------------------------------------------------------------

  describe("copy()", () => {
    it("copies selected nodes to clipboard", () => {
      const { node1 } = setupFlow();

      useClipboardStore.getState().copy(["n1"]);

      const state = useClipboardStore.getState();
      expect(state.clipboard).toHaveLength(1);
      expect(state.clipboard![0].id).toBe("n1");
      expect(state.hasClipboard).toBe(true);
    });

    it("copies multiple nodes", () => {
      setupFlow();

      useClipboardStore.getState().copy(["n1", "n2"]);

      const state = useClipboardStore.getState();
      expect(state.clipboard).toHaveLength(2);
    });

    it("copies interconnecting edges only", () => {
      setupFlow();

      useClipboardStore.getState().copy(["n1", "n2"]);

      const state = useClipboardStore.getState();
      // Only edge from n1->n2 should be copied, not n2->n3
      expect(state.clipboardEdges).toHaveLength(1);
      expect(state.clipboardEdges![0].source).toBe("n1");
      expect(state.clipboardEdges![0].target).toBe("n2");
    });

    it("does not copy edges to nodes outside selection", () => {
      setupFlow();

      useClipboardStore.getState().copy(["n1"]);

      const state = useClipboardStore.getState();
      expect(state.clipboardEdges).toHaveLength(0);
    });

    it("handles copying non-existent nodes gracefully", () => {
      setupFlow();

      useClipboardStore.getState().copy(["nonexistent"]);

      const state = useClipboardStore.getState();
      expect(state.clipboard).toHaveLength(0);
      expect(state.hasClipboard).toBe(false);
    });

    it("does nothing with empty nodeIds array", () => {
      setupFlow();

      useClipboardStore.getState().copy([]);

      const state = useClipboardStore.getState();
      expect(state.clipboard).toBeNull();
      expect(state.hasClipboard).toBe(false);
    });

    it("deep clones nodes (modifying original does not affect clipboard)", () => {
      const { node1 } = setupFlow();

      useClipboardStore.getState().copy(["n1"]);

      // Mutate original
      useFlowStore.getState().updateNodeData("n1", { label: "changed" });

      const clipboardNode = useClipboardStore.getState().clipboard![0];
      expect(clipboardNode.data.label).toBe("inject");
    });
  });

  // -----------------------------------------------------------------------
  // cut
  // -----------------------------------------------------------------------

  describe("cut()", () => {
    it("copies nodes to clipboard and removes them from canvas", () => {
      setupFlow();

      useClipboardStore.getState().cut(["n1"]);

      // Clipboard should have the node
      const clipState = useClipboardStore.getState();
      expect(clipState.clipboard).toHaveLength(1);
      expect(clipState.hasClipboard).toBe(true);

      // Canvas should not have the node
      const flowState = useFlowStore.getState();
      expect(flowState.nodes.find((n) => n.id === "n1")).toBeUndefined();
    });

    it("removes edges to/from cut nodes", () => {
      setupFlow();

      useClipboardStore.getState().cut(["n1"]);

      const flowState = useFlowStore.getState();
      // Edge n1->n2 should be gone
      expect(flowState.edges.find((e) => e.source === "n1")).toBeUndefined();
    });

    it("cuts multiple nodes", () => {
      setupFlow();

      useClipboardStore.getState().cut(["n1", "n2"]);

      const flowState = useFlowStore.getState();
      expect(flowState.nodes).toHaveLength(1);
      expect(flowState.nodes[0].id).toBe("n3");
    });
  });

  // -----------------------------------------------------------------------
  // paste
  // -----------------------------------------------------------------------

  describe("paste()", () => {
    it("returns empty array when clipboard is empty", () => {
      const result = useClipboardStore.getState().paste();
      expect(result).toEqual([]);
    });

    it("pastes nodes with new IDs", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      const pasted = useClipboardStore.getState().paste();

      expect(pasted).toHaveLength(1);
      // New ID should be different from original
      expect(pasted[0].id).not.toBe("n1");
      // But type should be preserved
      expect(pasted[0].data.type).toBe("inject");
    });

    it("offsets pasted node positions by 20px", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      const pasted = useClipboardStore.getState().paste();

      // Original n1 is at (100, 100), pasted should be at (120, 120)
      expect(pasted[0].position.x).toBe(120);
      expect(pasted[0].position.y).toBe(120);
    });

    it("supports custom offset", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      const pasted = useClipboardStore.getState().paste(50, 30);

      expect(pasted[0].position.x).toBe(150);
      expect(pasted[0].position.y).toBe(130);
    });

    it("adds pasted nodes to the flow store", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      useClipboardStore.getState().paste();

      // Should have 4 nodes now: original 3 + 1 pasted
      expect(useFlowStore.getState().nodes).toHaveLength(4);
    });

    it("pastes edges with remapped IDs", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1", "n2"]);

      const pasted = useClipboardStore.getState().paste();

      // Flow store should have original 2 edges + 1 pasted edge
      const flowEdges = useFlowStore.getState().edges;
      expect(flowEdges.length).toBeGreaterThanOrEqual(3);

      // Find the new edge (should reference the pasted node IDs)
      const pastedIds = new Set(pasted.map((n) => n.id));
      const newEdge = flowEdges.find(
        (e) => pastedIds.has(e.source) && pastedIds.has(e.target),
      );
      expect(newEdge).toBeDefined();
    });

    it("can paste multiple times without clearing clipboard", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      useClipboardStore.getState().paste();
      useClipboardStore.getState().paste();

      // Should have 5 nodes: original 3 + 2 pasted
      expect(useFlowStore.getState().nodes).toHaveLength(5);
    });

    it("each paste generates unique IDs", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      const paste1 = useClipboardStore.getState().paste();
      const paste2 = useClipboardStore.getState().paste();

      expect(paste1[0].id).not.toBe(paste2[0].id);
    });
  });

  // -----------------------------------------------------------------------
  // exportToJson
  // -----------------------------------------------------------------------

  describe("exportToJson()", () => {
    it("exports all nodes as valid JSON array", () => {
      setupFlow();

      const json = useClipboardStore.getState().exportToJson();
      const parsed = JSON.parse(json);

      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed).toHaveLength(3);
    });

    it("exports Node-RED flow format", () => {
      setupFlow();

      const json = useClipboardStore.getState().exportToJson();
      const parsed = JSON.parse(json);

      // Each node should have id, type, x, y, wires
      for (const node of parsed) {
        expect(node).toHaveProperty("id");
        expect(node).toHaveProperty("type");
        expect(node).toHaveProperty("x");
        expect(node).toHaveProperty("y");
        expect(node).toHaveProperty("wires");
      }
    });

    it("exports wires correctly", () => {
      setupFlow();

      const json = useClipboardStore.getState().exportToJson();
      const parsed = JSON.parse(json);

      // n1 has a wire to n2
      const n1 = parsed.find((n: { id: string }) => n.id === "n1");
      expect(n1.wires).toEqual([["n2"]]);
    });

    it("exports specific nodes only", () => {
      setupFlow();

      const json = useClipboardStore.getState().exportToJson(["n1"]);
      const parsed = JSON.parse(json);

      expect(parsed).toHaveLength(1);
      expect(parsed[0].id).toBe("n1");
    });

    it("exports empty array when no nodes", () => {
      const json = useClipboardStore.getState().exportToJson();
      const parsed = JSON.parse(json);

      expect(parsed).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // importFromJson
  // -----------------------------------------------------------------------

  describe("importFromJson()", () => {
    it("imports nodes from Node-RED JSON", () => {
      const json = JSON.stringify([
        { id: "abc", type: "inject", x: 100, y: 200, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].data.type).toBe("inject");
    });

    it("generates new IDs on import", () => {
      const json = JSON.stringify([
        { id: "abc", type: "inject", x: 100, y: 200, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes[0].id).not.toBe("abc");
    });

    it("imports edges from wires", () => {
      const json = JSON.stringify([
        { id: "a1", type: "inject", x: 100, y: 100, z: "tab-1", wires: [["a2"]] },
        { id: "a2", type: "debug", x: 300, y: 100, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.edges.length).toBeGreaterThanOrEqual(1);
    });

    it("remaps edge source/target to new IDs", () => {
      const json = JSON.stringify([
        { id: "a1", type: "inject", x: 100, y: 100, z: "tab-1", wires: [["a2"]] },
        { id: "a2", type: "debug", x: 300, y: 100, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      // Edges should reference the new IDs, not originals
      const nodeIds = new Set(result.nodes.map((n) => n.id));
      for (const edge of result.edges) {
        expect(nodeIds.has(edge.source)).toBe(true);
        expect(nodeIds.has(edge.target)).toBe(true);
      }
    });

    it("adds imported nodes to the flow store", () => {
      const json = JSON.stringify([
        { id: "abc", type: "inject", x: 100, y: 200, z: "tab-1", wires: [] },
      ]);

      useClipboardStore.getState().importFromJson(json);

      expect(useFlowStore.getState().nodes).toHaveLength(1);
    });

    it("handles multiple nodes", () => {
      const json = JSON.stringify([
        { id: "a", type: "inject", x: 100, y: 100, z: "tab-1", wires: [["b"]] },
        { id: "b", type: "function", x: 300, y: 100, z: "tab-1", wires: [["c"]] },
        { id: "c", type: "debug", x: 500, y: 100, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes).toHaveLength(3);
    });

    it("skips tab entries", () => {
      const json = JSON.stringify([
        { id: "tab-1", type: "tab", label: "Flow 1" },
        { id: "abc", type: "inject", x: 100, y: 200, z: "tab-1", wires: [] },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes).toHaveLength(1);
    });

    it("preserves extra node properties", () => {
      const json = JSON.stringify([
        {
          id: "abc",
          type: "function",
          x: 100,
          y: 200,
          z: "tab-1",
          wires: [],
          func: "return msg;",
          outputs: 1,
        },
      ]);

      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes[0].data.func).toBe("return msg;");
      expect(result.nodes[0].data.outputs).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // clear
  // -----------------------------------------------------------------------

  describe("clear()", () => {
    it("clears the clipboard", () => {
      setupFlow();
      useClipboardStore.getState().copy(["n1"]);

      expect(useClipboardStore.getState().hasClipboard).toBe(true);

      useClipboardStore.getState().clear();

      const state = useClipboardStore.getState();
      expect(state.clipboard).toBeNull();
      expect(state.clipboardEdges).toBeNull();
      expect(state.hasClipboard).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // Round-trip: export -> import
  // -----------------------------------------------------------------------

  describe("round-trip export/import", () => {
    it("can re-import exported JSON", () => {
      setupFlow();

      const json = useClipboardStore.getState().exportToJson(["n1", "n2"]);
      const result = useClipboardStore.getState().importFromJson(json);

      expect(result.nodes).toHaveLength(2);
      // Types should be preserved
      const types = result.nodes.map((n) => n.data.type);
      expect(types).toContain("inject");
      expect(types).toContain("debug");
    });
  });
});
