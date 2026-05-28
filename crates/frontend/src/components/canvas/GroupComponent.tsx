/**
 * GroupComponent — React Flow custom node that renders a group rectangle.
 *
 * Visually matches Node-RED group appearance:
 *  - Rounded rectangle with light fill and dashed border
 *  - Label at top-left corner
 *  - Resizable by dragging corners/edges
 *  - Moves all contained nodes when the group is moved
 *  - Background color customizable via style.fill
 */

import { memo, useCallback, useRef, useState } from "react";
import type { Node, NodeProps } from "@xyflow/react";
import { useGroupStore } from "../../store/group-store";

// ---------------------------------------------------------------------------
// Data shape carried by a group node on the React Flow canvas
// ---------------------------------------------------------------------------

export interface GroupComponentData {
  label: string;
  /** IDs of member nodes */
  nodes: string[];
  /** Background fill color */
  fill?: string;
  /** Border stroke color */
  stroke?: string;
  /** Label color */
  labelColor?: string;
}

export type GroupNode = Node<GroupComponentData, "group">;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HANDLE_SIZE = 8;
const MIN_WIDTH = 100;
const MIN_HEIGHT = 60;
const LABEL_HEIGHT = 24;

// ---------------------------------------------------------------------------
// Resize handle positions
// ---------------------------------------------------------------------------

type HandleCorner = "nw" | "ne" | "sw" | "se";

interface ResizeState {
  corner: HandleCorner;
  startX: number;
  startY: number;
  startW: number;
  startH: number;
  startXPos: number;
  startYPos: number;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

function GroupComponentRaw({ id, data, selected }: NodeProps<GroupNode>) {
  const group = useGroupStore((s) => s.groups.find((g) => g.id === id));
  const moveGroup = useGroupStore((s) => s.moveGroup);
  const resizeGroup = useGroupStore((s) => s.resizeGroup);
  const updateGroup = useGroupStore((s) => s.updateGroup);

  const [isEditing, setIsEditing] = useState(false);
  const [editLabel, setEditLabel] = useState(data.label);
  const [resizeState, setResizeState] = useState<ResizeState | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Fall back to data if store not hydrated yet
  const w = group?.w ?? 200;
  const h = group?.h ?? 150;
  const label = data.label ?? "Group";
  const fillColor = data.fill ?? "#f0f0f0";
  const strokeColor = data.stroke ?? "#999";
  const labelColor = data.labelColor ?? "#333";

  // -----------------------------------------------------------------------
  // Drag: when the group node is dragged via React Flow, move members
  // -----------------------------------------------------------------------

  const handleDragStart = useCallback(
    (e: React.MouseEvent) => {
      // Only handle if click is on the group background (not a resize handle)
      if ((e.target as HTMLElement).dataset.resizeHandle) return;

      e.stopPropagation();
      const startX = e.clientX;
      const startY = e.clientY;

      const onMove = (ev: MouseEvent) => {
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;
        // We accumulate from start, so reset each move
        // Actually just move by delta from last position
        moveGroup(id, dx, dy);
      };

      const onUp = () => {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [id, moveGroup],
  );

  // -----------------------------------------------------------------------
  // Resize
  // -----------------------------------------------------------------------

  const handleResizeStart = useCallback(
    (corner: HandleCorner) => (e: React.MouseEvent) => {
      e.stopPropagation();
      e.preventDefault();

      const startX = e.clientX;
      const startY = e.clientY;

      setResizeState({
        corner,
        startX,
        startY,
        startW: w,
        startH: h,
        startXPos: group?.x ?? 0,
        startYPos: group?.y ?? 0,
      });

      const onMove = (ev: MouseEvent) => {
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;

        let newW = w;
        let newH = h;
        let newX = group?.x ?? 0;
        let newY = group?.y ?? 0;

        switch (corner) {
          case "se":
            newW = Math.max(MIN_WIDTH, w + dx);
            newH = Math.max(MIN_HEIGHT, h + dy);
            break;
          case "sw":
            newW = Math.max(MIN_WIDTH, w - dx);
            newH = Math.max(MIN_HEIGHT, h + dy);
            newX = (group?.x ?? 0) + (w - newW);
            break;
          case "ne":
            newW = Math.max(MIN_WIDTH, w + dx);
            newH = Math.max(MIN_HEIGHT, h - dy);
            newY = (group?.y ?? 0) + (h - newH);
            break;
          case "nw":
            newW = Math.max(MIN_WIDTH, w - dx);
            newH = Math.max(MIN_HEIGHT, h - dy);
            newX = (group?.x ?? 0) + (w - newW);
            newY = (group?.y ?? 0) + (h - newH);
            break;
        }

        resizeGroup(id, newW, newH);
        if (newX !== (group?.x ?? 0) || newY !== (group?.y ?? 0)) {
          updateGroup(id, { x: newX, y: newY });
        }
      };

      const onUp = () => {
        setResizeState(null);
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [id, w, h, group, resizeGroup, updateGroup],
  );

  // -----------------------------------------------------------------------
  // Label editing
  // -----------------------------------------------------------------------

  const handleLabelDoubleClick = useCallback(() => {
    setEditLabel(label);
    setIsEditing(true);
  }, [label]);

  const handleLabelBlur = useCallback(() => {
    setIsEditing(false);
    updateGroup(id, { label: editLabel });
  }, [id, editLabel, updateGroup]);

  const handleLabelKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        setIsEditing(false);
        updateGroup(id, { label: editLabel });
      } else if (e.key === "Escape") {
        setIsEditing(false);
        setEditLabel(label);
      }
    },
    [id, editLabel, label, updateGroup],
  );

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  return (
    <div
      ref={containerRef}
      className="absolute"
      style={{
        width: w,
        height: h,
      }}
      data-testid="group-container"
    >
      {/* Group body */}
      <div
        className="absolute inset-0 rounded-[8px] select-none"
        style={{
          backgroundColor: fillColor,
          border: selected
            ? "2px dashed #4a90d9"
            : `1.5px dashed ${strokeColor}`,
          cursor: "move",
        }}
        onMouseDown={handleDragStart}
        data-testid="group-body"
      >
        {/* Label area */}
        <div
          className="absolute top-0 left-0 right-0 flex items-center px-2"
          style={{ height: LABEL_HEIGHT }}
          onDoubleClick={handleLabelDoubleClick}
          data-testid="group-label-area"
        >
          {isEditing ? (
            <input
              type="text"
              value={editLabel}
              onChange={(e) => setEditLabel(e.target.value)}
              onBlur={handleLabelBlur}
              onKeyDown={handleLabelKeyDown}
              className="w-full text-[12px] font-semibold bg-white border border-blue-400 rounded px-1 outline-none"
              style={{ color: labelColor }}
              autoFocus
              data-testid="group-label-input"
            />
          ) : (
            <span
              className="text-[12px] font-semibold truncate"
              style={{ color: labelColor }}
              data-testid="group-label-text"
            >
              {label}
            </span>
          )}
        </div>
      </div>

      {/* Resize handles (visible when selected) */}
      {selected && (
        <>
          {/* SE handle */}
          <div
            data-resize-handle="true"
            className="absolute cursor-se-resize"
            style={{
              width: HANDLE_SIZE,
              height: HANDLE_SIZE,
              right: -HANDLE_SIZE / 2,
              bottom: -HANDLE_SIZE / 2,
              backgroundColor: "#4a90d9",
              borderRadius: 2,
            }}
            onMouseDown={handleResizeStart("se")}
            data-testid="resize-handle-se"
          />
          {/* SW handle */}
          <div
            data-resize-handle="true"
            className="absolute cursor-sw-resize"
            style={{
              width: HANDLE_SIZE,
              height: HANDLE_SIZE,
              left: -HANDLE_SIZE / 2,
              bottom: -HANDLE_SIZE / 2,
              backgroundColor: "#4a90d9",
              borderRadius: 2,
            }}
            onMouseDown={handleResizeStart("sw")}
            data-testid="resize-handle-sw"
          />
          {/* NE handle */}
          <div
            data-resize-handle="true"
            className="absolute cursor-ne-resize"
            style={{
              width: HANDLE_SIZE,
              height: HANDLE_SIZE,
              right: -HANDLE_SIZE / 2,
              top: -HANDLE_SIZE / 2,
              backgroundColor: "#4a90d9",
              borderRadius: 2,
            }}
            onMouseDown={handleResizeStart("ne")}
            data-testid="resize-handle-ne"
          />
          {/* NW handle */}
          <div
            data-resize-handle="true"
            className="absolute cursor-nw-resize"
            style={{
              width: HANDLE_SIZE,
              height: HANDLE_SIZE,
              left: -HANDLE_SIZE / 2,
              top: -HANDLE_SIZE / 2,
              backgroundColor: "#4a90d9",
              borderRadius: 2,
            }}
            onMouseDown={handleResizeStart("nw")}
            data-testid="resize-handle-nw"
          />
        </>
      )}
    </div>
  );
}

export const GroupComponent = memo(GroupComponentRaw);
