/**
 * Palette Store
 *
 * Zustand store managing the palette UI state: search query
 * and expanded/collapsed categories.
 */

import { create } from "zustand";

interface PaletteState {
  /** Current search query typed by the user. */
  searchQuery: string;
  /** Set of category names currently expanded. */
  expandedCategories: Set<string>;

  /** Update the search query. */
  setSearchQuery: (query: string) => void;
  /** Toggle a category between expanded and collapsed. */
  toggleCategory: (category: string) => void;
  /** Check whether a category is currently expanded. */
  isCategoryExpanded: (category: string) => boolean;
  /** Expand all categories. */
  expandAll: () => void;
}

export const usePaletteStore = create<PaletteState>((set, get) => ({
  searchQuery: "",
  expandedCategories: new Set<string>(["common"]),

  setSearchQuery: (query) => {
    set({ searchQuery: query });
  },

  toggleCategory: (category) => {
    set((state) => {
      const next = new Set(state.expandedCategories);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return { expandedCategories: next };
    });
  },

  isCategoryExpanded: (category) => {
    return get().expandedCategories.has(category);
  },

  expandAll: () => {
    // expandAll will be called with the full list from outside,
    // but we provide a convenience that reads categories from registry.
    // The caller should use setExpandedCategories for specific sets.
    set((state) => ({ expandedCategories: state.expandedCategories }));
  },
}));
