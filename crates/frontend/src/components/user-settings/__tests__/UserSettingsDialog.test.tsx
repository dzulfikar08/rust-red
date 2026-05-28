/**
 * Tests for UserSettingsDialog
 *
 * Covers: rendering tabs, theme change, grid size editing,
 * keyboard shortcuts tab, about tab, save/cancel flow.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { UserSettingsDialog } from "../UserSettingsDialog";
import { useSettingsStore } from "../../../store/settings-store";

// ---------------------------------------------------------------------------
// Mock theme store
// ---------------------------------------------------------------------------

const mockSetTheme = vi.fn();

vi.mock("../../../store/theme-store", () => ({
  useThemeStore: (selector?: (state: { theme: string; setTheme: ReturnType<typeof vi.fn>; toggleTheme: ReturnType<typeof vi.fn> }) => unknown) => {
    const state = {
      theme: "dark",
      setTheme: mockSetTheme,
      toggleTheme: vi.fn(),
    };
    return selector ? selector(state) : state;
  },
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderDialog(overrides: Partial<Parameters<typeof UserSettingsDialog>[0]> = {}) {
  return render(
    <UserSettingsDialog
      open={true}
      onClose={vi.fn()}
      {...overrides}
    />,
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("UserSettingsDialog", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset settings store to defaults
    useSettingsStore.setState({
      theme: "system",
      locale: "en",
      editorPreferences: { gridSize: 20, snapToGrid: false, showTips: true },
      sidebarWidth: 300,
      paletteCategoriesExpanded: {},
      _generic: {},
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders nothing when open=false", () => {
      render(<UserSettingsDialog open={false} onClose={onClose} />);
      expect(screen.queryByTestId("user-settings-dialog")).not.toBeInTheDocument();
    });

    it("renders the dialog when open=true", () => {
      renderDialog();
      expect(screen.getByTestId("user-settings-dialog")).toBeInTheDocument();
    });

    it("renders the overlay", () => {
      renderDialog();
      expect(screen.getByTestId("user-settings-overlay")).toBeInTheDocument();
    });

    it("renders all four tab buttons", () => {
      renderDialog();
      expect(screen.getByText("Editor")).toBeInTheDocument();
      expect(screen.getByText("Appearance")).toBeInTheDocument();
      expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
      expect(screen.getByText("About")).toBeInTheDocument();
    });

    it("renders Save and Cancel buttons", () => {
      renderDialog();
      expect(screen.getByTestId("user-settings-btn-save")).toBeInTheDocument();
      expect(screen.getByTestId("user-settings-btn-cancel")).toBeInTheDocument();
    });

    it("renders the close button in the header", () => {
      renderDialog();
      expect(screen.getByTestId("user-settings-btn-close")).toBeInTheDocument();
    });

    it("shows Editor tab by default", () => {
      renderDialog();
      // Editor tab has grid size input
      expect(screen.getByTestId("settings-grid-size")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Tab switching
  // -----------------------------------------------------------------------

  describe("tab switching", () => {
    it("switches to Appearance tab", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      // Appearance tab has theme radios
      expect(screen.getByTestId("settings-theme-dark")).toBeInTheDocument();
      expect(screen.getByTestId("settings-theme-light")).toBeInTheDocument();
      expect(screen.getByTestId("settings-theme-system")).toBeInTheDocument();
      expect(screen.getByTestId("settings-locale")).toBeInTheDocument();
    });

    it("switches to Keyboard Shortcuts tab", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Keyboard Shortcuts"));
      expect(screen.getByTestId("settings-shortcut-search")).toBeInTheDocument();
      // Should show shortcut rows
      expect(screen.getByText("Undo")).toBeInTheDocument();
    });

    it("switches to About tab", () => {
      renderDialog();
      fireEvent.click(screen.getByText("About"));
      expect(screen.getByTestId("settings-about-version")).toBeInTheDocument();
      expect(screen.getByTestId("settings-about-nr-compat")).toBeInTheDocument();
    });

    it("respects initialTab prop", () => {
      renderDialog({ initialTab: "about" });
      // About tab content should be visible immediately
      expect(screen.getByTestId("settings-about-version")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Editor tab interactions
  // -----------------------------------------------------------------------

  describe("Editor tab", () => {
    it("displays current grid size", () => {
      renderDialog();
      const input = screen.getByTestId("settings-grid-size") as HTMLInputElement;
      expect(input.value).toBe("20");
    });

    it("allows changing grid size", () => {
      renderDialog();
      const input = screen.getByTestId("settings-grid-size") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "40" } });
      expect(input.value).toBe("40");
    });

    it("displays snap to grid checkbox", () => {
      renderDialog();
      const cb = screen.getByTestId("settings-snap-to-grid") as HTMLInputElement;
      expect(cb.checked).toBe(false);
    });

    it("allows toggling snap to grid", () => {
      renderDialog();
      const cb = screen.getByTestId("settings-snap-to-grid") as HTMLInputElement;
      fireEvent.click(cb);
      expect(cb.checked).toBe(true);
    });

    it("displays show tips checkbox checked by default", () => {
      renderDialog();
      const cb = screen.getByTestId("settings-show-tips") as HTMLInputElement;
      expect(cb.checked).toBe(true);
    });

    it("displays zoom level", () => {
      renderDialog();
      expect(screen.getByTestId("settings-zoom-level")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Appearance tab interactions
  // -----------------------------------------------------------------------

  describe("Appearance tab", () => {
    it("shows current theme as selected", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      const systemRadio = screen.getByTestId("settings-theme-system") as HTMLInputElement;
      expect(systemRadio.checked).toBe(true);
    });

    it("allows changing theme selection", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      const darkRadio = screen.getByTestId("settings-theme-dark") as HTMLInputElement;
      fireEvent.click(darkRadio);
      expect(darkRadio.checked).toBe(true);
    });

    it("shows current locale in dropdown", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      const select = screen.getByTestId("settings-locale") as HTMLSelectElement;
      expect(select.value).toBe("en");
    });

    it("allows changing language", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      const select = screen.getByTestId("settings-locale") as HTMLSelectElement;
      fireEvent.change(select, { target: { value: "de" } });
      expect(select.value).toBe("de");
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard Shortcuts tab
  // -----------------------------------------------------------------------

  describe("Keyboard Shortcuts tab", () => {
    it("shows all default shortcuts", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Keyboard Shortcuts"));
      // Check for known actions (some appear multiple times due to multiple keybindings)
      expect(screen.getByText("Undo")).toBeInTheDocument();
      expect(screen.getAllByText("Redo").length).toBeGreaterThanOrEqual(2);
      expect(screen.getAllByText("Delete Selected").length).toBeGreaterThanOrEqual(2);
      expect(screen.getByText("Deselect")).toBeInTheDocument();
    });

    it("filters shortcuts by action name", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Keyboard Shortcuts"));
      const search = screen.getByTestId("settings-shortcut-search");
      fireEvent.change(search, { target: { value: "undo" } });
      expect(screen.getByText("Undo")).toBeInTheDocument();
      expect(screen.queryByText("Redo")).not.toBeInTheDocument();
    });

    it("filters shortcuts by shortcut key", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Keyboard Shortcuts"));
      const search = screen.getByTestId("settings-shortcut-search");
      // The shortcut for Deselect is displayed as "Esc" (keyMap transforms Escape -> Esc)
      fireEvent.change(search, { target: { value: "esc" } });
      expect(screen.getByText("Deselect")).toBeInTheDocument();
    });

    it("shows no results message when filter matches nothing", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Keyboard Shortcuts"));
      const search = screen.getByTestId("settings-shortcut-search");
      fireEvent.change(search, { target: { value: "zzzznonexistent" } });
      expect(screen.getByText("No shortcuts found")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // About tab
  // -----------------------------------------------------------------------

  describe("About tab", () => {
    it("displays application name", () => {
      renderDialog({ initialTab: "about" });
      expect(screen.getByText("Rust-RED")).toBeInTheDocument();
    });

    it("displays version info", () => {
      renderDialog({ initialTab: "about" });
      expect(screen.getByTestId("settings-about-version")).toHaveTextContent("0.1.0");
    });

    it("displays Node-RED compatibility version", () => {
      renderDialog({ initialTab: "about" });
      expect(screen.getByTestId("settings-about-nr-compat")).toHaveTextContent(
        "3.1.x",
      );
    });

    it("displays documentation link", () => {
      renderDialog({ initialTab: "about" });
      expect(screen.getByText("Documentation")).toBeInTheDocument();
    });

    it("displays repository link", () => {
      renderDialog({ initialTab: "about" });
      expect(screen.getByText("Repository")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Save / Cancel
  // -----------------------------------------------------------------------

  describe("Save and Cancel", () => {
    it("calls onClose when Cancel is clicked", () => {
      const onCloseFn = vi.fn();
      renderDialog({ onClose: onCloseFn });
      fireEvent.click(screen.getByTestId("user-settings-btn-cancel"));
      expect(onCloseFn).toHaveBeenCalledOnce();
    });

    it("calls onClose when close button (X) is clicked", () => {
      const onCloseFn = vi.fn();
      renderDialog({ onClose: onCloseFn });
      fireEvent.click(screen.getByTestId("user-settings-btn-close"));
      expect(onCloseFn).toHaveBeenCalledOnce();
    });

    it("persists editor preferences on Save", () => {
      renderDialog();
      // Change grid size
      const input = screen.getByTestId("settings-grid-size") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "40" } });

      // Toggle snap to grid
      const cb = screen.getByTestId("settings-snap-to-grid") as HTMLInputElement;
      fireEvent.click(cb);

      // Save
      fireEvent.click(screen.getByTestId("user-settings-btn-save"));

      // Verify store was updated
      const state = useSettingsStore.getState();
      expect(state.editorPreferences.gridSize).toBe(40);
      expect(state.editorPreferences.snapToGrid).toBe(true);
    });

    it("persists theme on Save", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      fireEvent.click(screen.getByTestId("settings-theme-dark"));
      fireEvent.click(screen.getByTestId("user-settings-btn-save"));

      expect(useSettingsStore.getState().theme).toBe("dark");
    });

    it("persists locale on Save", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      fireEvent.change(screen.getByTestId("settings-locale"), {
        target: { value: "de" },
      });
      fireEvent.click(screen.getByTestId("user-settings-btn-save"));

      expect(useSettingsStore.getState().locale).toBe("de");
    });

    it("syncs runtime theme store on Save", () => {
      renderDialog();
      fireEvent.click(screen.getByText("Appearance"));
      fireEvent.click(screen.getByTestId("settings-theme-dark"));
      fireEvent.click(screen.getByTestId("user-settings-btn-save"));

      expect(mockSetTheme).toHaveBeenCalledWith("dark");
    });

    it("calls onClose after Save", () => {
      const onCloseFn = vi.fn();
      renderDialog({ onClose: onCloseFn });
      fireEvent.click(screen.getByTestId("user-settings-btn-save"));
      expect(onCloseFn).toHaveBeenCalledOnce();
    });

    it("reverts changes on Cancel (does not update store)", () => {
      renderDialog();
      // Change grid size
      const input = screen.getByTestId("settings-grid-size") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "50" } });

      // Cancel
      fireEvent.click(screen.getByTestId("user-settings-btn-cancel"));

      // Store should still have default
      expect(useSettingsStore.getState().editorPreferences.gridSize).toBe(20);
    });
  });
});
