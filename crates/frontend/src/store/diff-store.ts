/**
 * Diff Store -- replaces Node-RED's RED.diff.
 *
 * Zustand store for computing and managing flow diffs between
 * the local (current editor) state and the remote (deployed) state.
 */

import { create } from "zustand";
import { flowsApi } from "../api/flows";
import { useFlowStore } from "./flow-store";
import { useNotificationStore } from "./notification-store";
import type { Node } from "@xyflow/react";
import type { FlowNodeData } from "../api/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FlowNode = Node<FlowNodeData>;

export interface PropertyChange {
  old: unknown;
  new: unknown;
}

export interface ModifiedNode {
  node: FlowNode;
  changes: Record<string, PropertyChange>;
}

export interface DiffResult {
  added: FlowNode[];
  removed: FlowNode[];
  modified: ModifiedNode[];
  unchanged: FlowNode[];
}

export type DiffFilter = "all" | "changes";

interface DiffStore {
  localFlow: FlowNode[] | null;
  remoteFlow: FlowNode[] | null;
  diffResult: DiffResult | null;
  isLoading: boolean;
  filter: DiffFilter;

  computeDiff(local: FlowNode[], remote: FlowNode[]): DiffResult;
  fetchRemoteAndDiff(): Promise<void>;
  setFilter: (filter: DiffFilter) => void;
  clear(): void;
}

// ---------------------------------------------------------------------------
// Diff algorithm
// ---------------------------------------------------------------------------

/**
 * Compare two FlowNode arrays and produce a DiffResult.
 *
 * Nodes are matched by `id`. For each matched pair, properties
 * at the top level and within `data` are compared to detect changes.
 */
export function computeDiff(
  local: FlowNode[],
  remote: FlowNode[],
): DiffResult {
  const localMap = new Map<string, FlowNode>();
  const remoteMap = new Map<string, FlowNode>();

  for (const node of local) {
    localMap.set(node.id, node);
  }
  for (const node of remote) {
    remoteMap.set(node.id, node);
  }

  const added: FlowNode[] = [];
  const removed: FlowNode[] = [];
  const modified: ModifiedNode[] = [];
  const unchanged: FlowNode[] = [];

  // Detect added and modified nodes
  for (const [id, localNode] of localMap) {
    const remoteNode = remoteMap.get(id);
    if (!remoteNode) {
      added.push(localNode);
      continue;
    }

    const changes = diffNodeProperties(localNode, remoteNode);
    if (Object.keys(changes).length > 0) {
      modified.push({ node: localNode, changes });
    } else {
      unchanged.push(localNode);
    }
  }

  // Detect removed nodes
  for (const [id, remoteNode] of remoteMap) {
    if (!localMap.has(id)) {
      removed.push(remoteNode);
    }
  }

  return { added, removed, modified, unchanged };
}

/**
 * Compare two nodes and return the properties that differ.
 * Compares top-level keys and keys inside `data`.
 */
function diffNodeProperties(
  local: FlowNode,
  remote: FlowNode,
): Record<string, PropertyChange> {
  const changes: Record<string, PropertyChange> = {};

  // Compare top-level position
  if (local.position && remote.position) {
    if (local.position.x !== remote.position.x) {
      changes["x"] = { old: remote.position.x, new: local.position.x };
    }
    if (local.position.y !== remote.position.y) {
      changes["y"] = { old: remote.position.y, new: local.position.y };
    }
  } else if (local.position && !remote.position) {
    changes["x"] = { old: undefined, new: local.position.x };
    changes["y"] = { old: undefined, new: local.position.y };
  }

  // Compare data properties
  const localData = local.data ?? {};
  const remoteData = remote.data ?? {};
  const allKeys = new Set([
    ...Object.keys(localData),
    ...Object.keys(remoteData),
  ]);

  for (const key of allKeys) {
    const localVal = localData[key];
    const remoteVal = remoteData[key];

    if (!isEqual(localVal, remoteVal)) {
      changes[key] = { old: remoteVal, new: localVal };
    }
  }

  return changes;
}

/**
 * Shallow equality check. Handles primitives and JSON-serializable objects.
 */
function isEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a === b;
  if (typeof a !== typeof b) return false;
  if (typeof a === "object") {
    try {
      return JSON.stringify(a) === JSON.stringify(b);
    } catch {
      return false;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useDiffStore = create<DiffStore>((set, get) => ({
  localFlow: null,
  remoteFlow: null,
  diffResult: null,
  isLoading: false,
  filter: "changes",

  computeDiff(local, remote) {
    const result = computeDiff(local, remote);
    set({
      localFlow: local,
      remoteFlow: remote,
      diffResult: result,
    });
    return result;
  },

  async fetchRemoteAndDiff() {
    set({ isLoading: true });

    try {
      const response = await flowsApi.getFlows();
      const remoteNodes = response.nodes ?? [];
      const localNodes = useFlowStore.getState().nodes;

      get().computeDiff(localNodes, remoteNodes);
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "Failed to fetch remote flows";
      useNotificationStore
        .getState()
        .error("Diff failed", message);
    } finally {
      set({ isLoading: false });
    }
  },

  setFilter(filter: DiffFilter) {
    set({ filter });
  },

  clear() {
    set({
      localFlow: null,
      remoteFlow: null,
      diffResult: null,
      isLoading: false,
      filter: "changes",
    });
  },
}));

export default useDiffStore;
