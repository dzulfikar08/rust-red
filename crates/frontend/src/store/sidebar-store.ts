import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type SidebarTab =
  | "info"
  | "debug"
  | "config"
  | "context"
  | "help"
  | "outliner";

export interface SidebarStore {
  /** Currently active sidebar tab. */
  activeTab: SidebarTab;
  /** Whether the sidebar content panel is open. */
  isOpen: boolean;
  /** Sidebar panel width in pixels. */
  width: number;

  setActiveTab(tab: SidebarTab): void;
  toggle(): void;
  open(tab?: SidebarTab): void;
  close(): void;
  setWidth(width: number): void;
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_WIDTH = 320;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useSidebarStore = create<SidebarStore>((set) => ({
  activeTab: "info",
  isOpen: false,
  width: DEFAULT_WIDTH,

  setActiveTab: (tab) => {
    set({ activeTab: tab });
  },

  toggle: () => {
    set((state) => ({ isOpen: !state.isOpen }));
  },

  open: (tab) => {
    if (tab !== undefined) {
      set({ isOpen: true, activeTab: tab });
    } else {
      set({ isOpen: true });
    }
  },

  close: () => {
    set({ isOpen: false });
  },

  setWidth: (width) => {
    set({ width });
  },
}));
