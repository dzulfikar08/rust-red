/**
 * Editor Panel Store
 *
 * Manages state for the node editor panel that appears when a node is
 * double-clicked. The editor can render in the sidebar or as a pop-out modal.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface EditorPanelStore {
  /** Whether the editor panel is currently visible. */
  isOpen: boolean;
  /** When true the editor renders as a centered modal overlay instead of
   *  inline in the sidebar. */
  isModal: boolean;
  /** ID of the node currently being edited (null when closed). */
  nodeId: string | null;
  /** Node type string for the node being edited. */
  nodeType: string | null;
  /** Snapshot of the node's original properties when editing began. */
  originalData: Record<string, unknown>;
  /** Current form values — mutated as the user edits. */
  formData: Record<string, unknown>;
  /** Whether formData differs from originalData. */
  isDirty: boolean;

  // ---- Actions ----

  /** Open the editor for the given node. */
  openEditor: (
    nodeId: string,
    nodeType: string,
    nodeData: Record<string, unknown>,
  ) => void;
  /** Close the editor (reset all state). */
  closeEditor: () => void;
  /** Toggle between sidebar and modal mode. */
  toggleModal: () => void;
  /** Replace the entire form data object. */
  setFormData: (data: Record<string, unknown>) => void;
  /** Update a single field in the form data. */
  updateField: (key: string, value: unknown) => void;
  /** Reset dirty flag — used after a successful save. */
  markClean: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Shallow equality check for two flat records. */
function shallowEqual(
  a: Record<string, unknown>,
  b: Record<string, unknown>,
): boolean {
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  if (keysA.length !== keysB.length) return false;
  return keysA.every((k) => a[k] === b[k]);
}

// ---------------------------------------------------------------------------
// Initial / empty state
// ---------------------------------------------------------------------------

const EMPTY_STATE = {
  isOpen: false,
  isModal: false,
  nodeId: null,
  nodeType: null,
  originalData: {},
  formData: {},
  isDirty: false,
} as const;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useEditorPanelStore = create<EditorPanelStore>((set, get) => ({
  ...EMPTY_STATE,

  openEditor: (nodeId, nodeType, nodeData) => {
    set({
      isOpen: true,
      isModal: false,
      nodeId,
      nodeType,
      originalData: { ...nodeData },
      formData: { ...nodeData },
      isDirty: false,
    });
  },

  closeEditor: () => {
    set({ ...EMPTY_STATE });
  },

  toggleModal: () => {
    set((state) => ({ isModal: !state.isModal }));
  },

  setFormData: (data) => {
    set((state) => ({
      formData: { ...data },
      isDirty: !shallowEqual(data, state.originalData),
    }));
  },

  updateField: (key, value) => {
    const next = { ...get().formData, [key]: value };
    set({
      formData: next,
      isDirty: !shallowEqual(next, get().originalData),
    });
  },

  markClean: () => {
    set((state) => ({
      originalData: { ...state.formData },
      isDirty: false,
    }));
  },
}));
