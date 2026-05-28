/**
 * Subflow Store
 *
 * Zustand store for managing subflow definitions.
 * Replaces Node-RED's RED.subflow (~1420 lines).
 *
 * A subflow encapsulates a set of nodes into a reusable building block
 * that appears in the palette and can be dragged onto the canvas like
 * any other node type.
 */

import { create } from "zustand";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SubflowDefinition {
  /** Unique subflow identifier */
  id: string;
  /** Human-readable name shown in the palette and node label */
  name: string;
  /** Palette category, e.g. "function", "common" */
  category: string;
  /** Node colour hex, e.g. "#da9aaa" */
  color: string;
  /** Font Awesome icon class */
  icon: string;
  /** Description / info text */
  description: string;
  /** Number of input ports */
  in: number;
  /** Number of output ports */
  out: number;
  /** IDs of the nodes contained inside the subflow */
  nodes: string[];
  /** Environment variable definitions */
  env: SubflowEnvVar[];
}

export interface SubflowEnvVar {
  name: string;
  value: string;
  type: "str" | "num" | "bool" | "json";
}

export interface SubflowResult {
  /** The created subflow definition */
  subflow: SubflowDefinition;
  /** IDs of nodes that were moved into the subflow */
  movedNodeIds: string[];
}

// ---------------------------------------------------------------------------
// ID counter
// ---------------------------------------------------------------------------

let subflowCounter = 0;

function generateSubflowId(): string {
  subflowCounter += 1;
  return `subflow_${Date.now()}_${subflowCounter}`;
}

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

interface SubflowStore {
  /** All registered subflow definitions */
  subflows: Map<string, SubflowDefinition>;

  /**
   * Create a new empty subflow definition and register it.
   * Returns the new subflow ID.
   */
  createSubflow: (opts?: Partial<SubflowDefinition>) => string;

  /**
   * Delete a subflow by ID.  Emits `subflow:deleted`.
   */
  deleteSubflow: (id: string) => void;

  /**
   * Apply partial updates to an existing subflow.  Emits `subflow:changed`.
   */
  updateSubflow: (id: string, updates: Partial<SubflowDefinition>) => void;

  /**
   * Retrieve a subflow definition by ID.
   */
  getSubflow: (id: string) => SubflowDefinition | undefined;

  /**
   * Convert a set of existing canvas nodes into a new subflow.
   * Returns the created subflow definition and the list of moved node IDs.
   */
  convertToSubflow: (nodeIds: string[]) => SubflowResult;

  /**
   * Get all subflow definitions as an array.
   */
  getAllSubflows: () => SubflowDefinition[];
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useSubflowStore = create<SubflowStore>((set, get) => ({
  subflows: new Map(),

  createSubflow: (opts) => {
    const id = generateSubflowId();
    const def: SubflowDefinition = {
      id,
      name: opts?.name ?? `Subflow ${get().subflows.size + 1}`,
      category: opts?.category ?? "function",
      color: opts?.color ?? "#da9aaa",
      icon: opts?.icon ?? "debugger",
      description: opts?.description ?? "",
      in: opts?.in ?? 1,
      out: opts?.out ?? 1,
      nodes: opts?.nodes ?? [],
      env: opts?.env ?? [],
    };

    set((state) => {
      const next = new Map(state.subflows);
      next.set(id, def);
      return { subflows: next };
    });

    eventBus.emit("subflow:created", { id, name: def.name });
    return id;
  },

  deleteSubflow: (id) => {
    const def = get().subflows.get(id);
    if (!def) return;

    set((state) => {
      const next = new Map(state.subflows);
      next.delete(id);
      return { subflows: next };
    });

    eventBus.emit("subflow:deleted", { id, name: def.name });
  },

  updateSubflow: (id, updates) => {
    const existing = get().subflows.get(id);
    if (!existing) return;

    const updated: SubflowDefinition = { ...existing, ...updates };

    set((state) => {
      const next = new Map(state.subflows);
      next.set(id, updated);
      return { subflows: next };
    });

    eventBus.emit("subflow:changed", { id });
  },

  getSubflow: (id) => {
    return get().subflows.get(id);
  },

  convertToSubflow: (nodeIds) => {
    const id = get().createSubflow({
      nodes: [...nodeIds],
      name: `Subflow ${get().subflows.size + 1}`,
    });

    const subflow = get().subflows.get(id)!;

    eventBus.emit("subflow:converted", {
      id,
      nodeIds,
    });

    return {
      subflow,
      movedNodeIds: [...nodeIds],
    };
  },

  getAllSubflows: () => {
    return Array.from(get().subflows.values());
  },
}));
