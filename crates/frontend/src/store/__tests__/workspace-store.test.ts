import { describe, it, expect, beforeEach, vi } from "vitest";
import { useWorkspaceStore } from "../workspace-store";
import { eventBus } from "../../red/core/events";

function resetStore() {
  useWorkspaceStore.setState({
    flows: [],
    activeFlowId: null,
  });
}

describe("useWorkspaceStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty flows and no active flow", () => {
      const state = useWorkspaceStore.getState();
      expect(state.flows).toEqual([]);
      expect(state.activeFlowId).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // addFlow
  // -----------------------------------------------------------------------

  describe("addFlow()", () => {
    it("adds a flow with a default label", () => {
      const id = useWorkspaceStore.getState().addFlow();
      const state = useWorkspaceStore.getState();
      expect(state.flows).toHaveLength(1);
      expect(state.flows[0].id).toBe(id);
      expect(state.flows[0].label).toBe("Flow 1");
      expect(state.flows[0].type).toBe("tab");
      expect(state.flows[0].disabled).toBe(false);
    });

    it("adds a flow with a custom label", () => {
      const id = useWorkspaceStore.getState().addFlow("My Flow");
      const state = useWorkspaceStore.getState();
      expect(state.flows[0].label).toBe("My Flow");
      expect(state.flows[0].id).toBe(id);
    });

    it("auto-activates the first flow added", () => {
      const id = useWorkspaceStore.getState().addFlow();
      expect(useWorkspaceStore.getState().activeFlowId).toBe(id);
    });

    it("does not change active flow when adding additional flows", () => {
      const id1 = useWorkspaceStore.getState().addFlow("First");
      const id2 = useWorkspaceStore.getState().addFlow("Second");
      expect(useWorkspaceStore.getState().activeFlowId).toBe(id1);
      expect(useWorkspaceStore.getState().flows).toHaveLength(2);
      // id2 exists but is not active
      expect(useWorkspaceStore.getState().flows[1].id).toBe(id2);
    });

    it("generates unique IDs for each flow", () => {
      const ids = new Set<string>();
      for (let i = 0; i < 10; i++) {
        ids.add(useWorkspaceStore.getState().addFlow());
      }
      expect(ids.size).toBe(10);
    });

    it("increments default label counter", () => {
      useWorkspaceStore.getState().addFlow();
      useWorkspaceStore.getState().addFlow();
      useWorkspaceStore.getState().addFlow();
      const state = useWorkspaceStore.getState();
      expect(state.flows[0].label).toBe("Flow 1");
      expect(state.flows[1].label).toBe("Flow 2");
      expect(state.flows[2].label).toBe("Flow 3");
    });
  });

  // -----------------------------------------------------------------------
  // removeFlow
  // -----------------------------------------------------------------------

  describe("removeFlow()", () => {
    it("removes a flow by id", () => {
      const id = useWorkspaceStore.getState().addFlow("To Remove");
      useWorkspaceStore.getState().removeFlow(id);
      expect(useWorkspaceStore.getState().flows).toHaveLength(0);
    });

    it("switches to another flow if the active flow is removed", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const id2 = useWorkspaceStore.getState().addFlow("Flow 2");
      // Active is id1 (first flow)
      useWorkspaceStore.getState().removeFlow(id1);
      expect(useWorkspaceStore.getState().activeFlowId).toBe(id2);
    });

    it("sets activeFlowId to null if last flow is removed", () => {
      const id = useWorkspaceStore.getState().addFlow("Only Flow");
      useWorkspaceStore.getState().removeFlow(id);
      expect(useWorkspaceStore.getState().activeFlowId).toBeNull();
      expect(useWorkspaceStore.getState().flows).toHaveLength(0);
    });

    it("does not change active flow when removing a non-active flow", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Active");
      const id2 = useWorkspaceStore.getState().addFlow("Inactive");
      useWorkspaceStore.getState().removeFlow(id2);
      expect(useWorkspaceStore.getState().activeFlowId).toBe(id1);
    });

    it("handles removing non-existent flow gracefully", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      useWorkspaceStore.getState().removeFlow("non-existent");
      expect(useWorkspaceStore.getState().flows).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // renameFlow
  // -----------------------------------------------------------------------

  describe("renameFlow()", () => {
    it("renames a flow", () => {
      const id = useWorkspaceStore.getState().addFlow("Old Name");
      useWorkspaceStore.getState().renameFlow(id, "New Name");
      expect(useWorkspaceStore.getState().flows[0].label).toBe("New Name");
    });

    it("does not affect other flows", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Flow 1");
      useWorkspaceStore.getState().addFlow("Flow 2");
      useWorkspaceStore.getState().renameFlow(id1, "Renamed");
      expect(useWorkspaceStore.getState().flows[0].label).toBe("Renamed");
      expect(useWorkspaceStore.getState().flows[1].label).toBe("Flow 2");
    });
  });

  // -----------------------------------------------------------------------
  // setActiveFlow
  // -----------------------------------------------------------------------

  describe("setActiveFlow()", () => {
    it("sets the active flow", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const id2 = useWorkspaceStore.getState().addFlow("Flow 2");
      useWorkspaceStore.getState().setActiveFlow(id2);
      expect(useWorkspaceStore.getState().activeFlowId).toBe(id2);
    });

    it("emits workspace:changed event when switching", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const id2 = useWorkspaceStore.getState().addFlow("Flow 2");

      const handler = vi.fn();
      const unsub = eventBus.on("workspace:changed", handler);

      useWorkspaceStore.getState().setActiveFlow(id2);

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({ id: id2 });

      unsub();
    });

    it("does not emit event when setting same active flow", () => {
      const id = useWorkspaceStore.getState().addFlow("Flow");

      const handler = vi.fn();
      const unsub = eventBus.on("workspace:changed", handler);

      useWorkspaceStore.getState().setActiveFlow(id);

      expect(handler).not.toHaveBeenCalled();

      unsub();
    });
  });

  // -----------------------------------------------------------------------
  // getActiveFlow
  // -----------------------------------------------------------------------

  describe("getActiveFlow()", () => {
    it("returns undefined when no active flow", () => {
      expect(useWorkspaceStore.getState().getActiveFlow()).toBeUndefined();
    });

    it("returns the active flow object", () => {
      const id = useWorkspaceStore.getState().addFlow("Active Flow");
      const flow = useWorkspaceStore.getState().getActiveFlow();
      expect(flow).toBeDefined();
      expect(flow!.id).toBe(id);
      expect(flow!.label).toBe("Active Flow");
    });

    it("returns updated flow after rename", () => {
      const id = useWorkspaceStore.getState().addFlow("Original");
      useWorkspaceStore.getState().renameFlow(id, "Renamed");
      const flow = useWorkspaceStore.getState().getActiveFlow();
      expect(flow!.label).toBe("Renamed");
    });
  });

  // -----------------------------------------------------------------------
  // reorderFlows
  // -----------------------------------------------------------------------

  describe("reorderFlows()", () => {
    it("moves a flow from one index to another", () => {
      useWorkspaceStore.getState().addFlow("A");
      useWorkspaceStore.getState().addFlow("B");
      useWorkspaceStore.getState().addFlow("C");
      // [A, B, C] -> move A (index 0) to index 2 -> [B, C, A]
      useWorkspaceStore.getState().reorderFlows(0, 2);
      const state = useWorkspaceStore.getState();
      expect(state.flows.map((f) => f.label)).toEqual(["B", "C", "A"]);
    });

    it("moves a flow backwards", () => {
      useWorkspaceStore.getState().addFlow("A");
      useWorkspaceStore.getState().addFlow("B");
      useWorkspaceStore.getState().addFlow("C");
      // [A, B, C] -> move C (index 2) to index 0 -> [C, A, B]
      useWorkspaceStore.getState().reorderFlows(2, 0);
      const state = useWorkspaceStore.getState();
      expect(state.flows.map((f) => f.label)).toEqual(["C", "A", "B"]);
    });

    it("preserves active flow after reorder", () => {
      const idA = useWorkspaceStore.getState().addFlow("A");
      useWorkspaceStore.getState().addFlow("B");
      useWorkspaceStore.getState().reorderFlows(0, 1);
      expect(useWorkspaceStore.getState().activeFlowId).toBe(idA);
    });

    it("no-op when from and to are the same", () => {
      useWorkspaceStore.getState().addFlow("A");
      useWorkspaceStore.getState().addFlow("B");
      useWorkspaceStore.getState().reorderFlows(0, 0);
      expect(useWorkspaceStore.getState().flows.map((f) => f.label)).toEqual(["A", "B"]);
    });
  });

  // -----------------------------------------------------------------------
  // setFlowDisabled
  // -----------------------------------------------------------------------

  describe("setFlowDisabled()", () => {
    it("disables a flow", () => {
      const id = useWorkspaceStore.getState().addFlow("Flow");
      useWorkspaceStore.getState().setFlowDisabled(id, true);
      expect(useWorkspaceStore.getState().flows[0].disabled).toBe(true);
    });

    it("enables a flow", () => {
      const id = useWorkspaceStore.getState().addFlow("Flow");
      useWorkspaceStore.getState().setFlowDisabled(id, true);
      useWorkspaceStore.getState().setFlowDisabled(id, false);
      expect(useWorkspaceStore.getState().flows[0].disabled).toBe(false);
    });
  });
});
