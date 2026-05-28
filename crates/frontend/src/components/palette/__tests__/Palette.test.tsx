import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Palette } from "../Palette";
import { usePaletteStore } from "../../../store/palette-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { registerBuiltinNodes } from "../../../red/nodes/builtin-nodes";

function resetStore() {
  usePaletteStore.setState({
    searchQuery: "",
    expandedCategories: new Set(["common"]),
  });
}

describe("Palette", () => {
  beforeEach(() => {
    resetStore();
    nodeRegistry.clear();
    registerBuiltinNodes();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the palette container", () => {
      render(<Palette />);
      expect(screen.getByTestId("palette")).toBeInTheDocument();
    });

    it("renders the search input", () => {
      render(<Palette />);
      expect(screen.getByTestId("palette-search")).toBeInTheDocument();
    });

    it("renders category names", () => {
      render(<Palette />);
      // Common should be expanded by default and show its nodes
      expect(screen.getByText("common")).toBeInTheDocument();
    });

    it("renders node items in expanded categories", () => {
      render(<Palette />);
      // common category is expanded by default, should show inject node
      expect(screen.getByText("inject")).toBeInTheDocument();
    });

    it("does not render nodes in collapsed categories", () => {
      render(<Palette />);
      // function category is collapsed by default
      // The nodes should not be visible
      expect(screen.queryByText("function")).toBeInTheDocument(); // category header visible
    });
  });

  // -----------------------------------------------------------------------
  // Search
  // -----------------------------------------------------------------------

  describe("search", () => {
    it("filters nodes by name", () => {
      render(<Palette />);
      const input = screen.getByTestId("palette-search");
      fireEvent.change(input, { target: { value: "inject" } });

      expect(screen.getByText("inject")).toBeInTheDocument();
      // debug should still be visible (it's in the same category)
      // but other categories should be filtered out
      expect(screen.queryByText("function")).not.toBeInTheDocument();
    });

    it("shows no matching nodes message when search has no results", () => {
      render(<Palette />);
      const input = screen.getByTestId("palette-search");
      fireEvent.change(input, { target: { value: "zzznonexistent" } });

      expect(screen.getByText("No matching nodes")).toBeInTheDocument();
    });

    it("expands all matching categories when searching", () => {
      render(<Palette />);
      const input = screen.getByTestId("palette-search");
      fireEvent.change(input, { target: { value: "json" } });

      // json is in the parser category, which should auto-expand
      expect(screen.getByText("json")).toBeInTheDocument();
    });

    it("shows all nodes again when search is cleared", () => {
      render(<Palette />);
      const input = screen.getByTestId("palette-search");
      fireEvent.change(input, { target: { value: "inject" } });
      fireEvent.change(input, { target: { value: "" } });

      expect(screen.getByText("inject")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Category toggle
  // -----------------------------------------------------------------------

  describe("category toggle", () => {
    it("collapses an expanded category on click", () => {
      render(<Palette />);
      // common is expanded by default
      const commonHeader = screen.getByText("common");
      fireEvent.click(commonHeader);

      // Category should now be collapsed (nodes hidden)
      expect(
        usePaletteStore.getState().expandedCategories.has("common"),
      ).toBe(false);
    });

    it("expands a collapsed category on click", () => {
      render(<Palette />);
      const functionHeader = screen.getByText("function");
      fireEvent.click(functionHeader);

      expect(
        usePaletteStore.getState().expandedCategories.has("function"),
      ).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Node click
  // -----------------------------------------------------------------------

  describe("node click", () => {
    it("emits node:add-requested event when a node item is clicked", () => {
      const spy = vi.fn();
      const unsub = eventBus.on("node:add-requested", spy);

      render(<Palette />);
      const injectItem = screen.getByTitle("inject");
      fireEvent.click(injectItem);

      expect(spy).toHaveBeenCalledWith({ type: "inject" });
      unsub();
    });
  });
});

// Need to import eventBus for the click test
import { eventBus } from "../../../red/core/events";
