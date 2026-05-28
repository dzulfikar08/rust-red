/**
 * Palette Editor Store
 *
 * Zustand store for managing the palette editor dialog:
 * installed modules, npm search results, and mock install/remove.
 *
 * Replaces Node-RED's RED.paletteEditor (~1930 lines).
 */

import { create } from "zustand";
import { nodeRegistry } from "../red/nodes/registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface InstalledModule {
  name: string;
  version: string;
  description: string;
  nodes: string[];
  /** True if this is a core / built-in module that cannot be removed. */
  local?: boolean;
}

export type InstallStatus = "idle" | "installing" | "success" | "error";

export interface PaletteEditorStore {
  /** Modules currently installed (core + user-installed). */
  installed: InstalledModule[];
  /** Search results from npm registry lookup (mocked). */
  available: InstalledModule[];
  /** True when an async operation is in progress. */
  isLoading: boolean;
  /** Status of the most recent install attempt. */
  installStatus: InstallStatus;

  /** Search npm for packages matching the query (mock). */
  searchNpm: (query: string) => Promise<void>;
  /** Install a module by name (mock). */
  installModule: (name: string) => Promise<void>;
  /** Remove a non-core module by name. */
  removeModule: (name: string) => void;
  /** Refresh the installed list from the node registry. */
  refreshInstalled: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build the initial "installed" list from the node registry categories. */
function buildInstalledFromRegistry(): InstalledModule[] {
  const categories = nodeRegistry.getCategories();
  return categories.map((cat) => {
    const nodes = nodeRegistry.getTypesByCategory(cat);
    return {
      name: `@node-red/${cat}`,
      version: "3.1.0",
      description: `Core ${cat} nodes`,
      nodes: nodes.map((n) => n.type),
      local: true,
    };
  });
}

/** Mock npm search results for a given query. */
function mockNpmResults(query: string): InstalledModule[] {
  const pool: InstalledModule[] = [
    {
      name: "node-red-dashboard",
      version: "3.1.0",
      description: "Dashboard UI nodes for Node-RED",
      nodes: ["ui_button", "ui_slider", "ui_chart", "ui_text"],
    },
    {
      name: "node-red-node-serialport",
      version: "1.0.4",
      description: "Serial port nodes for Node-RED",
      nodes: ["serial in", "serial out"],
    },
    {
      name: "node-red-contrib-influxdb",
      version: "0.6.1",
      description: "InfluxDB input and output nodes",
      nodes: ["influxdb in", "influxdb out"],
    },
    {
      name: "node-red-contrib-postgres",
      version: "1.0.2",
      description: "PostgreSQL node for Node-RED",
      nodes: ["postgres"],
    },
    {
      name: "node-red-node-feedparser",
      version: "0.3.0",
      description: "RSS/Atom feed parser node",
      nodes: ["feedparse"],
    },
    {
      name: "node-red-contrib-telegrambot",
      version: "12.0.1",
      description: "Telegram bot nodes for Node-RED",
      nodes: ["telegram bot", "telegram sender"],
    },
  ];

  const q = query.toLowerCase();
  return pool.filter(
    (m) =>
      m.name.toLowerCase().includes(q) ||
      m.description.toLowerCase().includes(q),
  );
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const usePaletteEditorStore = create<PaletteEditorStore>()(
  (set, get) => ({
    installed: buildInstalledFromRegistry(),
    available: [],
    isLoading: false,
    installStatus: "idle",

    // ------------------------------------------------------------------
    // searchNpm -- mock implementation
    // ------------------------------------------------------------------
    searchNpm: async (query: string) => {
      set({ isLoading: true });
      // Simulate network delay
      await new Promise((r) => setTimeout(r, 300));

      const installedNames = new Set(get().installed.map((m) => m.name));
      const results = mockNpmResults(query).map((m) => ({
        ...m,
        // Mark already-installed modules
        local: installedNames.has(m.name),
      }));

      set({ available: results, isLoading: false });
    },

    // ------------------------------------------------------------------
    // installModule -- mock implementation
    // ------------------------------------------------------------------
    installModule: async (name: string) => {
      set({ installStatus: "installing" });
      // Simulate install delay
      await new Promise((r) => setTimeout(r, 500));

      const existing = get().available.find((m) => m.name === name);
      if (!existing) {
        set({ installStatus: "error" });
        return;
      }

      // Add to installed list
      set((state) => ({
        installed: [
          ...state.installed,
          { ...existing, local: false },
        ],
        installStatus: "success",
      }));

      // Also register mock node types in the registry so they appear in
      // the palette (types registered under their category "community").
      for (const nodeType of existing.nodes) {
        nodeRegistry.registerType(nodeType, {
          id: nodeType,
          type: nodeType,
          name: nodeType,
          category: "community",
          color: "#c0edc0",
          inputs: 1,
          outputs: 1,
          defaults: {},
          paletteLabel: nodeType,
        });
      }

      // Reset install status after a moment
      setTimeout(() => {
        set({ installStatus: "idle" });
      }, 1000);
    },

    // ------------------------------------------------------------------
    // removeModule
    // ------------------------------------------------------------------
    removeModule: (name: string) => {
      const mod = get().installed.find((m) => m.name === name);
      if (!mod || mod.local) return; // cannot remove core modules

      // Remove node types from registry
      for (const nodeType of mod.nodes) {
        nodeRegistry.removeType(nodeType);
      }

      set((state) => ({
        installed: state.installed.filter((m) => m.name !== name),
      }));
    },

    // ------------------------------------------------------------------
    // refreshInstalled
    // ------------------------------------------------------------------
    refreshInstalled: () => {
      set({ installed: buildInstalledFromRegistry() });
    },
  }),
);

export default usePaletteEditorStore;
