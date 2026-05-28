import { describe, it, expect } from "vitest";
import {
  flowToReactFlow,
  flowNodeToRFNode,
  wireToEdges,
  reactFlowToFlow,
  rfNodeToFlowNode,
} from "../flow-model";
import type { Flow, FlowNode, ConfigNode } from "../types";

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const makeFlowNode = (overrides: Partial<FlowNode> = {}): FlowNode => ({
  id: "node-1",
  type: "inject",
  x: 100,
  y: 200,
  z: "tab-1",
  wires: [["node-2"]],
  ...overrides,
});

const makeFlow = (overrides: Partial<Flow> = {}): Flow => ({
  id: "tab-1",
  label: "Flow 1",
  nodes: [makeFlowNode()],
  configs: [],
  ...overrides,
});

// ---------------------------------------------------------------------------
// flowNodeToRFNode
// ---------------------------------------------------------------------------

describe("flowNodeToRFNode", () => {
  it("maps basic fields correctly", () => {
    const node = makeFlowNode();
    const rf = flowNodeToRFNode(node);

    expect(rf.id).toBe("node-1");
    expect(rf.type).toBe("nrNode");
    expect(rf.position).toEqual({ x: 100, y: 200 });
    expect(rf.data.type).toBe("inject");
    expect(rf.data.label).toBe("inject"); // no name → falls back to type
  });

  it("uses node.name as label when present", () => {
    const node = makeFlowNode({ name: "My Inject" });
    const rf = flowNodeToRFNode(node);
    expect(rf.data.label).toBe("My Inject");
  });

  it("preserves extra node properties in data", () => {
    const node = makeFlowNode({ topic: "hello", payload: 42 });
    const rf = flowNodeToRFNode(node);
    expect(rf.data.topic).toBe("hello");
    expect(rf.data.payload).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// wireToEdges
// ---------------------------------------------------------------------------

describe("wireToEdges", () => {
  it("creates edges from a single-output node", () => {
    const node = makeFlowNode({ wires: [["node-2"]] });
    const edges = wireToEdges(node);

    expect(edges).toHaveLength(1);
    expect(edges[0]).toEqual({
      id: "node-1:0->node-2",
      source: "node-1",
      sourceHandle: "output-0",
      target: "node-2",
      targetHandle: "input-0",
      type: "nrWire",
    });
  });

  it("creates edges from a multi-output node", () => {
    const node = makeFlowNode({
      type: "switch",
      wires: [["node-a"], ["node-b", "node-c"]],
    });
    const edges = wireToEdges(node);

    expect(edges).toHaveLength(3);
    expect(edges[0].sourceHandle).toBe("output-0");
    expect(edges[0].target).toBe("node-a");
    expect(edges[1].sourceHandle).toBe("output-1");
    expect(edges[1].target).toBe("node-b");
    expect(edges[2].sourceHandle).toBe("output-1");
    expect(edges[2].target).toBe("node-c");
  });

  it("returns empty array for a node with no wires", () => {
    const node = makeFlowNode({ wires: [] });
    expect(wireToEdges(node)).toEqual([]);
  });

  it("handles undefined wires", () => {
    const node = makeFlowNode({ wires: undefined });
    expect(wireToEdges(node as FlowNode)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// flowToReactFlow
// ---------------------------------------------------------------------------

describe("flowToReactFlow", () => {
  it("converts a complete flow with nodes and configs", () => {
    const flow = makeFlow({
      nodes: [
        makeFlowNode({ id: "n1", type: "inject", wires: [["n2"]] }),
        makeFlowNode({ id: "n2", type: "debug", wires: [] }),
      ],
      configs: [
        {
          id: "c1",
          type: "mqtt-broker",
          x: 0,
          y: 0,
          z: "tab-1",
          wires: [],
          users: ["n1"],
        } as ConfigNode,
      ],
    });

    const result = flowToReactFlow(flow);

    // 2 regular nodes + 1 config node
    expect(result.nodes).toHaveLength(3);
    // 1 edge: n1 → n2
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].source).toBe("n1");
    expect(result.edges[0].target).toBe("n2");
  });

  it("handles an empty flow", () => {
    const flow = makeFlow({ nodes: [], configs: [] });
    const result = flowToReactFlow(flow);
    expect(result.nodes).toHaveLength(0);
    expect(result.edges).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// reactFlowToFlow
// ---------------------------------------------------------------------------

describe("reactFlowToFlow", () => {
  it("round-trips a simple inject → debug flow", () => {
    // Start with a flow, convert to RF, then back
    const originalNode1: FlowNode = {
      id: "n1",
      type: "inject",
      name: "Inject",
      x: 100,
      y: 100,
      z: "tab-1",
      wires: [["n2"]],
    };
    const originalNode2: FlowNode = {
      id: "n2",
      type: "debug",
      x: 300,
      y: 100,
      z: "tab-1",
      wires: [],
    };

    const flow: Flow = {
      id: "tab-1",
      label: "Test",
      nodes: [originalNode1, originalNode2],
      configs: [],
    };

    const { nodes: rfNodes, edges: rfEdges } = flowToReactFlow(flow);
    const { nodes: roundTripped } = reactFlowToFlow(rfNodes, rfEdges);

    expect(roundTripped).toHaveLength(2);

    const rt1 = roundTripped.find((n) => n.id === "n1")!;
    expect(rt1.type).toBe("inject");
    expect(rt1.x).toBe(100);
    expect(rt1.y).toBe(100);
    expect(rt1.wires).toEqual([["n2"]]);

    const rt2 = roundTripped.find((n) => n.id === "n2")!;
    expect(rt2.type).toBe("debug");
    expect(rt2.wires).toEqual([]);
  });

  it("handles multi-output round-trip", () => {
    const original: FlowNode = {
      id: "sw1",
      type: "switch",
      x: 200,
      y: 200,
      z: "tab-1",
      wires: [["a"], ["b", "c"]],
    };

    const flow: Flow = {
      id: "tab-1",
      label: "Switch test",
      nodes: [original, { id: "a", type: "debug", x: 400, y: 100, z: "tab-1", wires: [] },
        { id: "b", type: "debug", x: 400, y: 200, z: "tab-1", wires: [] },
        { id: "c", type: "debug", x: 400, y: 300, z: "tab-1", wires: [] }],
      configs: [],
    };

    const { nodes, edges } = flowToReactFlow(flow);
    const { nodes: roundTripped } = reactFlowToFlow(nodes, edges);

    const sw = roundTripped.find((n) => n.id === "sw1")!;
    expect(sw.wires).toEqual([["a"], ["b", "c"]]);
  });

  it("separates config nodes based on _isConfig flag", () => {
    const { nodes: rfNodes, edges: rfEdges } = flowToReactFlow({
      id: "tab-1",
      label: "Config test",
      nodes: [makeFlowNode({ id: "n1", wires: [] })],
      configs: [{
        id: "cfg1",
        type: "mqtt-broker",
        x: 0, y: 0, z: "tab-1",
        wires: [],
        users: ["n1"],
      } as ConfigNode],
    });

    // Mark config node
    const cfgNode = rfNodes.find((n) => n.id === "cfg1")!;
    cfgNode.data._isConfig = true;
    cfgNode.data._users = ["n1"];

    const { nodes, configs } = reactFlowToFlow(rfNodes, rfEdges);

    expect(nodes.find((n) => n.id === "cfg1")).toBeUndefined();
    expect(configs.find((c) => c.id === "cfg1")).toBeDefined();
    expect(configs[0].users).toEqual(["n1"]);
  });
});

// ---------------------------------------------------------------------------
// rfNodeToFlowNode (unit-level)
// ---------------------------------------------------------------------------

describe("rfNodeToFlowNode", () => {
  it("strips synthetic data keys", () => {
    const node = makeFlowNode({ name: "Test", topic: "t" });
    const rf = flowNodeToRFNode(node);
    const flow = rfNodeToFlowNode(rf, node.wires);

    expect(flow.id).toBe(node.id);
    expect(flow.type).toBe("inject");
    expect(flow.x).toBe(100);
    expect(flow.y).toBe(200);
    expect(flow.wires).toEqual(node.wires);
    // name should be preserved because it differed from type
    expect(flow.name).toBe("Test");
    // topic should be preserved via rest spread
    expect(flow.topic).toBe("t");
    // internal keys should NOT be present
    expect((flow as Record<string, unknown>).category).toBeUndefined();
    expect((flow as Record<string, unknown>).color).toBeUndefined();
  });
});
