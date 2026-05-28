/**
 * Sidebar -- Right-side panel container matching Node-RED's sidebar.
 *
 * This layout component handles:
 *  - Width and resizing (drag handle on left edge)
 *  - Open/close visibility
 *  - Delegates tab switching and content rendering to SidebarContainer
 *
 * When closed, only the thin tab strip (~36px) is visible so the user can
 * click a tab to re-open the sidebar.
 */

import { useState, useCallback, useRef, useEffect } from "react";
import { useSidebarStore } from "../../store/sidebar-store";
import { SidebarContainer } from "../sidebar/SidebarContainer";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface SidebarProps {
  open: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MIN_WIDTH = 200;
const MAX_WIDTH = 600;
const TAB_BAR_WIDTH = 36; // width of the icon tab strip

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Sidebar({ open }: SidebarProps) {
  const width = useSidebarStore((s) => s.width);
  const setWidth = useSidebarStore((s) => s.setWidth);
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
    [width, setWidth],
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

  // Always render the sidebar (tab bar strip is always visible).
  // When open, the content panel is shown alongside the tab bar.
  return (
    <aside
      className="flex h-full relative"
      data-testid="sidebar"
      style={{
        width: open ? width + TAB_BAR_WIDTH : TAB_BAR_WIDTH,
        minWidth: open ? MIN_WIDTH + TAB_BAR_WIDTH : TAB_BAR_WIDTH,
        maxWidth: MAX_WIDTH + TAB_BAR_WIDTH,
        transition: isResizing ? "none" : "width 150ms ease",
      }}
    >
      {/* Resize drag handle (left edge) -- only visible when open */}
      {open && (
        <div
          ref={resizeRef}
          className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-400/50 active:bg-blue-400 z-10"
          onMouseDown={handleMouseDown}
          data-testid="sidebar-resize-handle"
        />
      )}

      <SidebarContainer />
    </aside>
  );
}
