import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useSettingsStore } from "../settings-store";
import type { Theme, EditorPreferences } from "../settings-store";

/**
 * Helper: reset the Zustand store to defaults between tests.
 * We rehydrate from a clean localStorage.
 */
function resetStore() {
  localStorage.removeItem("rust-red-settings");
  useSettingsStore.setState({
    theme: "system",
    locale: "en",
    editorPreferences: {
      gridSize: 20,
      snapToGrid: false,
      showTips: true,
    },
    sidebarWidth: 300,
    paletteCategoriesExpanded: {},
    _generic: {},
  });
}

describe("useSettingsStore", () => {
  beforeEach(() => {
    resetStore();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("has correct defaults", () => {
      const state = useSettingsStore.getState();
      expect(state.theme).toBe("system");
      expect(state.locale).toBe("en");
      expect(state.editorPreferences).toEqual({
        gridSize: 20,
        snapToGrid: false,
        showTips: true,
      });
      expect(state.sidebarWidth).toBe(300);
      expect(state.paletteCategoriesExpanded).toEqual({});
    });
  });

  // -----------------------------------------------------------------------
  // setTheme
  // -----------------------------------------------------------------------

  describe("setTheme()", () => {
    it("sets the theme to dark", () => {
      useSettingsStore.getState().setTheme("dark");
      expect(useSettingsStore.getState().theme).toBe("dark");
    });

    it("sets the theme to light", () => {
      useSettingsStore.getState().setTheme("light");
      expect(useSettingsStore.getState().theme).toBe("light");
    });

    it("sets the theme to system", () => {
      useSettingsStore.getState().setTheme("system");
      expect(useSettingsStore.getState().theme).toBe("system");
    });
  });

  // -----------------------------------------------------------------------
  // setLocale
  // -----------------------------------------------------------------------

  describe("setLocale()", () => {
    it("sets the locale", () => {
      useSettingsStore.getState().setLocale("de");
      expect(useSettingsStore.getState().locale).toBe("de");
    });
  });

  // -----------------------------------------------------------------------
  // updateEditorPreference
  // -----------------------------------------------------------------------

  describe("updateEditorPreference()", () => {
    it("updates a single preference without affecting others", () => {
      useSettingsStore.getState().updateEditorPreference("gridSize", 40);
      const prefs = useSettingsStore.getState().editorPreferences;
      expect(prefs.gridSize).toBe(40);
      expect(prefs.snapToGrid).toBe(false);
      expect(prefs.showTips).toBe(true);
    });

    it("updates snapToGrid", () => {
      useSettingsStore.getState().updateEditorPreference("snapToGrid", true);
      expect(useSettingsStore.getState().editorPreferences.snapToGrid).toBe(true);
    });

    it("updates showTips", () => {
      useSettingsStore.getState().updateEditorPreference("showTips", false);
      expect(useSettingsStore.getState().editorPreferences.showTips).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // setSidebarWidth
  // -----------------------------------------------------------------------

  describe("setSidebarWidth()", () => {
    it("sets the sidebar width", () => {
      useSettingsStore.getState().setSidebarWidth(450);
      expect(useSettingsStore.getState().sidebarWidth).toBe(450);
    });
  });

  // -----------------------------------------------------------------------
  // togglePaletteCategory
  // -----------------------------------------------------------------------

  describe("togglePaletteCategory()", () => {
    it("toggles a category from undefined to true", () => {
      useSettingsStore.getState().togglePaletteCategory("function");
      expect(useSettingsStore.getState().paletteCategoriesExpanded["function"]).toBe(true);
    });

    it("toggles a category from true to false", () => {
      useSettingsStore.getState().togglePaletteCategory("function");
      useSettingsStore.getState().togglePaletteCategory("function");
      expect(useSettingsStore.getState().paletteCategoriesExpanded["function"]).toBe(false);
    });

    it("does not affect other categories", () => {
      useSettingsStore.getState().togglePaletteCategory("function");
      useSettingsStore.getState().togglePaletteCategory("network");
      const expanded = useSettingsStore.getState().paletteCategoriesExpanded;
      expect(expanded["function"]).toBe(true);
      expect(expanded["network"]).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Generic get / set
  // -----------------------------------------------------------------------

  describe("get() / set()", () => {
    it("get() returns typed fields by name", () => {
      expect(useSettingsStore.getState().get<Theme>("theme")).toBe("system");
      expect(useSettingsStore.getState().get<string>("locale")).toBe("en");
      expect(useSettingsStore.getState().get<number>("sidebarWidth")).toBe(300);
    });

    it("set() updates typed fields by name", () => {
      useSettingsStore.getState().set("theme", "dark");
      expect(useSettingsStore.getState().theme).toBe("dark");

      useSettingsStore.getState().set("locale", "fr");
      expect(useSettingsStore.getState().locale).toBe("fr");

      useSettingsStore.getState().set("sidebarWidth", 500);
      expect(useSettingsStore.getState().sidebarWidth).toBe(500);
    });

    it("get() returns undefined for unknown keys", () => {
      expect(useSettingsStore.getState().get("nonexistent")).toBeUndefined();
    });

    it("set() stores and get() retrieves arbitrary values", () => {
      useSettingsStore.getState().set("customKey", { foo: "bar" });
      expect(useSettingsStore.getState().get<{ foo: string }>("customKey")).toEqual({ foo: "bar" });
    });

    it("set() overwrites generic values", () => {
      useSettingsStore.getState().set("mySetting", "v1");
      useSettingsStore.getState().set("mySetting", "v2");
      expect(useSettingsStore.getState().get<string>("mySetting")).toBe("v2");
    });

    it("set() handles editorPreferences via generic path", () => {
      const newPrefs: EditorPreferences = { gridSize: 50, snapToGrid: true, showTips: false };
      useSettingsStore.getState().set("editorPreferences", newPrefs);
      expect(useSettingsStore.getState().editorPreferences).toEqual(newPrefs);
    });

    it("set() handles paletteCategoriesExpanded via generic path", () => {
      useSettingsStore.getState().set("paletteCategoriesExpanded", { input: true });
      expect(useSettingsStore.getState().paletteCategoriesExpanded).toEqual({ input: true });
    });
  });

  // -----------------------------------------------------------------------
  // Persistence
  // -----------------------------------------------------------------------

  describe("persistence", () => {
    it("persists state to localStorage", () => {
      useSettingsStore.getState().setTheme("dark");
      useSettingsStore.getState().setLocale("ja");

      // Zustand persist writes asynchronously; flush
      // We check the raw localStorage value
      const raw = localStorage.getItem("rust-red-settings");
      expect(raw).not.toBeNull();
      const parsed = JSON.parse(raw!);
      expect(parsed.state.theme).toBe("dark");
      expect(parsed.state.locale).toBe("ja");
    });

    it("restores state from localStorage", () => {
      // Seed localStorage with persisted data
      const persisted = {
        state: {
          theme: "light",
          locale: "es",
          editorPreferences: { gridSize: 30, snapToGrid: true, showTips: false },
          sidebarWidth: 400,
          paletteCategoriesExpanded: { output: true },
          _generic: { myVal: 42 },
        },
        version: 0,
      };
      localStorage.setItem("rust-red-settings", JSON.stringify(persisted));

      // Re-create the store by triggering rehydration
      useSettingsStore.persist.rehydrate();

      // Note: rehydration may be async. Let's check after a tick.
      // Since we're in sync test, we verify the store picked it up.
      const state = useSettingsStore.getState();
      expect(state.theme).toBe("light");
      expect(state.locale).toBe("es");
      expect(state.editorPreferences.gridSize).toBe(30);
      expect(state.sidebarWidth).toBe(400);
      expect(state.paletteCategoriesExpanded["output"]).toBe(true);
    });
  });
});
