import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { PaletteEditorDialog } from "../PaletteEditorDialog";
import { usePaletteEditorStore } from "../../../store/palette-editor-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { registerBuiltinNodes } from "../../../red/nodes/builtin-nodes";

function resetStore() {
  usePaletteEditorStore.setState({
    installed: [],
    available: [],
    isLoading: false,
    installStatus: "idle",
  });
}

function renderDialog(overrides: { open?: boolean } = {}) {
  const onClose = vi.fn();
  const result = render(
    <PaletteEditorDialog
      open={overrides.open ?? true}
      onClose={onClose}
    />,
  );
  return { ...result, onClose };
}

describe("PaletteEditorDialog", () => {
  beforeEach(() => {
    nodeRegistry.clear();
    registerBuiltinNodes();
    resetStore();
    usePaletteEditorStore.getState().refreshInstalled();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders nothing when open=false", () => {
      renderDialog({ open: false });
      expect(screen.queryByTestId("palette-editor-dialog")).not.toBeInTheDocument();
    });

    it("renders the dialog when open=true", () => {
      renderDialog();
      expect(screen.getByTestId("palette-editor-dialog")).toBeInTheDocument();
    });

    it("renders the overlay", () => {
      renderDialog();
      expect(screen.getByTestId("palette-editor-overlay")).toBeInTheDocument();
    });

    it("renders the title 'Manage Palette'", () => {
      renderDialog();
      expect(screen.getByText("Manage Palette")).toBeInTheDocument();
    });

    it("renders Installed and Install tabs", () => {
      renderDialog();
      expect(screen.getByText("Installed")).toBeInTheDocument();
      expect(screen.getByText("Install")).toBeInTheDocument();
    });

    it("renders the Done button", () => {
      renderDialog();
      expect(screen.getByTestId("palette-editor-btn-done")).toBeInTheDocument();
    });

    it("renders the close button", () => {
      renderDialog();
      expect(screen.getByTestId("palette-editor-btn-close")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Close behavior
  // -----------------------------------------------------------------------

  describe("close behavior", () => {
    it("calls onClose when close button is clicked", () => {
      const { onClose } = renderDialog();
      fireEvent.click(screen.getByTestId("palette-editor-btn-close"));
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("calls onClose when Done button is clicked", () => {
      const { onClose } = renderDialog();
      fireEvent.click(screen.getByTestId("palette-editor-btn-done"));
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("calls onClose on Escape key", () => {
      const { onClose } = renderDialog();
      fireEvent.keyDown(window, { key: "Escape" });
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  // -----------------------------------------------------------------------
  // Installed tab
  // -----------------------------------------------------------------------

  describe("Installed tab", () => {
    it("shows installed module rows", () => {
      renderDialog();
      const rows = screen.getAllByTestId("palette-module-row");
      expect(rows.length).toBeGreaterThan(0);
    });

    it("shows module names", () => {
      renderDialog();
      const names = screen.getAllByTestId("palette-module-name");
      expect(names.length).toBeGreaterThan(0);
      // Should include a core category module
      const text = names.map((n) => n.textContent).join(",");
      expect(text).toContain("@node-red/");
    });

    it("shows 'built-in' badges for core modules", () => {
      renderDialog();
      const badges = screen.getAllByTestId("palette-module-badge");
      expect(badges.length).toBeGreaterThan(0);
      for (const badge of badges) {
        expect(badge.textContent).toBe("built-in");
      }
    });

    it("does not show remove buttons for core modules", () => {
      renderDialog();
      expect(screen.queryByTestId("palette-module-remove")).not.toBeInTheDocument();
    });

    it("shows node counts", () => {
      renderDialog();
      const nodeCounts = screen.getAllByTestId("palette-module-nodes");
      expect(nodeCounts.length).toBeGreaterThan(0);
    });
  });

  // -----------------------------------------------------------------------
  // Install tab
  // -----------------------------------------------------------------------

  describe("Install tab", () => {
    it("switches to Install tab on click", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      expect(screen.getByTestId("palette-npm-search")).toBeInTheDocument();
    });

    it("shows placeholder text before searching", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      expect(
        screen.getByText(/Search for Node-RED packages on npm/),
      ).toBeInTheDocument();
    });

    it("renders the search input", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      const input = screen.getByTestId("palette-npm-search");
      expect(input).toBeInTheDocument();
    });

    it("renders the search button", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      expect(screen.getByTestId("palette-npm-search-btn")).toBeInTheDocument();
    });

    it("shows search results after searching", async () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));

      const input = screen.getByTestId("palette-npm-search");
      fireEvent.change(input, { target: { value: "dashboard" } });
      fireEvent.click(screen.getByTestId("palette-npm-search-btn"));

      await waitFor(() => {
        expect(
          screen.getAllByTestId("palette-search-result").length,
        ).toBeGreaterThan(0);
      });
    });

    it("shows install buttons on search results", async () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));

      const input = screen.getByTestId("palette-npm-search");
      fireEvent.change(input, { target: { value: "dashboard" } });
      fireEvent.click(screen.getByTestId("palette-npm-search-btn"));

      await waitFor(() => {
        const buttons = screen.getAllByTestId("palette-search-install");
        expect(buttons.length).toBeGreaterThan(0);
        expect(buttons[0].textContent).toContain("Install");
      });
    });
  });

  // -----------------------------------------------------------------------
  // Tab switching
  // -----------------------------------------------------------------------

  describe("tab switching", () => {
    it("shows Installed tab content by default", () => {
      renderDialog();
      expect(screen.getByTestId("palette-installed-list")).toBeInTheDocument();
    });

    it("hides Installed tab content when Install tab is active", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      expect(screen.queryByTestId("palette-installed-list")).not.toBeInTheDocument();
    });

    it("returns to Installed tab on click", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Install"));
      fireEvent.click(screen.getByText("Installed"));
      expect(screen.getByTestId("palette-installed-list")).toBeInTheDocument();
    });
  });
});
