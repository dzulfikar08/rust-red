/**
 * Settings store — replaces Node-RED's RED.settings.
 *
 * A Zustand store with localStorage persistence for user preferences.
 * Provides both typed accessor actions and a generic `get`/`set` API
 * compatible with Node-RED's RED.settings.get/set pattern.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type Theme = "dark" | "light" | "system";

export interface EditorPreferences {
  gridSize: number;
  snapToGrid: boolean;
  showTips: boolean;
}

export interface SettingsState {
  // State
  theme: Theme;
  locale: string;
  editorPreferences: EditorPreferences;
  sidebarWidth: number;
  paletteCategoriesExpanded: Record<string, boolean>;

  // Actions
  setTheme: (theme: Theme) => void;
  setLocale: (locale: string) => void;
  updateEditorPreference: <K extends keyof EditorPreferences>(
    key: K,
    value: EditorPreferences[K],
  ) => void;
  setSidebarWidth: (width: number) => void;
  togglePaletteCategory: (category: string) => void;

  /**
   * Generic getter — mirrors RED.settings.get(key).
   * Returns `undefined` for unknown keys.
   */
  get: <T = unknown>(key: string) => T | undefined;

  /**
   * Generic setter — mirrors RED.settings.set(key, value).
   * Stores arbitrary values under a generic map so plugins can read/write
   * settings without pre-defined keys.
   */
  set: <T = unknown>(key: string, value: T) => void;
}

// ---------------------------------------------------------------------------
// Extra generic settings stored alongside typed state
// ---------------------------------------------------------------------------

/** Internal extension to hold arbitrary plugin settings. */
interface SettingsInternal extends SettingsState {
  _generic: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useSettingsStore = create<SettingsInternal>()(
  persist(
    (set, get) => ({
      // Default state
      theme: "system" as Theme,
      locale: "en",
      editorPreferences: {
        gridSize: 20,
        snapToGrid: false,
        showTips: true,
      },
      sidebarWidth: 300,
      paletteCategoriesExpanded: {},
      _generic: {},

      // Actions
      setTheme: (theme) => set({ theme }),

      setLocale: (locale) => set({ locale }),

      updateEditorPreference: (key, value) =>
        set((state) => ({
          editorPreferences: {
            ...state.editorPreferences,
            [key]: value,
          },
        })),

      setSidebarWidth: (width) => set({ sidebarWidth: width }),

      togglePaletteCategory: (category) =>
        set((state) => ({
          paletteCategoriesExpanded: {
            ...state.paletteCategoriesExpanded,
            [category]: !state.paletteCategoriesExpanded[category],
          },
        })),

      // Generic get/set
      get: <T = unknown,>(key: string): T | undefined => {
        const state = get();
        // Check typed fields first
        if (key in state && key !== "_generic") {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          return (state as any)[key] as T;
        }
        return state._generic[key] as T | undefined;
      },

      set: <T = unknown,>(key: string, value: T): void => {
        // If it maps to a known typed field, update via the proper action
        const state = get();
        if (key === "theme") {
          set({ theme: value as Theme });
        } else if (key === "locale") {
          set({ locale: value as string });
        } else if (key === "sidebarWidth") {
          set({ sidebarWidth: value as number });
        } else if (key === "editorPreferences") {
          set({ editorPreferences: value as EditorPreferences });
        } else if (key === "paletteCategoriesExpanded") {
          set({ paletteCategoriesExpanded: value as Record<string, boolean> });
        } else {
          set({ _generic: { ...state._generic, [key]: value } });
        }
      },
    }),
    {
      name: "rust-red-settings",
    },
  ),
);

export default useSettingsStore;
