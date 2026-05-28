import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  useContextMenuStore,
  initContextMenuListener,
  destroyContextMenuListener,
} from "../context-menu-store";

function resetStore() {
  useContextMenuStore.setState({
    visible: false,
    position: { x: 0, y: 0, flowX: 0, flowY: 0 },
    items: [],
    selectedNodeIds: [],
    isOnNode: false,
  });
}

describe("useContextMenuStore", () => {
  beforeEach(() => {
    resetStore();
    destroyContextMenuListener();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts hidden", () => {
      expect(useContextMenuStore.getState().visible).toBe(false);
    });

    it("starts at position (0, 0)", () => {
      const { position } = useContextMenuStore.getState();
      expect(position).toEqual({ x: 0, y: 0, flowX: 0, flowY: 0 });
    });

    it("starts with empty items", () => {
      expect(useContextMenuStore.getState().items).toEqual([]);
    });

    it("starts with empty selectedNodeIds", () => {
      expect(useContextMenuStore.getState().selectedNodeIds).toEqual([]);
    });

    it("starts with isOnNode false", () => {
      expect(useContextMenuStore.getState().isOnNode).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // show()
  // -----------------------------------------------------------------------

  describe("show()", () => {
    it("sets visible to true", () => {
      useContextMenuStore.getState().show({
        x: 100,
        y: 200,
        flowX: 50,
        flowY: 75,
        items: [{ label: "Test" }],
      });
      expect(useContextMenuStore.getState().visible).toBe(true);
    });

    it("sets position from arguments", () => {
      useContextMenuStore.getState().show({
        x: 150,
        y: 250,
        flowX: 50,
        flowY: 75,
        items: [],
      });
      expect(useContextMenuStore.getState().position).toEqual({
        x: 150,
        y: 250,
        flowX: 50,
        flowY: 75,
      });
    });

    it("stores menu items", () => {
      const items = [
        { label: "Cut", shortcut: "Ctrl-X" },
        { label: "", separator: true },
        { label: "Delete" },
      ];
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items,
      });
      expect(useContextMenuStore.getState().items).toEqual(items);
    });

    it("stores selectedNodeIds", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [],
        selectedNodeIds: ["node-1", "node-2"],
      });
      expect(useContextMenuStore.getState().selectedNodeIds).toEqual([
        "node-1",
        "node-2",
      ]);
    });

    it("defaults selectedNodeIds to empty array", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [],
      });
      expect(useContextMenuStore.getState().selectedNodeIds).toEqual([]);
    });

    it("stores isOnNode flag", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [],
        isOnNode: true,
      });
      expect(useContextMenuStore.getState().isOnNode).toBe(true);
    });

    it("defaults isOnNode to false", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [],
      });
      expect(useContextMenuStore.getState().isOnNode).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // hide()
  // -----------------------------------------------------------------------

  describe("hide()", () => {
    it("sets visible to false", () => {
      useContextMenuStore.getState().show({
        x: 100,
        y: 200,
        flowX: 0,
        flowY: 0,
        items: [{ label: "Test" }],
      });
      expect(useContextMenuStore.getState().visible).toBe(true);

      useContextMenuStore.getState().hide();
      expect(useContextMenuStore.getState().visible).toBe(false);
    });

    it("clears items", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [{ label: "Test" }],
      });
      useContextMenuStore.getState().hide();
      expect(useContextMenuStore.getState().items).toEqual([]);
    });

    it("clears selectedNodeIds", () => {
      useContextMenuStore.getState().show({
        x: 0,
        y: 0,
        flowX: 0,
        flowY: 0,
        items: [],
        selectedNodeIds: ["a", "b"],
        isOnNode: true,
      });
      useContextMenuStore.getState().hide();
      expect(useContextMenuStore.getState().selectedNodeIds).toEqual([]);
      expect(useContextMenuStore.getState().isOnNode).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // show/hide cycle
  // -----------------------------------------------------------------------

  describe("show/hide cycle", () => {
    it("can show, hide, and show again with different data", () => {
      useContextMenuStore.getState().show({
        x: 10,
        y: 20,
        flowX: 5,
        flowY: 10,
        items: [{ label: "First" }],
        selectedNodeIds: ["a"],
        isOnNode: true,
      });

      expect(useContextMenuStore.getState().visible).toBe(true);

      useContextMenuStore.getState().hide();
      expect(useContextMenuStore.getState().visible).toBe(false);

      useContextMenuStore.getState().show({
        x: 30,
        y: 40,
        flowX: 15,
        flowY: 20,
        items: [{ label: "Second" }],
        selectedNodeIds: [],
        isOnNode: false,
      });

      const state = useContextMenuStore.getState();
      expect(state.visible).toBe(true);
      expect(state.position).toEqual({ x: 30, y: 40, flowX: 15, flowY: 20 });
      expect(state.items).toEqual([{ label: "Second" }]);
    });
  });

  // -----------------------------------------------------------------------
  // Event bus listener
  // -----------------------------------------------------------------------

  describe("initContextMenuListener()", () => {
    it("does not throw when called", () => {
      expect(() => initContextMenuListener()).not.toThrow();
    });

    it("does not throw when called multiple times", () => {
      initContextMenuListener();
      initContextMenuListener();
      expect(useContextMenuStore.getState().visible).toBe(false);
    });

    it("can be destroyed without error", () => {
      initContextMenuListener();
      expect(() => destroyContextMenuListener()).not.toThrow();
    });
  });
});
