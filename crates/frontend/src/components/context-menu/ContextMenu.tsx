/**
 * ContextMenu — floating right-click context menu for the Node-RED canvas.
 *
 * Features:
 *  - Positioned at mouse coordinates, clamped to viewport
 *  - Closes on click outside, Escape key, or item click
 *  - Keyboard navigation: arrow keys + Enter
 *  - Disabled items shown greyed out
 *  - Separators as horizontal lines
 *  - Dark background matching Node-RED theme
 */

import { useEffect, useRef, useState, useCallback } from "react";
import type { MenuItem } from "../../store/context-menu-store";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface ContextMenuProps {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [focusedIndex, setFocusedIndex] = useState(-1);

  // Filter out trailing/leading/duplicate separators for display
  const visibleItems = filterSeparators(items);

  // -----------------------------------------------------------------------
  // Viewport clamping
  // -----------------------------------------------------------------------

  const getMenuBounds = useCallback(() => {
    const menuW = 220;
    const itemH = 32;
    const separatorH = 9;
    const paddingY = 8; // py-1 = 4px top + 4px bottom

    let totalH = paddingY;
    for (const item of visibleItems) {
      totalH += item.separator ? separatorH : itemH;
    }

    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const left = x + menuW > vw ? Math.max(8, vw - menuW - 8) : x;
    const top = y + totalH > vh ? Math.max(8, vh - totalH - 8) : y;

    return { left, top };
  }, [x, y, visibleItems]);

  const { left, top } = getMenuBounds();

  // -----------------------------------------------------------------------
  // Click outside
  // -----------------------------------------------------------------------

  useEffect(() => {
    function handlePointerDown(e: PointerEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    }

    // Use requestAnimationFrame so we don't catch the right-click that opened us
    const raf = requestAnimationFrame(() => {
      document.addEventListener("pointerdown", handlePointerDown, true);
    });

    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, [onClose]);

  // -----------------------------------------------------------------------
  // Keyboard navigation
  // -----------------------------------------------------------------------

  // Build a list of focusable (non-separator, non-disabled) indices
  const focusableIndices = visibleItems
    .map((item, idx) => ({ item, idx }))
    .filter(({ item }) => !item.separator && !item.disabled);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setFocusedIndex((prev) => {
          const currentPos = focusableIndices.findIndex((f) => f.idx === prev);
          const nextPos =
            currentPos < focusableIndices.length - 1
              ? currentPos + 1
              : currentPos;
          return focusableIndices[nextPos]?.idx ?? -1;
        });
        return;
      }

      if (e.key === "ArrowUp") {
        e.preventDefault();
        setFocusedIndex((prev) => {
          const currentPos = focusableIndices.findIndex((f) => f.idx === prev);
          const nextPos = currentPos > 0 ? currentPos - 1 : 0;
          return focusableIndices[nextPos]?.idx ?? -1;
        });
        return;
      }

      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        const focusedItem = visibleItems[focusedIndex];
        if (focusedItem && !focusedItem.separator && !focusedItem.disabled && focusedItem.action) {
          focusedItem.action();
          onClose();
        }
        return;
      }
    },
    [onClose, focusedIndex, focusableIndices, visibleItems],
  );

  // Focus the menu on mount for keyboard nav
  useEffect(() => {
    menuRef.current?.focus();
  }, []);

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (visibleItems.length === 0) return null;

  return (
    <div
      ref={menuRef}
      role="menu"
      tabIndex={-1}
      data-testid="context-menu"
      className="fixed z-[100] bg-[#2a2a2a] rounded shadow-lg border border-[#555] py-1 min-w-[180px] max-w-[280px] outline-none"
      style={{ left, top }}
      onKeyDown={handleKeyDown}
    >
      {visibleItems.map((item, idx) => {
        if (item.separator) {
          return (
            <div
              key={`sep-${idx}`}
              role="separator"
              className="my-1 mx-2 border-t border-[#555]"
            />
          );
        }

        const isFocused = idx === focusedIndex;
        const isDisabled = item.disabled ?? false;

        return (
          <div
            key={`${item.label}-${idx}`}
            role="menuitem"
            tabIndex={isFocused ? 0 : -1}
            aria-disabled={isDisabled}
            data-testid={`context-menu-item-${item.label.toLowerCase().replace(/\s+/g, "-")}`}
            className={[
              "flex items-center justify-between gap-4 px-3 py-1.5 text-[13px] cursor-default select-none",
              isDisabled
                ? "text-[#777] cursor-not-allowed"
                : isFocused
                  ? "bg-[#4a4a4a] text-white"
                  : "text-[#ddd] hover:bg-[#4a4a4a]",
            ].join(" ")}
            onClick={() => {
              if (isDisabled) return;
              item.action?.();
              onClose();
            }}
            onMouseEnter={() => setFocusedIndex(idx)}
          >
            {/* Left: icon + label */}
            <span className="flex items-center gap-2">
              {item.icon && (
                <span className="w-4 h-4 flex items-center justify-center shrink-0">
                  {item.icon}
                </span>
              )}
              <span>{item.label}</span>
            </span>

            {/* Right: shortcut */}
            {item.shortcut && (
              <span className="text-[11px] text-[#999] ml-auto pl-4 shrink-0">
                {item.shortcut}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Remove leading, trailing, and consecutive separators.
 */
function filterSeparators(items: MenuItem[]): MenuItem[] {
  const result: MenuItem[] = [];
  let lastWasSeparator = true; // treat start as if prev was separator

  for (const item of items) {
    if (item.separator) {
      if (!lastWasSeparator) {
        result.push(item);
        lastWasSeparator = true;
      }
    } else {
      result.push(item);
      lastWasSeparator = false;
    }
  }

  // Remove trailing separator
  if (result.length > 0 && result[result.length - 1].separator) {
    result.pop();
  }

  return result;
}
