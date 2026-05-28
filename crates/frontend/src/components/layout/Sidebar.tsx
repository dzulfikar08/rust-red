/**
 * Sidebar -- Right-side panel container matching Node-RED's sidebar.
 *
 * Features:
 *  - Hidden (collapsed) by default
 *  - Fixed width (~320px) when open, sits on the right side of the layout
 *  - Resizable via drag handle on left edge
 *  - Placeholder content for sidebar tabs (info, debug, config, context, help)
 *    -- actual tab implementations are Phase 2
 */

import { useState, useCallback, useRef, useEffect } from "react";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface SidebarProps {
  open: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_WIDTH = 320;
const MIN_WIDTH = 200;
const MAX_WIDTH = 600;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Sidebar({ open }: SidebarProps) {
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const [isResizing, setIsResizing] = useState(false);
  const resizeRef = useRef<HTMLDivElement>(null);

  // ----- Resize drag handler -----

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsResizing(true);

      const startX = e.clientX;
      const startWidth = width;

      function onMouseMove(ev: MouseEvent) {
        // Dragging left edge: moving left increases width
        const delta = startX - ev.clientX;
        const newWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidth + delta));
        setWidth(newWidth);
      }

      function onMouseUp() {
        setIsResizing(false);
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
      }

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
    },
    [width],
  );

  // ----- Prevent text selection while resizing -----

  useEffect(() => {
    if (isResizing) {
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    } else {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }
    return () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing]);

  if (!open) return null;

  return (
    <aside
      className="flex flex-col h-full border-l border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden relative"
      style={{ width, minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH }}
      data-testid="sidebar"
    >
      {/* Resize drag handle (left edge) */}
      <div
        ref={resizeRef}
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-400/50 active:bg-blue-400 z-10"
        onMouseDown={handleMouseDown}
        data-testid="sidebar-resize-handle"
      />

      {/* Sidebar header (placeholder for tabs) */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
        {["info", "debug", "config", "context"].map((tab) => (
          <button
            key={tab}
            type="button"
            className={`px-2 py-1 text-[11px] font-medium rounded transition-colors ${
              tab === "info"
                ? "bg-gray-200 dark:bg-gray-600 text-gray-800 dark:text-gray-100"
                : "text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
            }`}
            data-testid={`sidebar-tab-${tab}`}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Sidebar content placeholder */}
      <div className="flex-1 flex items-center justify-center p-4">
        <p className="text-sm text-gray-400">Sidebar</p>
      </div>
    </aside>
  );
}
