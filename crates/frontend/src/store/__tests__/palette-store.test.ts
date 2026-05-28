import { describe, it, expect, beforeEach } from "vitest";
import { usePaletteStore } from "../palette-store";

function resetStore() {
  usePaletteStore.setState({
    searchQuery: "",
    expandedCategories: new Set(["common"]),
  });
}

describe("usePaletteStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty search query", () => {
      expect(usePaletteStore.getState().searchQuery).toBe("");
    });

    it("starts with 'common' category expanded", () => {
      expect(usePaletteStore.getState().expandedCategories.has("common")).toBe(
        true,
      );
    });

    it("starts with only 'common' expanded", () => {
      const expanded = usePaletteStore.getState().expandedCategories;
      expect(expanded.size).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // setSearchQuery
  // -----------------------------------------------------------------------

  describe("setSearchQuery()", () => {
    it("updates the search query", () => {
      usePaletteStore.getState().setSearchQuery("inject");
      expect(usePaletteStore.getState().searchQuery).toBe("inject");
    });

    it("can clear the search query", () => {
      usePaletteStore.getState().setSearchQuery("debug");
      usePaletteStore.getState().setSearchQuery("");
      expect(usePaletteStore.getState().searchQuery).toBe("");
    });
  });

  // -----------------------------------------------------------------------
  // toggleCategory
  // -----------------------------------------------------------------------

  describe("toggleCategory()", () => {
    it("expands a collapsed category", () => {
      expect(usePaletteStore.getState().expandedCategories.has("function")).toBe(
        false,
      );
      usePaletteStore.getState().toggleCategory("function");
      expect(usePaletteStore.getState().expandedCategories.has("function")).toBe(
        true,
      );
    });

    it("collapses an expanded category", () => {
      expect(usePaletteStore.getState().expandedCategories.has("common")).toBe(
        true,
      );
      usePaletteStore.getState().toggleCategory("common");
      expect(usePaletteStore.getState().expandedCategories.has("common")).toBe(
        false,
      );
    });

    it("can toggle multiple categories independently", () => {
      usePaletteStore.getState().toggleCategory("function");
      usePaletteStore.getState().toggleCategory("network");

      const expanded = usePaletteStore.getState().expandedCategories;
      expect(expanded.has("common")).toBe(true);
      expect(expanded.has("function")).toBe(true);
      expect(expanded.has("network")).toBe(true);

      // Collapse just function
      usePaletteStore.getState().toggleCategory("function");
      const expanded2 = usePaletteStore.getState().expandedCategories;
      expect(expanded2.has("function")).toBe(false);
      expect(expanded2.has("network")).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // isCategoryExpanded
  // -----------------------------------------------------------------------

  describe("isCategoryExpanded()", () => {
    it("returns true for expanded categories", () => {
      expect(usePaletteStore.getState().isCategoryExpanded("common")).toBe(true);
    });

    it("returns false for collapsed categories", () => {
      expect(
        usePaletteStore.getState().isCategoryExpanded("function"),
      ).toBe(false);
    });

    it("returns false for unknown categories", () => {
      expect(
        usePaletteStore.getState().isCategoryExpanded("nonexistent"),
      ).toBe(false);
    });
  });
});
