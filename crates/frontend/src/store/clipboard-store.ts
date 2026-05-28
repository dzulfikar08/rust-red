/**
 * Clipboard store — replaces Node-RED's RED.clipboard.
 *
 * A Zustand store implementing copy/cut/paste for flow nodes and edges,
 * plus JSON flow import/export in Node-RED flow format.
 *
 * Copy serializes selected nodes and their interconnecting edges.
 * Paste generates new IDs for all nodes and remaps edges/wires accordingly,
 * offsetting node positions by 20px to avoid overlap.
 */

import { create } from "zustand";
import type { Node as RFNode, Edge as RFEdge } from "@xyflow/react";
import { useFlowStore } from "./flow-store";
import type { FlowNode } from "../red/nodes/types";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Clipboard store state */
interface ClipboardState {
  /** Nodes currently in the clipboard (deep-cloned from canvas) */
  clipboard: RFNode[] | null;
  /** Edges between clipboard nodes */
  clipboardEdges: RFEdge[] | null;

  /** Copy selected nodes (by ID) to clipboard */
  copy: (nodeIds: string[]) => void;
  /** Copy + delete from canvas */
  cut: (nodeIds: string[]) => void;
  /** Paste clipboard contents, returning pasted nodes. New IDs are generated. */
  paste: (offsetX?: number, offsetY?: number) => RFNode[];
  /** Export nodes to Node-RED JSON format string */
  exportToJson: (nodeIds?: string[]) => string;
  /** Import from Node-RED JSON format string, returning nodes and edges */
  importFromJson: (json: string) => { nodes: RFNode[]; edges: RFEdge[] };
  /** Whether the clipboard has content */
  hasClipboard: boolean;
  /** Clear clipboard */
  clear: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PASTE_OFFSET = 20;

/**
 * Generate a random hex string of the given length (matches Node-RED IDs).
 */
function generateId(length = 8): string {
  const chars = "0123456789abcdef";
  let result = "";
  for (let i = 0; i < length; i++) {
    result += chars[Math.floor(Math.random() * chars.length)];
  }
  return result;
}

/**
 * Build a map from old node ID -> new node ID for the given nodes.
 */
function buildIdMap(nodes: RFNode[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const node of nodes) {
    map.set(node.id, generateId(8));
  }
  return map;
}

/**
 * Remap all IDs in a set of nodes using the given ID map.
 * Updates node.id, positions (with offset), and any wires in data.
 */
function remapNodes(
  nodes: RFNode[],
  idMap: Map<string, string>,
  offsetX: number,
  offsetY: number,
): RFNode[] {
  return nodes.map((node) => {
    const newId = idMap.get(node.id) ?? generateId(8);
    const remapped = {
      ...node,
      id: newId,
      position: {
        x: node.position.x + offsetX,
        y: node.position.y + offsetY,
      },
      data: { ...node.data },
    };

    // Remap wires if present in data (Node-RED format)
    if (Array.isArray(remapped.data.wires)) {
      remapped.data.wires = remapped.data.wires.map((port: string[]) =>
        port.map((targetId: string) => idMap.get(targetId) ?? targetId),
      );
    }

    // Remap z (tab/flow ID) - keep the same tab
    // (clipboard nodes came from the same flow)

    return remapped;
  });
}

/**
 * Remap edges to use new node IDs and generate new edge IDs.
 * Only includes edges whose source AND target are in the pasted set.
 */
function remapEdges(
  edges: RFEdge[],
  idMap: Map<string, string>,
): RFEdge[] {
  return edges
    .filter((edge) => idMap.has(edge.source) && idMap.has(edge.target))
    .map((edge) => ({
      ...edge,
      id: `${idMap.get(edge.source)!}:${edge.sourceHandle ?? 0}->${idMap.get(edge.target)!}`,
      source: idMap.get(edge.source)!,
      target: idMap.get(edge.target)!,
    }));
}

/**
 * Convert React Flow nodes/edges to Node-RED flow JSON format.
 */
function toNodeRedJson(nodes: RFNode[], edges: RFEdge[]): string {
  // Build wire map: sourceId -> portIndex -> targetIds
  const wireMap = new Map<string, Map<number, string[]>>();
  for (const edge of edges) {
    if (!wireMap.has(edge.source)) {
      wireMap.set(edge.source, new Map());
    }
    const portMap = wireMap.get(edge.source)!;
    const portIdx = parseHandleIndex(edge.sourceHandle);
    if (!portMap.has(portIdx)) {
      portMap.set(portIdx, []);
    }
    portMap.get(portIdx)!.push(edge.target);
  }

  const flowNodes = nodes.map((node) => {
    const wires: string[][] = [];
    const nodeWires = wireMap.get(node.id);
    if (nodeWires) {
      const maxPort = Math.max(...nodeWires.keys(), -1);
      for (let i = 0; i <= maxPort; i++) {
        wires.push(nodeWires.get(i) ?? []);
      }
    }

    // Build the node-red node object
    const { data, position } = node;
    const nrNode: Record<string, unknown> = {
      id: node.id,
      type: data.type || "unknown",
      x: position.x,
      y: position.y,
      z: (data.z as string) || "",
      wires,
    };

    // Include name if present
    if (data.label && data.label !== data.type) {
      nrNode.name = data.label;
    }

    // Include other data properties, skipping internal keys
    const skipKeys = new Set([
      "label",
      "type",
      "category",
      "color",
      "icon",
      "_isConfig",
      "_users",
      "wires",
      "z",
    ]);
    for (const [key, value] of Object.entries(data)) {
      if (!skipKeys.has(key) && !(key in nrNode)) {
        nrNode[key] = value;
      }
    }

    return nrNode;
  });

  return JSON.stringify(flowNodes, null, 2);
}

/**
 * Parse a React Flow handle string like "output-0" to a numeric index.
 */
function parseHandleIndex(handle: string | undefined): number {
  if (!handle) return 0;
  const match = handle.match(/^output-(\d+)$/);
  return match ? parseInt(match[1], 10) : 0;
}

/**
 * Parse Node-RED flow JSON and return React Flow nodes and edges.
 */
function parseNodeRedJson(json: string): { nodes: RFNode[]; edges: RFEdge[] } {
  const parsed = JSON.parse(json);
  const rawNodes = Array.isArray(parsed) ? parsed : [parsed];

  const nodes: RFNode[] = [];
  const edges: RFEdge[] = [];

  for (const raw of rawNodes) {
    // Skip tab/subflow entries
    if (raw.type === "tab" || raw.type === "subflow") continue;

    const nodeId = raw.id || generateId(8);
    const nodeType = raw.type || "unknown";

    const node: RFNode = {
      id: nodeId,
      type: "nrNode",
      position: {
        x: typeof raw.x === "number" ? raw.x : 0,
        y: typeof raw.y === "number" ? raw.y : 0,
      },
      data: {
        label: raw.name || nodeType,
        type: nodeType,
        category: raw.category || "",
        color: raw.color || "",
        icon: raw.icon,
        z: raw.z || "",
        ...extractExtraProps(raw),
      },
    };
    nodes.push(node);

    // Convert wires to edges
    if (Array.isArray(raw.wires)) {
      for (let portIdx = 0; portIdx < raw.wires.length; portIdx++) {
        for (const targetId of raw.wires[portIdx]) {
          edges.push({
            id: `${nodeId}:${portIdx}->${targetId}`,
            source: nodeId,
            sourceHandle: `output-${portIdx}`,
            target: targetId,
            targetHandle: "input-0",
            type: "nrWire",
          });
        }
      }
    }
  }

  return { nodes, edges };
}

/**
 * Extract extra properties from a Node-RED node that aren't part of the
 * standard set.
 */
function extractExtraProps(raw: Record<string, unknown>): Record<string, unknown> {
  const standardKeys = new Set([
    "id", "type", "name", "x", "y", "z", "wires",
    "category", "color", "icon",
  ]);
  const extra: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(raw)) {
    if (!standardKeys.has(key)) {
      extra[key] = value;
    }
  }
  return extra;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useClipboardStore = create<ClipboardState>((set, get) => ({
  clipboard: null,
  clipboardEdges: null,
  hasClipboard: false,

  copy: (nodeIds: string[]) => {
    if (nodeIds.length === 0) return;

    const { nodes, edges } = useFlowStore.getState();
    const idSet = new Set(nodeIds);

    // Deep clone selected nodes
    const copiedNodes = nodes
      .filter((n) => idSet.has(n.id))
      .map((n) => structuredClone(n));

    // Copy edges that connect within the selected set
    const copiedEdges = edges
      .filter((e) => idSet.has(e.source) && idSet.has(e.target))
      .map((e) => structuredClone(e));

    set({
      clipboard: copiedNodes,
      clipboardEdges: copiedEdges,
      hasClipboard: copiedNodes.length > 0,
    });
  },

  cut: (nodeIds: string[]) => {
    // Copy first
    get().copy(nodeIds);

    // Then delete nodes from canvas
    const { removeNode } = useFlowStore.getState();
    for (const id of nodeIds) {
      removeNode(id);
    }
  },

  paste: (offsetX = PASTE_OFFSET, offsetY = PASTE_OFFSET) => {
    const { clipboard, clipboardEdges } = get();
    if (!clipboard || clipboard.length === 0) return [];

    // Generate new IDs for all pasted nodes
    const idMap = buildIdMap(clipboard);

    // Remap nodes with new IDs and offset positions
    const newNodes = remapNodes(clipboard, idMap, offsetX, offsetY);

    // Remap edges (only those between pasted nodes)
    const newEdges = clipboardEdges
      ? remapEdges(clipboardEdges, idMap)
      : [];

    // Add to flow store
    const { setNodes, setEdges } = useFlowStore.getState();
    const currentNodes = useFlowStore.getState().nodes;
    const currentEdges = useFlowStore.getState().edges;
    setNodes([...currentNodes, ...newNodes]);
    setEdges([...currentEdges, ...newEdges]);

    // Emit event
    eventBus.emit("flows:imported", { count: newNodes.length });

    return newNodes;
  },

  exportToJson: (nodeIds?: string[]) => {
    const { nodes, edges } = useFlowStore.getState();
    const exportNodes = nodeIds
      ? nodes.filter((n) => nodeIds.includes(n.id))
      : nodes;

    const idSet = new Set(exportNodes.map((n) => n.id));
    const exportEdges = edges.filter(
      (e) => idSet.has(e.source) && idSet.has(e.target),
    );

    return toNodeRedJson(exportNodes, exportEdges);
  },

  importFromJson: (json: string) => {
    const { nodes, edges } = parseNodeRedJson(json);

    // Generate new IDs to avoid collisions
    const idMap = buildIdMap(nodes);
    const remappedNodes = remapNodes(nodes, idMap, 0, 0);
    const remappedEdges = remapEdges(edges, idMap);

    // Add to flow store
    const { setNodes, setEdges } = useFlowStore.getState();
    const currentNodes = useFlowStore.getState().nodes;
    const currentEdges = useFlowStore.getState().edges;
    setNodes([...currentNodes, ...remappedNodes]);
    setEdges([...currentEdges, ...remappedEdges]);

    // Emit event
    eventBus.emit("flows:imported", { count: remappedNodes.length });

    return { nodes: remappedNodes, edges: remappedEdges };
  },

  clear: () => {
    set({ clipboard: null, clipboardEdges: null, hasClipboard: false });
  },
}));

export default useClipboardStore;
