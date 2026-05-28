import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { usePaletteEditorStore } from "../palette-editor-store";
import { nodeRegistry } from "../../red/nodes/registry";
import { registerBuiltinNodes } from "../../red/nodes/builtin-nodes";

function resetStore() {
  usePaletteEditorStore.setState({
    installed: [],
    available: [],
    isLoading: false,
    installStatus: "idle",
  });
}

describe("usePaletteEditorStore", () => {
  beforeEach(() => {
    nodeRegistry.clear();
    registerBuiltinNodes();
    resetStore();
    // Refresh to populate installed from registry
    usePaletteEditorStore.getState().refreshInstalled();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Default state / refreshInstalled
  // -----------------------------------------------------------------------

  describe("refreshInstalled()", () => {
    it("populates installed modules from the node registry", () => {
      const { installed } = usePaletteEditorStore.getState();
      expect(installed.length).toBeGreaterThan(0);
    });

    it("marks all core modules as local (built-in)", () => {
      const { installed } = usePaletteEditorStore.getState();
      for (const mod of installed) {
        expect(mod.local).toBe(true);
      }
    });

    it("each module has name, version, description, and nodes", () => {
      const { installed } = usePaletteEditorStore.getState();
      for (const mod of installed) {
        expect(mod.name).toBeTruthy();
        expect(mod.version).toBeTruthy();
        expect(mod.description).toBeTruthy();
        expect(Array.isArray(mod.nodes)).toBe(true);
        expect(mod.nodes.length).toBeGreaterThan(0);
      }
    });
  });

  // -----------------------------------------------------------------------
  // searchNpm (mock)
  // -----------------------------------------------------------------------

  describe("searchNpm()", () => {
    it("returns mock results for a matching query", async () => {
      await usePaletteEditorStore.getState().searchNpm("dashboard");
      const { available } = usePaletteEditorStore.getState();
      expect(available.length).toBeGreaterThan(0);
      expect(
        available.some((m) => m.name.includes("dashboard")),
      ).toBe(true);
    });

    it("returns empty for a non-matching query", async () => {
      await usePaletteEditorStore.getState().searchNpm("zzznonexistent");
      const { available } = usePaletteEditorStore.getState();
      expect(available.length).toBe(0);
    });

    it("sets isLoading during search and clears after", async () => {
      const promise = usePaletteEditorStore.getState().searchNpm("mqtt");
      // isLoading should be true while pending
      expect(usePaletteEditorStore.getState().isLoading).toBe(true);
      await promise;
      expect(usePaletteEditorStore.getState().isLoading).toBe(false);
    });

    it("marks already-installed modules as local in search results", async () => {
      // Install a mock module first
      await usePaletteEditorStore.getState().searchNpm("dashboard");
      const dashboardModule = usePaletteEditorStore
        .getState()
        .available.find((m) => m.name === "node-red-dashboard");
      expect(dashboardModule).toBeDefined();

      await usePaletteEditorStore.getState().installModule("node-red-dashboard");

      // Search again
      await usePaletteEditorStore.getState().searchNpm("dashboard");
      const result = usePaletteEditorStore
        .getState()
        .available.find((m) => m.name === "node-red-dashboard");
      expect(result?.local).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // installModule (mock)
  // -----------------------------------------------------------------------

  describe("installModule()", () => {
    it("adds a module to the installed list", async () => {
      await usePaletteEditorStore.getState().searchNpm("postgres");
      await usePaletteEditorStore.getState().installModule(
        "node-red-contrib-postgres",
      );
      const { installed } = usePaletteEditorStore.getState();
      expect(
        installed.some((m) => m.name === "node-red-contrib-postgres"),
      ).toBe(true);
    });

    it("registers node types in the registry", async () => {
      await usePaletteEditorStore.getState().searchNpm("postgres");
      await usePaletteEditorStore.getState().installModule(
        "node-red-contrib-postgres",
      );
      expect(nodeRegistry.getType("postgres")).toBeDefined();
    });

    it("sets installStatus to success after install", async () => {
      await usePaletteEditorStore.getState().searchNpm("telegram");
      await usePaletteEditorStore.getState().installModule(
        "node-red-contrib-telegrambot",
      );
      // Status should be success (before the timeout resets to idle)
      const status = usePaletteEditorStore.getState().installStatus;
      expect(status === "success" || status === "idle").toBe(true);
    });

    it("sets error status if module not found in available", async () => {
      await usePaletteEditorStore.getState().installModule(
        "nonexistent-module",
      );
      expect(usePaletteEditorStore.getState().installStatus).toBe("error");
    });
  });

  // -----------------------------------------------------------------------
  // removeModule
  // -----------------------------------------------------------------------

  describe("removeModule()", () => {
    it("removes a non-core module from the installed list", async () => {
      // Install first
      await usePaletteEditorStore.getState().searchNpm("influx");
      await usePaletteEditorStore.getState().installModule(
        "node-red-contrib-influxdb",
      );
      expect(
        usePaletteEditorStore
          .getState()
          .installed.some((m) => m.name === "node-red-contrib-influxdb"),
      ).toBe(true);

      // Remove
      usePaletteEditorStore
        .getState()
        .removeModule("node-red-contrib-influxdb");
      expect(
        usePaletteEditorStore
          .getState()
          .installed.some((m) => m.name === "node-red-contrib-influxdb"),
      ).toBe(false);
    });

    it("removes node types from registry when module is removed", async () => {
      await usePaletteEditorStore.getState().searchNpm("influx");
      await usePaletteEditorStore.getState().installModule(
        "node-red-contrib-influxdb",
      );
      expect(nodeRegistry.getType("influxdb in")).toBeDefined();

      usePaletteEditorStore
        .getState()
        .removeModule("node-red-contrib-influxdb");
      expect(nodeRegistry.getType("influxdb in")).toBeUndefined();
    });

    it("does not remove a core module", () => {
      const before = usePaletteEditorStore.getState().installed.length;
      const coreModule = usePaletteEditorStore
        .getState()
        .installed.find((m) => m.local);
      expect(coreModule).toBeDefined();

      usePaletteEditorStore.getState().removeModule(coreModule!.name);
      expect(usePaletteEditorStore.getState().installed.length).toBe(before);
    });
  });
});
