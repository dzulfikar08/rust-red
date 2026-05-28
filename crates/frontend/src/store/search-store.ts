/**
 * Search Store
 *
 * Zustand store for global search across all flows.
 * Searches nodes by name, type, and property values (case-insensitive).
 * Replaces Node-RED's RED.search (~700 lines).
 */

import { create } from "zustand";
import type { Node } from "@xyflow/react";
import { useFlowStore } from "./flow-store";
import { useEditorStore } from "./editor-store";
import { useWorkspaceStore, type WorkspaceFlow } from "./workspace-store";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SearchResult {
  nodeId: string;
  nodeType: string;
  nodeLabel: string;
  flowId: string;
  flowLabel: string;
  matchField: string;
  matchContext: string;
}

interface SearchStore {
  query: string;
  results: SearchResult[];
  isSearching: boolean;
  selectedIndex: number;
  isOpen: boolean;

  open: () => void;
  close: () => void;
  search: (query: string) => void;
  selectResult: (index: number) => void;
  nextResult: () => void;
  prevResult: () => void;
  clear: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Extract the label for a node. Priority:
 *  1. data.label
 *  2. data.name
 *  3. data.type
 */
function getNodeLabel(node: Node): string {
  const data = node.data as Record<string, unknown>;
  if (typeof data.label === "string" && data.label.length > 0) return data.label;
  if (typeof data.name === "string" && data.name.length > 0) return data.name;
  if (typeof data.type === "string") return data.type;
  return node.id;
}

/**
 * Extract the node type from data or the node itself.
 */
function getNodeType(node: Node): string {
  const data = node.data as Record<string, unknown>;
  if (typeof data.type === "string") return data.type;
  if (typeof data.nodeType === "string") return data.nodeType;
  return node.type ?? "unknown";
}

/**
 * Get the flow (tab) ID that a node belongs to.
 * Falls back to the active flow if no z property exists.
 */
function getNodeFlowId(node: Node): string {
  const data = node.data as Record<string, unknown>;
  if (typeof data.z === "string" && data.z.length > 0) return data.z;
  // Nodes without explicit z belong to the active flow
  const activeFlowId = useWorkspaceStore.getState().activeFlowId;
  return activeFlowId ?? "";
}

/**
 * Resolve a flow label from its ID.
 */
function getFlowLabel(flowId: string, flows: WorkspaceFlow[]): string {
  const flow = flows.find((f) => f.id === flowId);
  return flow?.label ?? "Unknown Flow";
}

/**
 * Create a context snippet around a match within a string.
 */
function createContextSnippet(text: string, query: string): string {
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const idx = lowerText.indexOf(lowerQuery);
  if (idx === -1) return text;

  const start = Math.max(0, idx - 20);
  const end = Math.min(text.length, idx + query.length + 20);
  let snippet = text.slice(start, end);
  if (start > 0) snippet = `...${snippet}`;
  if (end < text.length) snippet = `${snippet}...`;
  return snippet;
}

/**
 * Search a single node's data for a query match. Returns all matches found.
 */
function searchNode(
  node: Node,
  query: string,
  flowId: string,
  flowLabel: string,
): SearchResult[] {
  const results: SearchResult[] = [];
  const lowerQuery = query.toLowerCase();
  const nodeLabel = getNodeLabel(node);
  const nodeType = getNodeType(node);
  const data = node.data as Record<string, unknown>;

  // 1. Match on node label (name)
  if (nodeLabel.toLowerCase().includes(lowerQuery)) {
    results.push({
      nodeId: node.id,
      nodeType,
      nodeLabel,
      flowId,
      flowLabel,
      matchField: "name",
      matchContext: createContextSnippet(nodeLabel, query),
    });
  }

  // 2. Match on node type
  if (nodeType.toLowerCase().includes(lowerQuery) && !results.some((r) => r.matchField === "name" && r.nodeId === node.id)) {
    results.push({
      nodeId: node.id,
      nodeType,
      nodeLabel,
      flowId,
      flowLabel,
      matchField: "type",
      matchContext: createContextSnippet(nodeType, query),
    });
  }

  // 3. Match on property values
  const skipKeys = new Set(["label", "name", "type", "nodeType", "z", "category", "color", "icon", "_isConfig", "_users"]);
  for (const [key, value] of Object.entries(data)) {
    if (skipKeys.has(key)) continue;
    if (value == null) continue;
    const strValue = typeof value === "string" ? value : String(value);
    if (strValue.toLowerCase().includes(lowerQuery)) {
      results.push({
        nodeId: node.id,
        nodeType,
        nodeLabel,
        flowId,
        flowLabel,
        matchField: key,
        matchContext: createContextSnippet(strValue, query),
      });
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useSearchStore = create<SearchStore>((set, get) => ({
  query: "",
  results: [],
  isSearching: false,
  selectedIndex: 0,
  isOpen: false,

  open: () => {
    set({ isOpen: true });
  },

  close: () => {
    set({ isOpen: false, query: "", results: [], selectedIndex: 0, isSearching: false });
  },

  search: (query: string) => {
    const trimmed = query.trim();
    set({ query, isSearching: true });

    if (trimmed.length === 0) {
      set({ results: [], isSearching: false, selectedIndex: 0 });
      return;
    }

    const { nodes } = useFlowStore.getState();
    const { flows } = useWorkspaceStore.getState();
    const allResults: SearchResult[] = [];

    for (const node of nodes) {
      const flowId = getNodeFlowId(node);
      const flowLabel = getFlowLabel(flowId, flows);
      const nodeResults = searchNode(node, trimmed, flowId, flowLabel);
      allResults.push(...nodeResults);
    }

    // Deduplicate: keep only first result per nodeId
    const seen = new Set<string>();
    const deduped: SearchResult[] = [];
    for (const r of allResults) {
      if (!seen.has(r.nodeId)) {
        seen.add(r.nodeId);
        deduped.push(r);
      }
    }

    set({ results: deduped, isSearching: false, selectedIndex: deduped.length > 0 ? 0 : 0 });
  },

  selectResult: (index: number) => {
    const { results } = get();
    if (index < 0 || index >= results.length) return;
    set({ selectedIndex: index });

    const result = results[index];

    // Switch to the correct flow tab
    const { setActiveFlow } = useWorkspaceStore.getState();
    setActiveFlow(result.flowId);

    // Select the node in the editor
    const { selectNode } = useEditorStore.getState();
    selectNode(result.nodeId);

    // Emit event for external listeners (viewport centering, etc.)
    eventBus.emit("search:result-selected", {
      nodeId: result.nodeId,
      flowId: result.flowId,
    });
  },

  nextResult: () => {
    const { results, selectedIndex } = get();
    if (results.length === 0) return;
    const next = (selectedIndex + 1) % results.length;
    get().selectResult(next);
  },

  prevResult: () => {
    const { results, selectedIndex } = get();
    if (results.length === 0) return;
    const prev = selectedIndex === 0 ? results.length - 1 : selectedIndex - 1;
    get().selectResult(prev);
  },

  clear: () => {
    set({ query: "", results: [], isSearching: false, selectedIndex: 0 });
  },
}));
