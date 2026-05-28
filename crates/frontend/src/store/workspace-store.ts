/**
 * Workspace Store
 *
 * Zustand store for managing flow tabs (workspaces).
 * Each flow tab corresponds to a Node-RED "tab" flow.
 * Replaces Node-RED's RED.workspaces (~700 lines).
 */
import { create } from "zustand";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface WorkspaceFlow {
  id: string;
  label: string;
  type: "tab";
  disabled: boolean;
  info?: string;
}

// ---------------------------------------------------------------------------
// ID counter for generating unique flow IDs
// ---------------------------------------------------------------------------

let flowCounter = 0;

function generateFlowId(): string {
  flowCounter += 1;
  return `flow_${Date.now()}_${flowCounter}`;
}

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

interface WorkspaceStore {
  /** All flow tabs */
  flows: WorkspaceFlow[];
  /** Currently visible flow ID */
  activeFlowId: string | null;

  /** Create a new flow tab, returns its id */
  addFlow: (label?: string) => string;
  /** Remove a flow tab by id */
  removeFlow: (id: string) => void;
  /** Rename a flow tab */
  renameFlow: (id: string, label: string) => void;
  /** Set the currently active flow */
  setActiveFlow: (id: string) => void;
  /** Get the currently active flow object */
  getActiveFlow: () => WorkspaceFlow | undefined;
  /** Reorder flows by moving from one index to another */
  reorderFlows: (fromIndex: number, toIndex: number) => void;
  /** Disable or enable a flow tab */
  setFlowDisabled: (id: string, disabled: boolean) => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useWorkspaceStore = create<WorkspaceStore>((set, get) => ({
  flows: [],
  activeFlowId: null,

  addFlow: (label?: string) => {
    const id = generateFlowId();
    const flow: WorkspaceFlow = {
      id,
      label: label ?? `Flow ${get().flows.length + 1}`,
      type: "tab",
      disabled: false,
    };
    set((state) => ({
      flows: [...state.flows, flow],
      // Auto-activate the first flow added, or keep current active
      activeFlowId: state.activeFlowId ?? id,
    }));
    return id;
  },

  removeFlow: (id: string) => {
    set((state) => {
      const flows = state.flows.filter((f) => f.id !== id);
      let activeFlowId = state.activeFlowId;

      // If we removed the active flow, switch to another
      if (activeFlowId === id) {
        activeFlowId = flows.length > 0 ? flows[0].id : null;
      }

      return { flows, activeFlowId };
    });
  },

  renameFlow: (id: string, label: string) => {
    set((state) => ({
      flows: state.flows.map((f) =>
        f.id === id ? { ...f, label } : f,
      ),
    }));
  },

  setActiveFlow: (id: string) => {
    const state = get();
    if (state.activeFlowId !== id) {
      set({ activeFlowId: id });
      eventBus.emit("workspace:changed", { id });
    }
  },

  getActiveFlow: () => {
    const state = get();
    return state.flows.find((f) => f.id === state.activeFlowId);
  },

  reorderFlows: (fromIndex: number, toIndex: number) => {
    set((state) => {
      const flows = [...state.flows];
      const [moved] = flows.splice(fromIndex, 1);
      flows.splice(toIndex, 0, moved);
      return { flows };
    });
  },

  setFlowDisabled: (id: string, disabled: boolean) => {
    set((state) => ({
      flows: state.flows.map((f) =>
        f.id === id ? { ...f, disabled } : f,
      ),
    }));
  },
}));
