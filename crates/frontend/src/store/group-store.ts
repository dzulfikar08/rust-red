/**
 * Group store — manages visual node grouping on the canvas.
 *
 * Replaces Node-RED's RED.group (~745 lines) with a compact Zustand store.
 * Groups are rendered as labeled rectangles that enclose member nodes,
 * move their contents when dragged, and support resize.
 */

import { create } from "zustand";
import type { Node } from "@xyflow/react";
import { useFlowStore } from "./flow-store";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FlowGroupStyle {
  label?: string;
  color?: string;
  fill?: string;
  stroke?: string;
}

export interface FlowGroup {
  id: string;
  label: string;
  type: "group";
  /** IDs of nodes belonging to this group */
  nodes: string[];
  /** Position on canvas */
  x: number;
  y: number;
  /** Dimensions */
  w: number;
  h: number;
  /** Visual style overrides */
  style?: FlowGroupStyle;
}

interface GroupStore {
  /** All groups */
  groups: FlowGroup[];

  /** Create a new group from the given node IDs. Returns the new group id. */
  createGroup(nodeIds: string[]): string;

  /** Delete a group by id (does NOT delete contained nodes). */
  deleteGroup(id: string): void;

  /** Update arbitrary fields on a group. */
  updateGroup(id: string, updates: Partial<FlowGroup>): void;

  /** Add a node to an existing group. */
  addNodeToGroup(groupId: string, nodeId: string): void;

  /** Remove a node from its group. */
  removeNodeFromGroup(groupId: string, nodeId: string): void;

  /** Look up which group (if any) contains the given node. */
  getGroupForNode(nodeId: string): FlowGroup | undefined;

  /** Move a group by delta, also moving all contained nodes in the flow store. */
  moveGroup(groupId: string, deltaX: number, deltaY: number): void;

  /** Resize a group. */
  resizeGroup(groupId: string, w: number, h: number): void;

  /** Recalculate the bounding box of a group from its member nodes. */
  recalcBounds(groupId: string): void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let _idCounter = 0;

/** Generate a unique group ID. */
function generateGroupId(): string {
  return `group-${Date.now()}-${++_idCounter}`;
}

/**
 * Compute the bounding rectangle of a set of nodes, with padding.
 * Returns { x, y, w, h }.
 */
function computeBounds(
  flowNodes: Node[],
  nodeIds: string[],
  padding = 30,
): { x: number; y: number; w: number; h: number } {
  const members = flowNodes.filter((n) => nodeIds.includes(n.id));
  if (members.length === 0) {
    return { x: 0, y: 0, w: 200, h: 150 };
  }

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const n of members) {
    const nw = (n.data?.width as number) ?? 140;
    const nh = (n.data?.height as number) ?? 50;
    minX = Math.min(minX, n.position.x);
    minY = Math.min(minY, n.position.y);
    maxX = Math.max(maxX, n.position.x + nw);
    maxY = Math.max(maxY, n.position.y + nh);
  }

  return {
    x: minX - padding,
    y: minY - padding,
    w: maxX - minX + padding * 2,
    h: maxY - minY + padding * 2,
  };
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useGroupStore = create<GroupStore>((set, get) => ({
  groups: [],

  createGroup: (nodeIds) => {
    const id = generateGroupId();
    const { nodes } = useFlowStore.getState();
    const bounds = computeBounds(nodes, nodeIds);

    const group: FlowGroup = {
      id,
      label: "Group",
      type: "group",
      nodes: [...nodeIds],
      ...bounds,
      style: {},
    };

    set((state) => ({
      groups: [...state.groups, group],
    }));

    // Emit event for other subsystems
    eventBus.emit("group:created", { id, nodeIds });

    return id;
  },

  deleteGroup: (id) => {
    set((state) => ({
      groups: state.groups.filter((g) => g.id !== id),
    }));
    eventBus.emit("group:deleted", { id });
  },

  updateGroup: (id, updates) => {
    set((state) => ({
      groups: state.groups.map((g) =>
        g.id === id ? { ...g, ...updates } : g,
      ),
    }));
  },

  addNodeToGroup: (groupId, nodeId) => {
    set((state) => ({
      groups: state.groups.map((g) => {
        if (g.id !== groupId) return g;
        if (g.nodes.includes(nodeId)) return g;
        return { ...g, nodes: [...g.nodes, nodeId] };
      }),
    }));

    // Recalculate bounds after adding a node
    get().recalcBounds(groupId);
  },

  removeNodeFromGroup: (groupId, nodeId) => {
    set((state) => ({
      groups: state.groups.map((g) => {
        if (g.id !== groupId) return g;
        return { ...g, nodes: g.nodes.filter((nid) => nid !== nodeId) };
      }),
    }));
  },

  getGroupForNode: (nodeId) => {
    return get().groups.find((g) => g.nodes.includes(nodeId));
  },

  moveGroup: (groupId, deltaX, deltaY) => {
    // Move the group position
    set((state) => ({
      groups: state.groups.map((g) => {
        if (g.id !== groupId) return g;
        return {
          ...g,
          x: g.x + deltaX,
          y: g.y + deltaY,
        };
      }),
    }));

    // Move all contained nodes in the flow store
    const group = get().groups.find((g) => g.id === groupId);
    if (!group) return;

    useFlowStore.setState((state) => ({
      nodes: state.nodes.map((n) => {
        if (!group.nodes.includes(n.id)) return n;
        return {
          ...n,
          position: {
            x: n.position.x + deltaX,
            y: n.position.y + deltaY,
          },
        };
      }),
    }));
  },

  resizeGroup: (groupId, w, h) => {
    set((state) => ({
      groups: state.groups.map((g) => {
        if (g.id !== groupId) return g;
        return { ...g, w: Math.max(100, w), h: Math.max(60, h) };
      }),
    }));
  },

  recalcBounds: (groupId) => {
    const group = get().groups.find((g) => g.id === groupId);
    if (!group || group.nodes.length === 0) return;

    const { nodes } = useFlowStore.getState();
    const bounds = computeBounds(nodes, group.nodes);

    set((state) => ({
      groups: state.groups.map((g) => {
        if (g.id !== groupId) return g;
        return { ...g, ...bounds };
      }),
    }));
  },
}));

export default useGroupStore;
