/**
 * Action store — replaces Node-RED's RED.actions.
 *
 * A Zustand store that backs the global action registry.
 * Actions can be registered, invoked by ID, queried, and grouped by scope.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Action {
  id: string; // e.g. 'core:undo', 'core:redo', 'core:delete-selected'
  name: string; // Human-readable display name
  scope: string; // e.g. 'core', 'node', 'flow'
  key?: string; // Keyboard shortcut label, e.g. 'Ctrl-Z'
  handler: () => void | Promise<void>;
  enabled?: () => boolean; // Dynamic enable check (defaults to true)
}

interface ActionState {
  actions: Map<string, Action>;

  registerAction: (action: Action) => void;
  unregisterAction: (id: string) => void;
  invokeAction: (id: string) => void;
  getAction: (id: string) => Action | undefined;
  getAllActions: () => Action[];
  getActionsByScope: (scope: string) => Action[];
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useActionStore = create<ActionState>((set, get) => ({
  actions: new Map(),

  registerAction: (action) => {
    set((state) => {
      const next = new Map(state.actions);
      next.set(action.id, action);
      return { actions: next };
    });
  },

  unregisterAction: (id) => {
    set((state) => {
      const next = new Map(state.actions);
      next.delete(id);
      return { actions: next };
    });
  },

  invokeAction: (id) => {
    const action = get().actions.get(id);
    if (!action) return;
    if (action.enabled && !action.enabled()) return;
    action.handler();
  },

  getAction: (id) => get().actions.get(id),

  getAllActions: () => Array.from(get().actions.values()),

  getActionsByScope: (scope) =>
    Array.from(get().actions.values()).filter((a) => a.scope === scope),
}));

export default useActionStore;
