/**
 * Flow Data Model & Conversion Utilities
 *
 * Converts between Node-RED's flow JSON format and @xyflow/react's
 * node/edge representation used by the React canvas.
 */

import type { Node as RFNode, Edge as RFEdge } from "@xyflow/react";
import type { Flow, FlowNode, ConfigNode } from "./types";

// ---------------------------------------------------------------------------
// React Flow node data shape
// ---------------------------------------------------------------------------

export interface NRNodeData {
  label: string;
  type: string;
  category: string;
  color: string;
  icon?: string;
  [key: string]: unknown;
}

export type NRFlowNode = RFNode<NRNodeData>;

// ---------------------------------------------------------------------------
// Flow  →  ReactFlow
// ---------------------------------------------------------------------------

/**
 * Convert a Node-RED `Flow` into React Flow `{ nodes, edges }`.
 */
export function flowToReactFlow(flow: Flow): {
  nodes: NRFlowNode[];
  edges: RFEdge[];
} {
  const rfNodes: NRFlowNode[] = [];
  const rfEdges: RFEdge[] = [];

  const allNodes: FlowNode[] = [
    ...flow.nodes,
    ...(flow.configs as FlowNode[]),
  ];

  for (const node of allNodes) {
    rfNodes.push(flowNodeToRFNode(node));
    rfEdges.push(...wireToEdges(node));
  }

  return { nodes: rfNodes, edges: rfEdges };
}

/**
 * Convert a single Node-RED `FlowNode` to a React Flow node.
 */
export function flowNodeToRFNode(node: FlowNode): NRFlowNode {
  return {
    id: node.id,
    type: "nrNode", // will be rendered by the custom NRFlowNodeComponent
    position: { x: node.x, y: node.y },
    data: {
      label: (node.name as string) || node.type,
      type: node.type,
      category: (node.category as string) || "",
      color: (node.color as string) || "",
      icon: node.icon as string | undefined,
      ...(node as Record<string, unknown>),
    },
  };
}

/**
 * Convert a Node-RED node's `wires` array into React Flow edge objects.
 */
export function wireToEdges(node: FlowNode): RFEdge[] {
  const edges: RFEdge[] = [];
  const wires = node.wires ?? [];

  for (let outputIdx = 0; outputIdx < wires.length; outputIdx++) {
    for (const targetId of wires[outputIdx]) {
      edges.push({
        id: `${node.id}:${outputIdx}->${targetId}`,
        source: node.id,
        sourceHandle: `output-${outputIdx}`,
        target: targetId,
        targetHandle: "input-0",
        type: "nrWire",
      });
    }
  }

  return edges;
}

// ---------------------------------------------------------------------------
// ReactFlow  →  Flow
// ---------------------------------------------------------------------------

/**
 * Convert React Flow nodes & edges back into Node-RED `FlowNode[]`.
 * Config nodes (those with no wires and not on the canvas) are separated.
 */
export function reactFlowToFlow(
  nodes: NRFlowNode[],
  edges: RFEdge[],
): { nodes: FlowNode[]; configs: ConfigNode[] } {
  const edgeMap = buildEdgeMap(edges);

  const flowNodes: FlowNode[] = [];
  const configNodes: ConfigNode[] = [];

  for (const rfNode of nodes) {
    const flowNode = rfNodeToFlowNode(rfNode, edgeMap.get(rfNode.id) ?? []);
    // Config nodes typically have 0 outputs and live outside the canvas,
    // but we treat all nodes the same here – the caller can decide.
    if (rfNode.data._isConfig) {
      configNodes.push({
        ...flowNode,
        users: (rfNode.data._users as string[]) ?? [],
      });
    } else {
      flowNodes.push(flowNode);
    }
  }

  return { nodes: flowNodes, configs: configNodes };
}

/**
 * Convert a single React Flow node back to a Node-RED FlowNode.
 */
export function rfNodeToFlowNode(
  rfNode: NRFlowNode,
  wires: string[][],
): FlowNode {
  const { data, position } = rfNode;

  // Extract only the relevant fields, spreading extra data props
  // but omitting the synthetic keys we added during conversion.
  const {
    label: _label,
    type: nodeType,
    category: _cat,
    color: _col,
    icon: _icon,
    _isConfig: _cfg,
    _users: _usr,
    ...rest
  } = data;

  const flowNode: FlowNode = {
    id: rfNode.id,
    type: nodeType,
    x: position.x,
    y: position.y,
    z: (data.z as string) || "",
    wires,
    ...rest,
  };

  // Only set name if it was explicitly provided and differs from type
  if (_label && _label !== nodeType) {
    flowNode.name = _label;
  }

  return flowNode;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build a map from source node ID → ordered array of wire arrays
 * (one per output port) from React Flow edges.
 */
function buildEdgeMap(
  edges: RFEdge[],
): Map<string, string[][]> {
  const map = new Map<string, Map<number, string[]>>();

  for (const edge of edges) {
    if (!map.has(edge.source)) {
      map.set(edge.source, new Map());
    }
    const outputMap = map.get(edge.source)!;
    const outputIdx = parseOutputIndex(edge.sourceHandle);
    if (!outputMap.has(outputIdx)) {
      outputMap.set(outputIdx, []);
    }
    outputMap.get(outputIdx)!.push(edge.target);
  }

  // Convert inner maps to arrays
  const result = new Map<string, string[][]>();
  for (const [sourceId, outputMap] of map) {
    const maxIdx = Math.max(...outputMap.keys(), -1);
    const wires: string[][] = [];
    for (let i = 0; i <= maxIdx; i++) {
      wires.push(outputMap.get(i) ?? []);
    }
    result.set(sourceId, wires);
  }

  return result;
}

/**
 * Parse "output-N" handle strings to a numeric index.
 */
function parseOutputIndex(handle: string | undefined): number {
  if (!handle) return 0;
  const match = handle.match(/^output-(\d+)$/);
  return match ? parseInt(match[1], 10) : 0;
}
