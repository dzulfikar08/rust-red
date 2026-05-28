import { describe, it, expect, beforeEach } from "vitest";
import { useSearchStore } from "../search-store";
import { useFlowStore } from "../flow-store";
import { useWorkspaceStore } from "../workspace-store";
import { eventBus } from "../../red/core/events";
import type { Node } from "@xyflow/react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useSearchStore", () => {
  beforeEach(() => {
    resetStores();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty query and results", () => {
      const state = useSearchStore.getState();
      expect(state.query).toBe("");
      expect(state.results).toEqual([]);
      expect(state.isSearching).toBe(false);
      expect(state.selectedIndex).toBe(0);
      expect(state.isOpen).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // open / close
  // -----------------------------------------------------------------------

  describe("open()", () => {
    it("sets isOpen to true", () => {
      useSearchStore.getState().open();
      expect(useSearchStore.getState().isOpen).toBe(true);
    });
  });

  describe("close()", () => {
    it("resets all search state", () => {
      useSearchStore.getState().open();
      useSearchStore.getState().search("test");
      useSearchStore.getState().close();
      const state = useSearchStore.getState();
      expect(state.isOpen).toBe(false);
      expect(state.query).toBe("");
      expect(state.results).toEqual([]);
      expect(state.selectedIndex).toBe(0);
      expect(state.isSearching).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // search - basic
  // -----------------------------------------------------------------------

  describe("search()", () => {
    it("finds no results with empty query", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject Node")],
      });
      useSearchStore.getState().search("");
      expect(useSearchStore.getState().results).toEqual([]);
    });

    it("finds no results with whitespace-only query", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject Node")],
      });
      useSearchStore.getState().search("   ");
      expect(useSearchStore.getState().results).toEqual([]);
    });

    it("finds results by node label (case-insensitive)", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "My Inject Node")],
      });
      useWorkspaceStore.getState().addFlow("Test Flow");

      useSearchStore.getState().search("inject");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].nodeId).toBe("n1");
      expect(results[0].nodeLabel).toBe("My Inject Node");
      expect(results[0].matchField).toBe("name");
    });

    it("finds results by node type (case-insensitive)", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "debug", "My Debug")],
      });
      useWorkspaceStore.getState().addFlow("Flow 1");

      useSearchStore.getState().search("debug");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].nodeId).toBe("n1");
    });

    it("finds results by property value", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "function", "Func", { func: "return msg.payload;" })],
      });
      useWorkspaceStore.getState().addFlow("Flow 1");

      useSearchStore.getState().search("payload");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].nodeId).toBe("n1");
      expect(results[0].matchField).toBe("func");
    });

    it("searches case-insensitively", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "UpperCase")],
      });
      useWorkspaceStore.getState().addFlow("Flow");

      useSearchStore.getState().search("uppercase");
      expect(useSearchStore.getState().results).toHaveLength(1);

      useSearchStore.getState().search("UPPERCASE");
      expect(useSearchStore.getState().results).toHaveLength(1);

      useSearchStore.getState().search("upperCase");
      expect(useSearchStore.getState().results).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // search - multiple nodes and flows
  // -----------------------------------------------------------------------

  describe("search across multiple nodes", () => {
    it("finds multiple matching nodes", () => {
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "Inject A"),
          makeNode("n2", "inject", "Inject B"),
          makeNode("n3", "debug", "Debug A"),
        ],
      });
      useWorkspaceStore.getState().addFlow("Flow 1");

      useSearchStore.getState().search("inject");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(2);
      expect(results.map((r) => r.nodeId).sort()).toEqual(["n1", "n2"]);
    });

    it("deduplicates results by nodeId", () => {
      // A node matching on both name and type should only appear once
      useFlowStore.setState({
        nodes: [makeNode("n1", "debug", "debug")],
      });
      useWorkspaceStore.getState().addFlow("Flow 1");

      useSearchStore.getState().search("debug");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].nodeId).toBe("n1");
    });

    it("includes flow label in results", () => {
      const flowId = useWorkspaceStore.getState().addFlow("My Custom Flow");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Test")],
      });
      // Manually set z to the flow
      useFlowStore.setState({
        nodes: [{ ...useFlowStore.getState().nodes[0], data: { ...useFlowStore.getState().nodes[0].data, z: flowId } }],
      });

      useSearchStore.getState().search("test");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].flowLabel).toBe("My Custom Flow");
    });
  });

  // -----------------------------------------------------------------------
  // search - property search
  // -----------------------------------------------------------------------

  describe("property value search", () => {
    it("finds results in numeric property values", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "delay", "Delay", { timeout: 5000 })],
      });
      useWorkspaceStore.getState().addFlow("Flow");

      useSearchStore.getState().search("5000");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].matchField).toBe("timeout");
    });

    it("skips null and undefined property values", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Inject", { topic: null, payload: undefined })],
      });
      useWorkspaceStore.getState().addFlow("Flow");

      useSearchStore.getState().search("null");
      expect(useSearchStore.getState().results).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Navigation
  // -----------------------------------------------------------------------

  describe("selectResult()", () => {
    it("selects a result by index", () => {
      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "debug", "B"),
        ],
      });

      useSearchStore.getState().search("a");
      // Need another search to get multiple results - let's search broader
      resetStores();
      const fid = useWorkspaceStore.getState().addFlow("Flow 1");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "AAA"),
          makeNode("n2", "inject", "BBB"),
        ],
      });
      useSearchStore.getState().search("inject");
      expect(useSearchStore.getState().results).toHaveLength(2);

      useSearchStore.getState().selectResult(1);
      expect(useSearchStore.getState().selectedIndex).toBe(1);
    });

    it("ignores out-of-bounds indices", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "A")],
      });
      useWorkspaceStore.getState().addFlow("Flow");
      useSearchStore.getState().search("a");
      expect(useSearchStore.getState().results).toHaveLength(1);

      useSearchStore.getState().selectResult(5);
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });

    it("switches to the correct flow tab", () => {
      const flow1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const flow2 = useWorkspaceStore.getState().addFlow("Flow 2");

      useFlowStore.setState({
        nodes: [
          { ...makeNode("n1", "inject", "Node 1"), data: { ...makeNode("n1", "inject", "Node 1").data, z: flow2 } },
        ],
      });

      useSearchStore.getState().search("node");
      useSearchStore.getState().selectResult(0);

      expect(useWorkspaceStore.getState().activeFlowId).toBe(flow2);
    });

    it("emits search:result-selected event", () => {
      const handler = vi.fn();
      const unsub = eventBus.on("search:result-selected", handler);

      const flowId = useWorkspaceStore.getState().addFlow("Flow 1");
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Test")],
      });

      useSearchStore.getState().search("test");
      useSearchStore.getState().selectResult(0);

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({
        nodeId: "n1",
        flowId: expect.any(String),
      });

      unsub();
    });
  });

  // -----------------------------------------------------------------------
  // nextResult / prevResult
  // -----------------------------------------------------------------------

  describe("nextResult()", () => {
    it("moves to the next result", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "AAA"),
          makeNode("n2", "inject", "BBB"),
          makeNode("n3", "inject", "CCC"),
        ],
      });

      useSearchStore.getState().search("inject");
      expect(useSearchStore.getState().selectedIndex).toBe(0);

      useSearchStore.getState().nextResult();
      expect(useSearchStore.getState().selectedIndex).toBe(1);

      useSearchStore.getState().nextResult();
      expect(useSearchStore.getState().selectedIndex).toBe(2);
    });

    it("wraps around to first result", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });

      useSearchStore.getState().search("inject");
      useSearchStore.getState().selectResult(1);
      useSearchStore.getState().nextResult();
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });

    it("does nothing when no results", () => {
      useSearchStore.getState().nextResult();
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });
  });

  describe("prevResult()", () => {
    it("moves to the previous result", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
        ],
      });

      useSearchStore.getState().search("inject");
      useSearchStore.getState().selectResult(1);
      useSearchStore.getState().prevResult();
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });

    it("wraps around to last result", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useFlowStore.setState({
        nodes: [
          makeNode("n1", "inject", "A"),
          makeNode("n2", "inject", "B"),
          makeNode("n3", "inject", "C"),
        ],
      });

      useSearchStore.getState().search("inject");
      useSearchStore.getState().prevResult();
      expect(useSearchStore.getState().selectedIndex).toBe(2);
    });

    it("does nothing when no results", () => {
      useSearchStore.getState().prevResult();
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // clear
  // -----------------------------------------------------------------------

  describe("clear()", () => {
    it("resets query and results", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "inject", "Test")],
      });
      useWorkspaceStore.getState().addFlow("Flow");

      useSearchStore.getState().search("test");
      expect(useSearchStore.getState().results.length).toBeGreaterThan(0);

      useSearchStore.getState().clear();
      expect(useSearchStore.getState().query).toBe("");
      expect(useSearchStore.getState().results).toEqual([]);
      expect(useSearchStore.getState().selectedIndex).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // Match context
  // -----------------------------------------------------------------------

  describe("match context", () => {
    it("provides context around the match", () => {
      useFlowStore.setState({
        nodes: [makeNode("n1", "function", "My Function Node")],
      });
      useWorkspaceStore.getState().addFlow("Flow");

      useSearchStore.getState().search("function");
      const results = useSearchStore.getState().results;
      expect(results).toHaveLength(1);
      expect(results[0].matchContext).toContain("unction");
    });
  });
});
