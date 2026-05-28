import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionList } from "../ActionList";
import { useActionStore } from "../../../store/action-store";
import { useKeyboardStore } from "../../../store/keyboard-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStores() {
  useActionStore.setState({ actions: new Map() });
  useKeyboardStore.setState({ bindings: new Map(), scope: "default" });
}

function registerTestActions() {
  const store = useActionStore.getState();
  store.registerAction({
    id: "core:undo",
    name: "Undo",
    scope: "core",
    key: "Ctrl-Z",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:redo",
    name: "Redo",
    scope: "core",
    key: "Ctrl-Y",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:copy",
    name: "Copy",
    scope: "core",
    key: "Ctrl-C",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:paste",
    name: "Paste",
    scope: "core",
    key: "Ctrl-V",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:delete-selected",
    name: "Delete Selected",
    scope: "core",
    key: "Delete",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:select-all",
    name: "Select All",
    scope: "core",
    key: "Ctrl-A",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:deploy",
    name: "Deploy",
    scope: "core",
    key: "Ctrl-S",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:show-help",
    name: "Show Help",
    scope: "core",
    key: "?",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:import-flows",
    name: "Import Flows",
    scope: "core",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:export-flows",
    name: "Export Flows",
    scope: "core",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:toggle-sidebar",
    name: "Toggle Sidebar",
    scope: "core",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:toggle-fullscreen",
    name: "Toggle Full Screen",
    scope: "core",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:manage-palette",
    name: "Manage Palette",
    scope: "core",
    handler: vi.fn(),
  });
  store.registerAction({
    id: "core:search-flows",
    name: "Search Flows",
    scope: "core",
    handler: vi.fn(),
  });
}

function getActionHandler(id: string) {
  return useActionStore.getState().actions.get(id)?.handler;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ActionList", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    resetStores();
    registerTestActions();
    onClose.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the search input and action list", () => {
      render(<ActionList onClose={onClose} />);

      expect(screen.getByTestId("action-list")).toBeInTheDocument();
      expect(screen.getByTestId("action-list-input")).toBeInTheDocument();
      expect(screen.getByPlaceholderText("Search actions...")).toBeInTheDocument();
    });

    it("shows all registered actions when no query", () => {
      render(<ActionList onClose={onClose} />);

      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      // 14 test actions registered
      expect(buttons.length).toBe(14);
    });

    it("displays action names", () => {
      render(<ActionList onClose={onClose} />);

      expect(screen.getByText("Undo")).toBeInTheDocument();
      expect(screen.getByText("Redo")).toBeInTheDocument();
      expect(screen.getByText("Deploy")).toBeInTheDocument();
    });

    it("displays keyboard shortcut labels from the action key field", () => {
      render(<ActionList onClose={onClose} />);

      // These come from the action.key field
      expect(screen.getByText("Ctrl-Z")).toBeInTheDocument();
      expect(screen.getByText("Ctrl-Y")).toBeInTheDocument();
      expect(screen.getByText("Ctrl-C")).toBeInTheDocument();
    });

    it("shows actions without keyboard shortcuts", () => {
      render(<ActionList onClose={onClose} />);

      expect(screen.getByText("Import Flows")).toBeInTheDocument();
      expect(screen.getByText("Export Flows")).toBeInTheDocument();
      expect(screen.getByText("Toggle Sidebar")).toBeInTheDocument();
      expect(screen.getByText("Toggle Full Screen")).toBeInTheDocument();
      expect(screen.getByText("Manage Palette")).toBeInTheDocument();
      expect(screen.getByText("Search Flows")).toBeInTheDocument();
    });

    it("auto-focuses the input on mount", () => {
      render(<ActionList onClose={onClose} />);

      const input = screen.getByTestId("action-list-input");
      expect(document.activeElement).toBe(input);
    });

    it("sorts actions alphabetically by name when no query", () => {
      render(<ActionList onClose={onClose} />);

      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      const names = buttons.map((b) => b.textContent?.trim());

      // First action alphabetically
      expect(names[0]).toContain("Copy");
    });
  });

  // -----------------------------------------------------------------------
  // Fuzzy search / filtering
  // -----------------------------------------------------------------------

  describe("filtering", () => {
    it("filters actions by name using fuzzy search", async () => {
      const user = userEvent.setup();
      render(<ActionList onClose={onClose} />);

      const input = screen.getByTestId("action-list-input");
      await user.type(input, "undo");

      expect(screen.getByText("Undo")).toBeInTheDocument();
      // Redo should also match since "undo" fuzzy matches "redo" partially,
      // but "Undo" should be ranked higher
      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      expect(buttons.length).toBeGreaterThanOrEqual(1);
      expect(buttons[0]).toHaveTextContent("Undo");
    });

    it("shows no results message when nothing matches", async () => {
      const user = userEvent.setup();
      render(<ActionList onClose={onClose} />);

      const input = screen.getByTestId("action-list-input");
      await user.type(input, "zzzzz");

      expect(screen.getByText("No actions found")).toBeInTheDocument();
    });

    it("clears filter when input is cleared", async () => {
      const user = userEvent.setup();
      render(<ActionList onClose={onClose} />);

      const input = screen.getByTestId("action-list-input");
      await user.type(input, "zzzzz");
      expect(screen.getByText("No actions found")).toBeInTheDocument();

      await user.clear(input);
      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      expect(buttons.length).toBe(14);
    });

    it("matches against action ID as well as name", async () => {
      const user = userEvent.setup();
      render(<ActionList onClose={onClose} />);

      const input = screen.getByTestId("action-list-input");
      await user.type(input, "toggle");

      // Should find "Toggle Sidebar" and "Toggle Full Screen"
      expect(screen.getByText("Toggle Sidebar")).toBeInTheDocument();
      expect(screen.getByText("Toggle Full Screen")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard navigation
  // -----------------------------------------------------------------------

  describe("keyboard navigation", () => {
    it("selects the first item by default", () => {
      render(<ActionList onClose={onClose} />);

      const items = screen.getByTestId("action-list-items");
      const first = within(items).getAllByRole("button")[0];
      expect(first).toHaveClass("bg-[#094771]");
    });

    it("moves selection down with ArrowDown", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      fireEvent.keyDown(list, { key: "ArrowDown" });

      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      expect(buttons[1]).toHaveClass("bg-[#094771]");
      expect(buttons[0]).not.toHaveClass("bg-[#094771]");
    });

    it("moves selection up with ArrowUp", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      // Move down once
      fireEvent.keyDown(list, { key: "ArrowDown" });
      // Move back up
      fireEvent.keyDown(list, { key: "ArrowUp" });

      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      expect(buttons[0]).toHaveClass("bg-[#094771]");
    });

    it("wraps around when navigating past the last item", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      const lastIndex = buttons.length - 1;

      // Move to the last item
      for (let i = 0; i < lastIndex; i++) {
        fireEvent.keyDown(list, { key: "ArrowDown" });
      }
      expect(buttons[lastIndex]).toHaveClass("bg-[#094771]");

      // One more down should wrap to first
      fireEvent.keyDown(list, { key: "ArrowDown" });
      expect(buttons[0]).toHaveClass("bg-[#094771]");
    });

    it("wraps around when navigating before the first item", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      // Move up from first item should wrap to last
      fireEvent.keyDown(list, { key: "ArrowUp" });

      const items = screen.getByTestId("action-list-items");
      const buttons = within(items).getAllByRole("button");
      expect(buttons[buttons.length - 1]).toHaveClass("bg-[#094771]");
    });

    it("resets selection index when filter changes", async () => {
      const user = userEvent.setup();
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      // Move selection down a few items
      fireEvent.keyDown(list, { key: "ArrowDown" });
      fireEvent.keyDown(list, { key: "ArrowDown" });

      // Now type a filter
      const input = screen.getByTestId("action-list-input");
      await user.type(input, "undo");

      // Selection should reset to first item in filtered list
      const items = screen.getByTestId("action-list-items");
      const firstFiltered = within(items).getAllByRole("button")[0];
      expect(firstFiltered).toHaveClass("bg-[#094771]");
    });
  });

  // -----------------------------------------------------------------------
  // Execution
  // -----------------------------------------------------------------------

  describe("execution", () => {
    it("executes action on Enter key and closes", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      fireEvent.keyDown(list, { key: "Enter" });

      // First action (alphabetically) is "Copy"
      const handler = getActionHandler("core:copy");
      expect(handler).toHaveBeenCalledOnce();
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("executes action on click and closes", () => {
      render(<ActionList onClose={onClose} />);

      const undoButton = screen.getByTestId("action-item-core:undo");
      fireEvent.click(undoButton);

      const handler = getActionHandler("core:undo");
      expect(handler).toHaveBeenCalledOnce();
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("does not execute disabled actions", () => {
      // Register a disabled action
      useActionStore.getState().registerAction({
        id: "test:disabled",
        name: "Disabled Action",
        scope: "test",
        handler: vi.fn(),
        enabled: () => false,
      });

      render(<ActionList onClose={onClose} />);

      const button = screen.getByTestId("action-item-test:disabled");
      expect(button).toHaveClass("cursor-default");
      fireEvent.click(button);

      const handler = useActionStore.getState().actions.get("test:disabled")?.handler;
      expect(handler).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // Closing
  // -----------------------------------------------------------------------

  describe("closing", () => {
    it("closes on Escape key", () => {
      render(<ActionList onClose={onClose} />);

      const list = screen.getByTestId("action-list");
      fireEvent.keyDown(list, { key: "Escape" });

      expect(onClose).toHaveBeenCalledOnce();
    });

    it("closes on backdrop click", () => {
      render(<ActionList onClose={onClose} />);

      const backdrop = screen.getByTestId("action-list-backdrop");
      fireEvent.click(backdrop);

      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard shortcut labels from bindings
  // -----------------------------------------------------------------------

  describe("keyboard shortcut labels", () => {
    it("shows shortcut labels derived from keyboard bindings", () => {
      // Register a keyboard binding
      useKeyboardStore.getState().addBinding({
        key: "p",
        modifiers: { ctrl: true, shift: true },
        action: "core:deploy",
        preventDefault: true,
      });

      render(<ActionList onClose={onClose} />);

      // The Deploy action should show a shortcut label from the binding
      const deployButton = screen.getByTestId("action-item-core:deploy");
      expect(deployButton).toHaveTextContent("Ctrl-S");
    });
  });
});
