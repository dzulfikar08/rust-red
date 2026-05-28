import { describe, it, expect, beforeEach } from "vitest";
import { useHistoryStore } from "../history-store";
import type { HistoryAction, WireData, EdgeData } from "../history-store";
import type { FlowNode } from "../../red/nodes/types";

/**
 * Helper: reset the history store to a clean state.
 */
function resetStore() {
  useHistoryStore.setState({
    undoStack: [],
    redoStack: [],
    maxHistory: 50,
    canUndo: false,
    canRedo: false,
  });
}

/** Convenience factory for a minimal FlowNode. */
function makeNode(overrides: Partial<FlowNode> = {}): FlowNode {
  return {
    id: "node-1",
    type: "inject",
    x: 100,
    y: 200,
    z: "tab-1",
    wires: [],
    ...overrides,
  };
}

describe("useHistoryStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty stacks", () => {
      const state = useHistoryStore.getState();
      expect(state.undoStack).toEqual([]);
      expect(state.redoStack).toEqual([]);
      expect(state.maxHistory).toBe(50);
    });

    it("canUndo and canRedo are false initially", () => {
      const state = useHistoryStore.getState();
      expect(state.canUndo).toBe(false);
      expect(state.canRedo).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // push
  // -----------------------------------------------------------------------

  describe("push()", () => {
    it("adds an action to the undo stack", () => {
      const action: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action);
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().undoStack[0]).toEqual(action);
    });

    it("clears the redo stack on push", () => {
      const action1: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action1);

      // Simulate undo so redo stack has an entry
      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().redoStack).toHaveLength(1);

      // Pushing a new action should clear redo
      const action2: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-2" }),
      };
      useHistoryStore.getState().push(action2);
      expect(useHistoryStore.getState().redoStack).toHaveLength(0);
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().undoStack[0]).toEqual(action2);
    });

    it("enforces maxHistory limit by dropping oldest entries", () => {
      useHistoryStore.setState({ maxHistory: 3 });

      for (let i = 0; i < 5; i++) {
        useHistoryStore.getState().push({
          type: "node:add",
          node: makeNode({ id: `node-${i}` }),
        });
      }

      const state = useHistoryStore.getState();
      expect(state.undoStack).toHaveLength(3);
      // Oldest entries (0, 1) should have been dropped
      expect(state.undoStack[0]).toEqual({
        type: "node:add",
        node: makeNode({ id: "node-2" }),
      });
      expect(state.undoStack[2]).toEqual({
        type: "node:add",
        node: makeNode({ id: "node-4" }),
      });
    });
  });

  // -----------------------------------------------------------------------
  // undo
  // -----------------------------------------------------------------------

  describe("undo()", () => {
    it("returns null when undo stack is empty", () => {
      expect(useHistoryStore.getState().undo()).toBeNull();
    });

    it("pops from undo stack and returns the action", () => {
      const action: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action);

      const result = useHistoryStore.getState().undo();
      expect(result).toEqual(action);
      expect(useHistoryStore.getState().undoStack).toHaveLength(0);
    });

    it("pushes the undone action onto the redo stack", () => {
      const action: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action);

      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().redoStack).toHaveLength(1);
      expect(useHistoryStore.getState().redoStack[0]).toEqual(action);
    });

    it("handles multiple undo operations in LIFO order", () => {
      const action1: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-1" }),
      };
      const action2: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-2" }),
      };
      useHistoryStore.getState().push(action1);
      useHistoryStore.getState().push(action2);

      // Last-in-first-out
      const undone2 = useHistoryStore.getState().undo();
      expect(undone2).toEqual(action2);

      const undone1 = useHistoryStore.getState().undo();
      expect(undone1).toEqual(action1);
    });
  });

  // -----------------------------------------------------------------------
  // redo
  // -----------------------------------------------------------------------

  describe("redo()", () => {
    it("returns null when redo stack is empty", () => {
      expect(useHistoryStore.getState().redo()).toBeNull();
    });

    it("pops from redo stack and returns the action", () => {
      const action: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action);
      useHistoryStore.getState().undo();

      const result = useHistoryStore.getState().redo();
      expect(result).toEqual(action);
      expect(useHistoryStore.getState().redoStack).toHaveLength(0);
    });

    it("pushes the redone action back onto the undo stack", () => {
      const action: HistoryAction = { type: "node:add", node: makeNode() };
      useHistoryStore.getState().push(action);
      useHistoryStore.getState().undo();

      useHistoryStore.getState().redo();
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().undoStack[0]).toEqual(action);
    });

    it("handles multiple redo operations in LIFO order", () => {
      const action1: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-1" }),
      };
      const action2: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-2" }),
      };
      useHistoryStore.getState().push(action1);
      useHistoryStore.getState().push(action2);

      // Undo both: redo stack becomes [action2, action1]
      useHistoryStore.getState().undo();
      useHistoryStore.getState().undo();

      // Redo both — LIFO from redo stack: action1 first, then action2
      const redone1 = useHistoryStore.getState().redo();
      expect(redone1).toEqual(action1);

      const redone2 = useHistoryStore.getState().redo();
      expect(redone2).toEqual(action2);
    });
  });

  // -----------------------------------------------------------------------
  // Push clears redo stack (standard undo/redo behaviour)
  // -----------------------------------------------------------------------

  describe("push clears redo stack", () => {
    it("a new push after undo clears the redo stack", () => {
      const action1: HistoryAction = {
        type: "node:add",
        node: makeNode({ id: "node-1" }),
      };
      useHistoryStore.getState().push(action1);
      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().redoStack).toHaveLength(1);

      // New action should clear redo
      const action2: HistoryAction = {
        type: "node:delete",
        node: makeNode({ id: "node-3" }),
      };
      useHistoryStore.getState().push(action2);

      expect(useHistoryStore.getState().redoStack).toHaveLength(0);
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().undoStack[0]).toEqual(action2);
    });
  });

  // -----------------------------------------------------------------------
  // Multi-action push and undo
  // -----------------------------------------------------------------------

  describe("multi-action", () => {
    it("pushes a multi action as a single undo step", () => {
      const multiAction: HistoryAction = {
        type: "multi",
        actions: [
          { type: "node:add", node: makeNode({ id: "n1" }) },
          { type: "node:add", node: makeNode({ id: "n2" }) },
          {
            type: "wire:add",
            edge: { source: "n1", sourcePort: 0, target: "n2" },
          },
        ],
      };

      useHistoryStore.getState().push(multiAction);
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().undoStack[0]).toEqual(multiAction);
    });

    it("undoes a multi action as a single step", () => {
      const multiAction: HistoryAction = {
        type: "multi",
        actions: [
          { type: "node:add", node: makeNode({ id: "n1" }) },
          { type: "node:add", node: makeNode({ id: "n2" }) },
        ],
      };

      useHistoryStore.getState().push(multiAction);
      const undone = useHistoryStore.getState().undo();

      expect(undone).toEqual(multiAction);
      expect(undone!.type).toBe("multi");
      expect((undone as { type: "multi"; actions: HistoryAction[] }).actions)
        .toHaveLength(2);
      expect(useHistoryStore.getState().undoStack).toHaveLength(0);
      expect(useHistoryStore.getState().redoStack).toHaveLength(1);
    });

    it("redoes a multi action as a single step", () => {
      const multiAction: HistoryAction = {
        type: "multi",
        actions: [{ type: "node:add", node: makeNode({ id: "n1" }) }],
      };

      useHistoryStore.getState().push(multiAction);
      useHistoryStore.getState().undo();

      const redone = useHistoryStore.getState().redo();
      expect(redone).toEqual(multiAction);
      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().redoStack).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Various action types
  // -----------------------------------------------------------------------

  describe("various action types", () => {
    it("handles node:move", () => {
      const action: HistoryAction = {
        type: "node:move",
        node: makeNode(),
        oldX: 100,
        oldY: 200,
        newX: 300,
        newY: 400,
      };
      useHistoryStore.getState().push(action);
      const undone = useHistoryStore.getState().undo();
      expect(undone).toEqual(action);
    });

    it("handles node:edit", () => {
      const action: HistoryAction = {
        type: "node:edit",
        node: makeNode(),
        oldProps: { name: "old" },
        newProps: { name: "new" },
      };
      useHistoryStore.getState().push(action);
      const undone = useHistoryStore.getState().undo();
      expect(undone).toEqual(action);
    });

    it("handles wire:add and wire:delete", () => {
      const edge: WireData = { source: "n1", sourcePort: 0, target: "n2" };
      const addAction: HistoryAction = { type: "wire:add", edge };
      const delAction: HistoryAction = { type: "wire:delete", edge };

      useHistoryStore.getState().push(addAction);
      useHistoryStore.getState().push(delAction);

      expect(useHistoryStore.getState().undoStack).toHaveLength(2);

      const undone = useHistoryStore.getState().undo();
      expect(undone).toEqual(delAction);
    });

    it("handles selection:delete", () => {
      const nodes = [makeNode({ id: "n1" }), makeNode({ id: "n2" })];
      const edges: EdgeData[] = [
        { source: "n1", sourcePort: 0, target: "n2" },
      ];
      const action: HistoryAction = {
        type: "selection:delete",
        nodes,
        edges,
      };
      useHistoryStore.getState().push(action);

      const undone = useHistoryStore.getState().undo();
      expect(undone!.type).toBe("selection:delete");
      expect(
        (undone as { type: "selection:delete"; nodes: FlowNode[] }).nodes,
      ).toHaveLength(2);
    });

    it("handles flow:add and flow:delete", () => {
      const flow = {
        id: "tab-1",
        label: "Flow 1",
        nodes: [],
        configs: [],
      };
      const addAction: HistoryAction = { type: "flow:add", flow };
      const delAction: HistoryAction = { type: "flow:delete", flow };

      useHistoryStore.getState().push(addAction);
      useHistoryStore.getState().push(delAction);
      expect(useHistoryStore.getState().undoStack).toHaveLength(2);
    });

    it("handles group:add and group:delete", () => {
      const group = { id: "g1", nodes: ["n1", "n2"] };
      const addAction: HistoryAction = { type: "group:add", group };
      const delAction: HistoryAction = { type: "group:delete", group };

      useHistoryStore.getState().push(addAction);
      useHistoryStore.getState().push(delAction);
      expect(useHistoryStore.getState().undoStack).toHaveLength(2);
    });
  });

  // -----------------------------------------------------------------------
  // canUndo / canRedo computed properties
  // -----------------------------------------------------------------------

  describe("canUndo / canRedo", () => {
    it("canUndo is true when undo stack has entries", () => {
      expect(useHistoryStore.getState().canUndo).toBe(false);

      useHistoryStore.getState().push({ type: "node:add", node: makeNode() });
      expect(useHistoryStore.getState().canUndo).toBe(true);
    });

    it("canRedo is true when redo stack has entries", () => {
      expect(useHistoryStore.getState().canRedo).toBe(false);

      useHistoryStore.getState().push({ type: "node:add", node: makeNode() });
      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().canRedo).toBe(true);
    });

    it("canUndo becomes false after undoing all actions", () => {
      useHistoryStore.getState().push({ type: "node:add", node: makeNode() });
      expect(useHistoryStore.getState().canUndo).toBe(true);

      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().canUndo).toBe(false);
    });

    it("canRedo becomes false after redoing all actions", () => {
      useHistoryStore.getState().push({ type: "node:add", node: makeNode() });
      useHistoryStore.getState().undo();
      useHistoryStore.getState().redo();
      expect(useHistoryStore.getState().canRedo).toBe(false);
    });

    it("canRedo becomes false after a new push", () => {
      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n1" }),
      });
      useHistoryStore.getState().undo();
      expect(useHistoryStore.getState().canRedo).toBe(true);

      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n2" }),
      });
      expect(useHistoryStore.getState().canRedo).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // clear
  // -----------------------------------------------------------------------

  describe("clear()", () => {
    it("clears both undo and redo stacks", () => {
      useHistoryStore.getState().push({ type: "node:add", node: makeNode() });
      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n2" }),
      });
      useHistoryStore.getState().undo();

      expect(useHistoryStore.getState().undoStack).toHaveLength(1);
      expect(useHistoryStore.getState().redoStack).toHaveLength(1);

      useHistoryStore.getState().clear();

      expect(useHistoryStore.getState().undoStack).toEqual([]);
      expect(useHistoryStore.getState().redoStack).toEqual([]);
      expect(useHistoryStore.getState().canUndo).toBe(false);
      expect(useHistoryStore.getState().canRedo).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // Max history limit
  // -----------------------------------------------------------------------

  describe("maxHistory limit", () => {
    it("respects custom maxHistory value", () => {
      useHistoryStore.setState({ maxHistory: 2 });

      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n1" }),
      });
      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n2" }),
      });
      useHistoryStore.getState().push({
        type: "node:add",
        node: makeNode({ id: "n3" }),
      });

      const state = useHistoryStore.getState();
      expect(state.undoStack).toHaveLength(2);
      expect(state.undoStack[0]).toEqual({
        type: "node:add",
        node: makeNode({ id: "n2" }),
      });
      expect(state.undoStack[1]).toEqual({
        type: "node:add",
        node: makeNode({ id: "n3" }),
      });
    });

    it("does not drop entries when below maxHistory", () => {
      useHistoryStore.setState({ maxHistory: 10 });

      for (let i = 0; i < 5; i++) {
        useHistoryStore.getState().push({
          type: "node:add",
          node: makeNode({ id: `n-${i}` }),
        });
      }

      expect(useHistoryStore.getState().undoStack).toHaveLength(5);
    });
  });
});
