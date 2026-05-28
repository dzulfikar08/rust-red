/**
 * Context menu store — manages visibility, position, and items for the
 * right-click context menu on the canvas.
 *
 * Listens for `canvas:context-menu` events from FlowCanvas and exposes
 * show/hide actions plus the current menu state for the ContextMenu component.
 */

import { create } from "zustand";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface MenuItem {
  label: string;
  shortcut?: string;
  action?: () => void;
  separator?: boolean;
  disabled?: boolean;
  icon?: React.ReactNode;
}

interface ContextMenuPosition {
  /** Screen (client) coordinates — where the menu is anchored */
  x: number;
  y: number;
  /** Flow (canvas) coordinates at right-click point */
  flowX: number;
  flowY: number;
}

interface ContextMenuState {
  /** Whether the menu is currently visible */
  visible: boolean;
  /** Position at which to show the menu */
  position: ContextMenuPosition;
  /** Menu items to render */
  items: MenuItem[];
  /** IDs of selected nodes when the menu was opened */
  selectedNodeIds: string[];
  /** Whether the menu was triggered on a node (vs empty canvas) */
  isOnNode: boolean;

  /** Show the menu with given items at given position */
  show: (opts: {
    x: number;
    y: number;
    flowX: number;
    flowY: number;
    items: MenuItem[];
    selectedNodeIds?: string[];
    isOnNode?: boolean;
  }) => void;
  /** Hide the menu */
  hide: () => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useContextMenuStore = create<ContextMenuState>((set) => ({
  visible: false,
  position: { x: 0, y: 0, flowX: 0, flowY: 0 },
  items: [],
  selectedNodeIds: [],
  isOnNode: false,

  show: (opts) =>
    set({
      visible: true,
      position: {
        x: opts.x,
        y: opts.y,
        flowX: opts.flowX,
        flowY: opts.flowY,
      },
      items: opts.items,
      selectedNodeIds: opts.selectedNodeIds ?? [],
      isOnNode: opts.isOnNode ?? false,
    }),

  hide: () =>
    set({
      visible: false,
      items: [],
      selectedNodeIds: [],
      isOnNode: false,
    }),
}));

// ---------------------------------------------------------------------------
// Event bus listener — auto-show on canvas:context-menu
// ---------------------------------------------------------------------------

let _eventUnsubscribe: (() => void) | null = null;

/**
 * Start listening for `canvas:context-menu` events.
 * Called once during app bootstrap. Safe to call multiple times.
 */
export function initContextMenuListener() {
  if (_eventUnsubscribe) return;

  _eventUnsubscribe = eventBus.on("canvas:context-menu", (data) => {
    const { x, y, flowX, flowY } = data as {
      x: number;
      y: number;
      flowX: number;
      flowY: number;
    };

    // Import flow store inline to avoid circular dependency at module level
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { useFlowStore } = require("./flow-store") as {
      useFlowStore: { getState: () => { nodes: { id: string; selected?: boolean }[] } };
    };
    const { nodes } = useFlowStore.getState();
    const selectedNodes = nodes.filter((n) => n.selected);
    const selectedNodeIds = selectedNodes.map((n) => n.id);

    // Determine if the click landed on a node by checking flow coordinates
    const isOnNode = selectedNodeIds.length > 0;

    // Build items will be deferred to the component that subscribes,
    // but we store the position and metadata so it can decide.
    useContextMenuStore.getState().show({
      x,
      y,
      flowX,
      flowY,
      items: [], // Items will be populated by the ContextMenuManager component
      selectedNodeIds,
      isOnNode,
    });
  });
}

/**
 * Stop listening (for tests / teardown).
 */
export function destroyContextMenuListener() {
  if (_eventUnsubscribe) {
    _eventUnsubscribe();
    _eventUnsubscribe = null;
  }
}

export default useContextMenuStore;
