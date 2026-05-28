/**
 * History store — replaces Node-RED's RED.history (history.js).
 *
 * A Zustand store implementing undo/redo for all editor actions that
 * modify the flow. Follows standard undo/redo semantics: push clears
 * the redo stack, undo moves from undo→redo, redo moves from redo→undo.
 *
 * The `multi` action type allows grouping several changes into a single
 * undo step (e.g. a paste operation that adds multiple nodes and wires).
 */

import { create } from "zustand";
import type { FlowNode, Flow } from "../red/nodes/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Wire / edge data used by wire:add and wire:delete actions. */
export interface WireData {
  source: string;
  sourcePort: number;
  target: string;
}

/** Edge data for selection:delete and other bulk operations. */
export interface EdgeData {
  source: string;
  sourcePort: number;
  target: string;
}

/** Recursive discriminated union of all trackable editor actions. */
export type HistoryAction =
  | { type: "node:add"; node: FlowNode }
  | { type: "node:delete"; node: FlowNode }
  | {
      type: "node:move";
      node: FlowNode;
      oldX: number;
      oldY: number;
      newX: number;
      newY: number;
    }
  | {
      type: "node:edit";
      node: FlowNode;
      oldProps: Record<string, unknown>;
      newProps: Record<string, unknown>;
    }
  | { type: "wire:add"; edge: WireData }
  | { type: "wire:delete"; edge: WireData }
  | { type: "selection:delete"; nodes: FlowNode[]; edges: EdgeData[] }
  | { type: "flow:add"; flow: Flow }
  | { type: "flow:delete"; flow: Flow }
  | { type: "group:add"; group: unknown }
  | { type: "group:delete"; group: unknown }
  | { type: "multi"; actions: HistoryAction[] };

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

export interface HistoryState {
  /** Actions that can be undone (most recent at the end). */
  undoStack: HistoryAction[];
  /** Actions that can be redone (most recent at the end). */
  redoStack: HistoryAction[];
  /** Maximum number of entries per stack. Default 50. */
  maxHistory: number;

  // Computed (derived from stacks)
  canUndo: boolean;
  canRedo: boolean;

  // Actions
  push: (action: HistoryAction) => void;
  undo: () => HistoryAction | null;
  redo: () => HistoryAction | null;
  clear: () => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const DEFAULT_MAX_HISTORY = 50;

export const useHistoryStore = create<HistoryState>((set, get) => ({
  undoStack: [],
  redoStack: [],
  maxHistory: DEFAULT_MAX_HISTORY,
  canUndo: false,
  canRedo: false,

  /**
   * Push an action onto the undo stack.
   * Clears the redo stack (standard undo/redo behaviour).
   * Enforces maxHistory by dropping the oldest entry.
   */
  push: (action: HistoryAction) => {
    set((state) => {
      const undoStack = [...state.undoStack, action];
      // Enforce max history limit
      if (undoStack.length > state.maxHistory) {
        undoStack.shift();
      }
      return { undoStack, redoStack: [], canUndo: true, canRedo: false };
    });
  },

  /**
   * Pop the most recent action from the undo stack, push it onto the redo
   * stack, and return it so the caller can reverse its effect.
   */
  undo: (): HistoryAction | null => {
    const state = get();
    if (state.undoStack.length === 0) {
      return null;
    }
    const action = state.undoStack[state.undoStack.length - 1];
    const undoStack = state.undoStack.slice(0, -1);
    const redoStack = [...state.redoStack, action];
    set({
      undoStack,
      redoStack,
      canUndo: undoStack.length > 0,
      canRedo: true,
    });
    return action;
  },

  /**
   * Pop the most recent action from the redo stack, push it onto the undo
   * stack, and return it so the caller can re-apply its effect.
   */
  redo: (): HistoryAction | null => {
    const state = get();
    if (state.redoStack.length === 0) {
      return null;
    }
    const action = state.redoStack[state.redoStack.length - 1];
    const redoStack = state.redoStack.slice(0, -1);
    const undoStack = [...state.undoStack, action];
    set({
      redoStack,
      undoStack,
      canUndo: true,
      canRedo: redoStack.length > 0,
    });
    return action;
  },

  /** Clear both stacks. */
  clear: () => {
    set({ undoStack: [], redoStack: [], canUndo: false, canRedo: false });
  },
}));

export default useHistoryStore;
