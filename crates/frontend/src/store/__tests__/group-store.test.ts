/**
 * Tests for group-store
 *
 * Tests CRUD operations, adding/removing nodes, group lookup, move, resize,
 * and bounds recalculation.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { useGroupStore } from "../group-store";
import { eventBus } from "../../red/core/events";

// ---------------------------------------------------------------------------
// Mock flow-store so we don't need the real React Flow dependency
// ---------------------------------------------------------------------------

const mockNodes = [
  { id: "n1", position: { x: 100, y: 100 }, data: { width: 140, height: 50 } },
  { id: "n2", position: { x: 300, y: 200 }, data: { width: 140, height: 50 } },
  { id: "n3", position: { x: 500, y: 300 }, data: { width: 140, height: 50 } },
];

// We need to access mockSetState from tests, so use vi.hoisted to create
// the mock function before vi.mock factory runs.
const { mockSetState } = vi.hoisted(() => ({
  mockSetState: vi.fn(),
}));

vi.mock("../flow-store", () => ({
  useFlowStore: {
    getState: () => ({
      nodes: mockNodes,
    }),
    setState: mockSetState,
  },
}));

// Mock event bus
vi.mock("../../red/core/events", () => ({
  eventBus: {
    emit: vi.fn(),
    on: vi.fn(() => vi.fn()),
  },
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useGroupStore.setState({ groups: [] });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useGroupStore", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty groups array", () => {
      expect(useGroupStore.getState().groups).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // createGroup
  // -----------------------------------------------------------------------

  describe("createGroup()", () => {
    it("creates a group with the given node IDs", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2"]);
      const group = useGroupStore.getState().groups.find((g) => g.id === id);

      expect(group).toBeDefined();
      expect(group!.type).toBe("group");
      expect(group!.nodes).toEqual(["n1", "n2"]);
      expect(group!.label).toBe("Group");
    });

    it("returns a unique group id", () => {
      const id1 = useGroupStore.getState().createGroup(["n1"]);
      const id2 = useGroupStore.getState().createGroup(["n2"]);
      expect(id1).not.toBe(id2);
    });

    it("computes bounding box from member nodes", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2"]);
      const group = useGroupStore.getState().groups.find((g) => g.id === id);

      // n1 at (100,100) 140x50, n2 at (300,200) 140x50
      // With default 30px padding:
      // x = 100 - 30 = 70, y = 100 - 30 = 70
      // maxX = 300 + 140 = 440, maxY = 200 + 50 = 250
      // w = 440 - 100 + 60 = 400, h = 250 - 100 + 60 = 210
      expect(group!.x).toBe(70);
      expect(group!.y).toBe(70);
      expect(group!.w).toBe(400);
      expect(group!.h).toBe(210);
    });

    it("handles empty node IDs with default bounds", () => {
      const id = useGroupStore.getState().createGroup([]);
      const group = useGroupStore.getState().groups.find((g) => g.id === id);

      expect(group).toBeDefined();
      expect(group!.w).toBe(200);
      expect(group!.h).toBe(150);
    });

    it("emits group:created event", () => {
      useGroupStore.getState().createGroup(["n1"]);

      expect(eventBus.emit).toHaveBeenCalledWith(
        "group:created",
        expect.objectContaining({ nodeIds: ["n1"] }),
      );
    });
  });

  // -----------------------------------------------------------------------
  // deleteGroup
  // -----------------------------------------------------------------------

  describe("deleteGroup()", () => {
    it("removes a group by id", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2"]);
      expect(useGroupStore.getState().groups).toHaveLength(1);

      useGroupStore.getState().deleteGroup(id);
      expect(useGroupStore.getState().groups).toHaveLength(0);
    });

    it("does not affect other groups", () => {
      const id1 = useGroupStore.getState().createGroup(["n1"]);
      const id2 = useGroupStore.getState().createGroup(["n2"]);

      useGroupStore.getState().deleteGroup(id1);
      expect(useGroupStore.getState().groups).toHaveLength(1);
      expect(useGroupStore.getState().groups[0].id).toBe(id2);
    });

    it("emits group:deleted event", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      vi.clearAllMocks();

      useGroupStore.getState().deleteGroup(id);
      expect(eventBus.emit).toHaveBeenCalledWith(
        "group:deleted",
        expect.objectContaining({ id }),
      );
    });
  });

  // -----------------------------------------------------------------------
  // updateGroup
  // -----------------------------------------------------------------------

  describe("updateGroup()", () => {
    it("updates label on a group", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().updateGroup(id, { label: "My Group" });

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.label).toBe("My Group");
    });

    it("updates style on a group", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().updateGroup(id, {
        style: { fill: "#ff0000", stroke: "#00ff00" },
      });

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.style?.fill).toBe("#ff0000");
      expect(group!.style?.stroke).toBe("#00ff00");
    });

    it("does not modify other groups", () => {
      const id1 = useGroupStore.getState().createGroup(["n1"]);
      const id2 = useGroupStore.getState().createGroup(["n2"]);

      useGroupStore.getState().updateGroup(id1, { label: "G1" });
      const g2 = useGroupStore.getState().groups.find((g) => g.id === id2);
      expect(g2!.label).toBe("Group");
    });
  });

  // -----------------------------------------------------------------------
  // addNodeToGroup
  // -----------------------------------------------------------------------

  describe("addNodeToGroup()", () => {
    it("adds a node to an existing group", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().addNodeToGroup(id, "n3");

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.nodes).toContain("n3");
      expect(group!.nodes).toHaveLength(2);
    });

    it("does not add duplicate nodes", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().addNodeToGroup(id, "n1");

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.nodes).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // removeNodeFromGroup
  // -----------------------------------------------------------------------

  describe("removeNodeFromGroup()", () => {
    it("removes a node from its group", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2", "n3"]);
      useGroupStore.getState().removeNodeFromGroup(id, "n2");

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.nodes).toEqual(["n1", "n3"]);
    });

    it("handles removing non-existent node gracefully", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().removeNodeFromGroup(id, "n99");

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.nodes).toEqual(["n1"]);
    });
  });

  // -----------------------------------------------------------------------
  // getGroupForNode
  // -----------------------------------------------------------------------

  describe("getGroupForNode()", () => {
    it("returns the group containing the given node", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2"]);
      const result = useGroupStore.getState().getGroupForNode("n1");
      expect(result!.id).toBe(id);
    });

    it("returns undefined for a node not in any group", () => {
      useGroupStore.getState().createGroup(["n1"]);
      const result = useGroupStore.getState().getGroupForNode("n3");
      expect(result).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // moveGroup
  // -----------------------------------------------------------------------

  describe("moveGroup()", () => {
    it("updates group position by delta", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      const before = useGroupStore.getState().groups.find((g) => g.id === id)!;

      useGroupStore.getState().moveGroup(id, 50, 30);
      const after = useGroupStore.getState().groups.find((g) => g.id === id)!;

      expect(after.x).toBe(before.x + 50);
      expect(after.y).toBe(before.y + 30);
    });

    it("calls flow store to move contained nodes", () => {
      const id = useGroupStore.getState().createGroup(["n1", "n2"]);
      vi.clearAllMocks();

      useGroupStore.getState().moveGroup(id, 10, 20);

      expect(mockSetState).toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // resizeGroup
  // -----------------------------------------------------------------------

  describe("resizeGroup()", () => {
    it("updates group dimensions", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().resizeGroup(id, 500, 400);

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.w).toBe(500);
      expect(group!.h).toBe(400);
    });

    it("enforces minimum width of 100", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().resizeGroup(id, 50, 100);

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.w).toBe(100);
    });

    it("enforces minimum height of 60", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      useGroupStore.getState().resizeGroup(id, 200, 30);

      const group = useGroupStore.getState().groups.find((g) => g.id === id);
      expect(group!.h).toBe(60);
    });
  });

  // -----------------------------------------------------------------------
  // recalcBounds
  // -----------------------------------------------------------------------

  describe("recalcBounds()", () => {
    it("recalculates bounds from member nodes", () => {
      const id = useGroupStore.getState().createGroup(["n1"]);
      // Manually resize the group to something wrong
      useGroupStore.getState().updateGroup(id, { w: 1, h: 1 });

      useGroupStore.getState().recalcBounds(id);
      const group = useGroupStore.getState().groups.find((g) => g.id === id);

      // Should be recalculated from n1 at (100,100) 140x50 with 30px padding
      expect(group!.w).toBe(200);
      expect(group!.h).toBe(110);
    });
  });
});
