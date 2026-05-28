/**
 * Tests for ContextMenu component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { ContextMenu } from "../ContextMenu";
import type { MenuItem } from "../../../store/context-menu-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const defaultItems: MenuItem[] = [
  { label: "Cut", shortcut: "Ctrl-X", action: vi.fn() },
  { label: "Copy", shortcut: "Ctrl-C", action: vi.fn() },
  { label: "", separator: true },
  { label: "Delete", action: vi.fn() },
];

function renderMenu(
  items: MenuItem[] = defaultItems,
  overrides: { x?: number; y?: number; onClose?: () => void } = {},
) {
  const onClose = overrides.onClose ?? vi.fn();
  const result = render(
    <ContextMenu
      x={overrides.x ?? 100}
      y={overrides.y ?? 100}
      items={items}
      onClose={onClose}
    />,
  );
  return { ...result, onClose };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ContextMenu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the context menu container", () => {
      renderMenu();
      expect(screen.getByTestId("context-menu")).toBeInTheDocument();
    });

    it("renders all non-separator items", () => {
      renderMenu();
      expect(screen.getByTestId("context-menu-item-cut")).toBeInTheDocument();
      expect(screen.getByTestId("context-menu-item-copy")).toBeInTheDocument();
      expect(screen.getByTestId("context-menu-item-delete")).toBeInTheDocument();
    });

    it("renders shortcut text", () => {
      renderMenu();
      expect(screen.getByText("Ctrl-X")).toBeInTheDocument();
      expect(screen.getByText("Ctrl-C")).toBeInTheDocument();
    });

    it("renders separator elements", () => {
      const { container } = renderMenu();
      const separators = container.querySelectorAll("[role='separator']");
      expect(separators).toHaveLength(1);
    });

    it("renders items with icons", () => {
      const items: MenuItem[] = [
        {
          label: "Test",
          icon: <span data-testid="test-icon">X</span>,
          action: vi.fn(),
        },
      ];
      renderMenu(items);
      expect(screen.getByTestId("test-icon")).toBeInTheDocument();
    });

    it("renders disabled items", () => {
      const items: MenuItem[] = [
        { label: "Enabled", action: vi.fn() },
        { label: "Disabled", action: vi.fn(), disabled: true },
      ];
      renderMenu(items);
      const disabledItem = screen.getByTestId("context-menu-item-disabled");
      expect(disabledItem).toHaveAttribute("aria-disabled", "true");
    });

    it("returns null for empty items array", () => {
      const { container } = renderMenu([]);
      expect(container.firstChild).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // Click interactions
  // -----------------------------------------------------------------------

  describe("click interactions", () => {
    it("calls action and onClose when item is clicked", () => {
      const action = vi.fn();
      const items: MenuItem[] = [{ label: "Cut", action }];
      const { onClose } = renderMenu(items);

      fireEvent.click(screen.getByTestId("context-menu-item-cut"));
      expect(action).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("does not call action when disabled item is clicked", () => {
      const action = vi.fn();
      const items: MenuItem[] = [
        { label: "Disabled", action, disabled: true },
      ];
      const { onClose } = renderMenu(items);

      fireEvent.click(screen.getByTestId("context-menu-item-disabled"));
      expect(action).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // Click outside to close
  // -----------------------------------------------------------------------

  describe("click outside to close", () => {
    it("calls onClose when clicking outside the menu", async () => {
      const onClose = vi.fn();
      render(
        <div>
          <div data-testid="outside">Outside</div>
          <ContextMenu
            x={100}
            y={100}
            items={defaultItems}
            onClose={onClose}
          />
        </div>,
      );

      // The click-outside listener is attached via requestAnimationFrame,
      // so we need to wait a tick
      await act(async () => {
        await new Promise((r) => requestAnimationFrame(r));
      });

      fireEvent.pointerDown(screen.getByTestId("outside"), {
        bubbles: true,
      });
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard navigation
  // -----------------------------------------------------------------------

  describe("keyboard navigation", () => {
    it("calls onClose on Escape key", () => {
      const onClose = vi.fn();
      const items: MenuItem[] = [{ label: "Item", action: vi.fn() }];
      render(<ContextMenu x={100} y={100} items={items} onClose={onClose} />);

      const menu = screen.getByTestId("context-menu");
      fireEvent.keyDown(menu, { key: "Escape" });
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("navigates down with ArrowDown key", () => {
      const items: MenuItem[] = [
        { label: "First", action: vi.fn() },
        { label: "Second", action: vi.fn() },
      ];
      renderMenu(items);

      const menu = screen.getByTestId("context-menu");
      fireEvent.keyDown(menu, { key: "ArrowDown" });

      // First item should be focused
      const firstItem = screen.getByTestId("context-menu-item-first");
      expect(firstItem).toHaveAttribute("tabindex", "0");
    });

    it("navigates up with ArrowUp key", () => {
      const items: MenuItem[] = [
        { label: "First", action: vi.fn() },
        { label: "Second", action: vi.fn() },
      ];
      renderMenu(items);

      const menu = screen.getByTestId("context-menu");

      // Move down twice
      fireEvent.keyDown(menu, { key: "ArrowDown" });
      fireEvent.keyDown(menu, { key: "ArrowDown" });

      // Move up once
      fireEvent.keyDown(menu, { key: "ArrowUp" });

      // Should be back on first
      const firstItem = screen.getByTestId("context-menu-item-first");
      expect(firstItem).toHaveAttribute("tabindex", "0");
    });

    it("calls action on Enter key when item is focused", () => {
      const action = vi.fn();
      const items: MenuItem[] = [{ label: "Go", action }];
      const { onClose } = renderMenu(items);

      const menu = screen.getByTestId("context-menu");

      // Focus the item
      fireEvent.keyDown(menu, { key: "ArrowDown" });
      // Press Enter
      fireEvent.keyDown(menu, { key: "Enter" });

      expect(action).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("does not call action on Enter when item is disabled", () => {
      const action = vi.fn();
      const items: MenuItem[] = [{ label: "Nope", action, disabled: true }];
      const { onClose } = renderMenu(items);

      const menu = screen.getByTestId("context-menu");
      fireEvent.keyDown(menu, { key: "ArrowDown" });
      fireEvent.keyDown(menu, { key: "Enter" });

      expect(action).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
    });

    it("skips separators during navigation", () => {
      const items: MenuItem[] = [
        { label: "First", action: vi.fn() },
        { label: "", separator: true },
        { label: "Third", action: vi.fn() },
      ];
      renderMenu(items);

      const menu = screen.getByTestId("context-menu");

      // Move down twice — should skip separator and land on Third
      fireEvent.keyDown(menu, { key: "ArrowDown" }); // First
      fireEvent.keyDown(menu, { key: "ArrowDown" }); // Third (skips separator)

      const thirdItem = screen.getByTestId("context-menu-item-third");
      expect(thirdItem).toHaveAttribute("tabindex", "0");
    });
  });

  // -----------------------------------------------------------------------
  // Separator filtering
  // -----------------------------------------------------------------------

  describe("separator filtering", () => {
    it("removes leading separators", () => {
      const items: MenuItem[] = [
        { label: "", separator: true },
        { label: "First", action: vi.fn() },
      ];
      renderMenu(items);
      // Only "First" should render (separator is trailing after filter removes leading, but it's not trailing)
      expect(screen.getByTestId("context-menu-item-first")).toBeInTheDocument();
    });

    it("removes trailing separators", () => {
      const items: MenuItem[] = [
        { label: "First", action: vi.fn() },
        { label: "", separator: true },
      ];
      const { container } = renderMenu(items);
      const separators = container.querySelectorAll("[role='separator']");
      expect(separators).toHaveLength(0);
    });

    it("removes consecutive separators", () => {
      const items: MenuItem[] = [
        { label: "A", action: vi.fn() },
        { label: "", separator: true },
        { label: "", separator: true },
        { label: "B", action: vi.fn() },
      ];
      const { container } = renderMenu(items);
      const separators = container.querySelectorAll("[role='separator']");
      expect(separators).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // Viewport clamping
  // -----------------------------------------------------------------------

  describe("viewport clamping", () => {
    it("positions menu at provided coordinates by default", () => {
      renderMenu(undefined, { x: 200, y: 300 });
      const menu = screen.getByTestId("context-menu");
      expect(menu.style.left).toBe("200px");
      expect(menu.style.top).toBe("300px");
    });

    it("clamps menu to the right edge of viewport", () => {
      // Position menu very close to right edge
      renderMenu(undefined, { x: window.innerWidth - 10, y: 100 });
      const menu = screen.getByTestId("context-menu");
      const left = parseInt(menu.style.left, 10);
      expect(left).toBeLessThan(window.innerWidth - 10);
    });

    it("clamps menu to the bottom edge of viewport", () => {
      // Position menu very close to bottom edge
      renderMenu(undefined, { x: 100, y: window.innerHeight - 10 });
      const menu = screen.getByTestId("context-menu");
      const top = parseInt(menu.style.top, 10);
      expect(top).toBeLessThan(window.innerHeight - 10);
    });
  });
});
