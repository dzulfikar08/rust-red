import { describe, it, expect, beforeEach, vi } from "vitest";
import { useSubflowStore } from "../subflow-store";
import { eventBus } from "../../red/core/events";

function resetStore() {
  useSubflowStore.setState({
    subflows: new Map(),
  });
}

describe("useSubflowStore", () => {
  beforeEach(() => {
    resetStore();
    eventBus.clear();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with an empty subflows map", () => {
      const { subflows } = useSubflowStore.getState();
      expect(subflows.size).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // createSubflow
  // -----------------------------------------------------------------------

  describe("createSubflow()", () => {
    it("creates a subflow with default values", () => {
      const id = useSubflowStore.getState().createSubflow();
      const sf = useSubflowStore.getState().getSubflow(id);

      expect(sf).toBeDefined();
      expect(sf!.name).toBe("Subflow 1");
      expect(sf!.category).toBe("function");
      expect(sf!.color).toBe("#da9aaa");
      expect(sf!.icon).toBe("debugger");
      expect(sf!.in).toBe(1);
      expect(sf!.out).toBe(1);
      expect(sf!.nodes).toEqual([]);
      expect(sf!.env).toEqual([]);
    });

    it("creates a subflow with custom options", () => {
      const id = useSubflowStore.getState().createSubflow({
        name: "My Subflow",
        category: "network",
        color: "#ff0000",
        icon: "feed",
        in: 2,
        out: 3,
        nodes: ["n1", "n2"],
        env: [{ name: "FOO", value: "bar", type: "str" }],
      });
      const sf = useSubflowStore.getState().getSubflow(id);

      expect(sf!.name).toBe("My Subflow");
      expect(sf!.category).toBe("network");
      expect(sf!.color).toBe("#ff0000");
      expect(sf!.icon).toBe("feed");
      expect(sf!.in).toBe(2);
      expect(sf!.out).toBe(3);
      expect(sf!.nodes).toEqual(["n1", "n2"]);
      expect(sf!.env).toEqual([{ name: "FOO", value: "bar", type: "str" }]);
    });

    it("generates unique IDs", () => {
      const ids = new Set<string>();
      for (let i = 0; i < 10; i++) {
        ids.add(useSubflowStore.getState().createSubflow());
      }
      expect(ids.size).toBe(10);
    });

    it("emits subflow:created event", () => {
      const handler = vi.fn();
      eventBus.on("subflow:created", handler);

      const id = useSubflowStore.getState().createSubflow({ name: "Test" });

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({ id, name: "Test" });
    });

    it("returns the new subflow ID", () => {
      const id = useSubflowStore.getState().createSubflow();
      expect(id).toBeTruthy();
      expect(typeof id).toBe("string");
    });
  });

  // -----------------------------------------------------------------------
  // deleteSubflow
  // -----------------------------------------------------------------------

  describe("deleteSubflow()", () => {
    it("deletes an existing subflow", () => {
      const id = useSubflowStore.getState().createSubflow();
      expect(useSubflowStore.getState().subflows.size).toBe(1);

      useSubflowStore.getState().deleteSubflow(id);
      expect(useSubflowStore.getState().subflows.size).toBe(0);
      expect(useSubflowStore.getState().getSubflow(id)).toBeUndefined();
    });

    it("does nothing for non-existent subflow", () => {
      useSubflowStore.getState().createSubflow();
      useSubflowStore.getState().deleteSubflow("nonexistent");
      expect(useSubflowStore.getState().subflows.size).toBe(1);
    });

    it("emits subflow:deleted event", () => {
      const handler = vi.fn();
      eventBus.on("subflow:deleted", handler);

      const id = useSubflowStore.getState().createSubflow({ name: "ToRemove" });
      useSubflowStore.getState().deleteSubflow(id);

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({ id, name: "ToRemove" });
    });

    it("does not emit event for non-existent subflow", () => {
      const handler = vi.fn();
      eventBus.on("subflow:deleted", handler);

      useSubflowStore.getState().deleteSubflow("nonexistent");
      expect(handler).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // updateSubflow
  // -----------------------------------------------------------------------

  describe("updateSubflow()", () => {
    it("updates specified fields on a subflow", () => {
      const id = useSubflowStore.getState().createSubflow({ name: "Original" });

      useSubflowStore.getState().updateSubflow(id, { name: "Updated", color: "#00ff00" });

      const sf = useSubflowStore.getState().getSubflow(id);
      expect(sf!.name).toBe("Updated");
      expect(sf!.color).toBe("#00ff00");
    });

    it("preserves non-updated fields", () => {
      const id = useSubflowStore.getState().createSubflow({
        name: "Test",
        category: "function",
        icon: "cog",
      });

      useSubflowStore.getState().updateSubflow(id, { name: "Renamed" });

      const sf = useSubflowStore.getState().getSubflow(id);
      expect(sf!.name).toBe("Renamed");
      expect(sf!.category).toBe("function");
      expect(sf!.icon).toBe("cog");
    });

    it("does nothing for non-existent subflow", () => {
      expect(() =>
        useSubflowStore.getState().updateSubflow("nonexistent", { name: "X" })
      ).not.toThrow();
    });

    it("emits subflow:changed event", () => {
      const handler = vi.fn();
      eventBus.on("subflow:changed", handler);

      const id = useSubflowStore.getState().createSubflow();
      useSubflowStore.getState().updateSubflow(id, { name: "New Name" });

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({ id });
    });
  });

  // -----------------------------------------------------------------------
  // getSubflow
  // -----------------------------------------------------------------------

  describe("getSubflow()", () => {
    it("returns the subflow definition for a valid ID", () => {
      const id = useSubflowStore.getState().createSubflow({ name: "FindMe" });
      const sf = useSubflowStore.getState().getSubflow(id);
      expect(sf).toBeDefined();
      expect(sf!.name).toBe("FindMe");
    });

    it("returns undefined for an unknown ID", () => {
      expect(useSubflowStore.getState().getSubflow("nope")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // convertToSubflow
  // -----------------------------------------------------------------------

  describe("convertToSubflow()", () => {
    it("creates a subflow from node IDs", () => {
      const result = useSubflowStore.getState().convertToSubflow(["n1", "n2", "n3"]);

      expect(result.subflow).toBeDefined();
      expect(result.subflow.nodes).toEqual(["n1", "n2", "n3"]);
      expect(result.movedNodeIds).toEqual(["n1", "n2", "n3"]);
    });

    it("returns a valid subflow ID that can be looked up", () => {
      const result = useSubflowStore.getState().convertToSubflow(["a", "b"]);
      const sf = useSubflowStore.getState().getSubflow(result.subflow.id);
      expect(sf).toBeDefined();
      expect(sf!.nodes).toEqual(["a", "b"]);
    });

    it("auto-generates a name", () => {
      const result = useSubflowStore.getState().convertToSubflow(["n1"]);
      expect(result.subflow.name).toMatch(/^Subflow \d+$/);
    });

    it("emits subflow:converted event", () => {
      const handler = vi.fn();
      eventBus.on("subflow:converted", handler);

      useSubflowStore.getState().convertToSubflow(["n1", "n2"]);

      expect(handler).toHaveBeenCalledOnce();
      expect(handler).toHaveBeenCalledWith({
        id: expect.any(String),
        nodeIds: ["n1", "n2"],
      });
    });

    it("works with empty node list", () => {
      const result = useSubflowStore.getState().convertToSubflow([]);
      expect(result.subflow.nodes).toEqual([]);
      expect(result.movedNodeIds).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // getAllSubflows
  // -----------------------------------------------------------------------

  describe("getAllSubflows()", () => {
    it("returns empty array when no subflows exist", () => {
      expect(useSubflowStore.getState().getAllSubflows()).toEqual([]);
    });

    it("returns all created subflows", () => {
      useSubflowStore.getState().createSubflow({ name: "A" });
      useSubflowStore.getState().createSubflow({ name: "B" });
      useSubflowStore.getState().createSubflow({ name: "C" });

      const all = useSubflowStore.getState().getAllSubflows();
      expect(all).toHaveLength(3);
      expect(all.map((s) => s.name)).toEqual(["A", "B", "C"]);
    });
  });
});
