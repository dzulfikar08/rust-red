/**
 * WorkspaceTabs Component
 *
 * Tab bar for managing multiple flow tabs, matching Node-RED's workspace tabs.
 * Positioned at the bottom of the canvas area.
 *
 * Features:
 *  - Click to switch active flow
 *  - Double-click to rename (inline edit)
 *  - Right-click for context menu
 *  - "+" button to add new flow
 *  - Close button on each tab
 *  - Scrollable when many tabs
 *  - Drag to reorder
 */
import { useState, useRef, useCallback, type MouseEvent } from "react";
import { Plus, X } from "lucide-react";
import { useWorkspaceStore } from "../../store/workspace-store";

// ---------------------------------------------------------------------------
// Inline edit sub-component
// ---------------------------------------------------------------------------

function InlineEdit({
  value,
  onSave,
  onCancel,
}: {
  value: string;
  onSave: (val: string) => void;
  onCancel: () => void;
}) {
  const [editValue, setEditValue] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      const trimmed = editValue.trim();
      if (trimmed) {
        onSave(trimmed);
      } else {
        onCancel();
      }
    } else if (e.key === "Escape") {
      onCancel();
    }
  };

  const handleBlur = () => {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== value) {
      onSave(trimmed);
    } else {
      onCancel();
    }
  };

  return (
    <input
      ref={inputRef}
      type="text"
      value={editValue}
      onChange={(e) => setEditValue(e.target.value)}
      onKeyDown={handleKeyDown}
      onBlur={handleBlur}
      autoFocus
      className="w-full bg-transparent border-none outline-none text-xs text-gray-100 px-0 py-0"
      onClick={(e) => e.stopPropagation()}
    />
  );
}

// ---------------------------------------------------------------------------
// Context menu sub-component
// ---------------------------------------------------------------------------

interface ContextMenuProps {
  x: number;
  y: number;
  flowId: string;
  flowLabel: string;
  flowDisabled: boolean;
  onRename: () => void;
  onDelete: () => void;
  onToggleDisable: () => void;
  onClose: () => void;
}

function TabContextMenu({
  x,
  y,
  flowLabel,
  flowDisabled,
  onRename,
  onDelete,
  onToggleDisable,
  onClose,
}: ContextMenuProps) {
  return (
    <>
      {/* Backdrop to close menu */}
      <div
        className="fixed inset-0 z-40"
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onClose();
        }}
      />
      <div
        className="fixed z-50 min-w-[160px] rounded-md border border-gray-600 bg-gray-800 py-1 shadow-lg animate-in"
        style={{ left: x, top: y }}
        onClick={(e) => e.stopPropagation()}
      >
        <button
          type="button"
          className="flex w-full items-center px-3 py-1.5 text-left text-xs text-gray-200 hover:bg-gray-700"
          onClick={() => {
            onRename();
            onClose();
          }}
        >
          Rename
        </button>
        <button
          type="button"
          className="flex w-full items-center px-3 py-1.5 text-left text-xs text-gray-200 hover:bg-gray-700"
          onClick={() => {
            onToggleDisable();
            onClose();
          }}
        >
          {flowDisabled ? "Enable" : "Disable"}
        </button>
        <hr className="my-1 border-gray-600" />
        <button
          type="button"
          className="flex w-full items-center px-3 py-1.5 text-left text-xs text-red-400 hover:bg-gray-700"
          onClick={() => {
            onDelete();
            onClose();
          }}
        >
          Delete "{flowLabel}"
        </button>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function WorkspaceTabs() {
  const flows = useWorkspaceStore((s) => s.flows);
  const activeFlowId = useWorkspaceStore((s) => s.activeFlowId);
  const addFlow = useWorkspaceStore((s) => s.addFlow);
  const removeFlow = useWorkspaceStore((s) => s.removeFlow);
  const renameFlow = useWorkspaceStore((s) => s.renameFlow);
  const setActiveFlow = useWorkspaceStore((s) => s.setActiveFlow);
  const reorderFlows = useWorkspaceStore((s) => s.reorderFlows);
  const setFlowDisabled = useWorkspaceStore((s) => s.setFlowDisabled);

  // Track which tab is being renamed inline
  const [editingId, setEditingId] = useState<string | null>(null);

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    flowId: string;
    flowLabel: string;
    flowDisabled: boolean;
  } | null>(null);

  // Drag reorder state
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // ----- Handlers -----

  const handleAddFlow = useCallback(() => {
    addFlow();
  }, [addFlow]);

  const handleTabClick = useCallback(
    (id: string) => {
      setActiveFlow(id);
    },
    [setActiveFlow],
  );

  const handleDoubleClick = useCallback((id: string) => {
    setEditingId(id);
  }, []);

  const handleRename = useCallback(
    (id: string, label: string) => {
      renameFlow(id, label);
      setEditingId(null);
    },
    [renameFlow],
  );

  const handleClose = useCallback(
    (id: string, e: MouseEvent) => {
      e.stopPropagation();
      removeFlow(id);
    },
    [removeFlow],
  );

  const handleContextMenu = useCallback(
    (e: MouseEvent, flowId: string) => {
      e.preventDefault();
      e.stopPropagation();
      const flow = flows.find((f) => f.id === flowId);
      if (!flow) return;
      setContextMenu({
        x: e.clientX,
        y: e.clientY,
        flowId: flow.id,
        flowLabel: flow.label,
        flowDisabled: flow.disabled,
      });
    },
    [flows],
  );

  const handleRenameFromMenu = useCallback(() => {
    if (contextMenu) {
      setEditingId(contextMenu.flowId);
    }
  }, [contextMenu]);

  const handleDeleteFromMenu = useCallback(() => {
    if (contextMenu) {
      removeFlow(contextMenu.flowId);
    }
  }, [contextMenu, removeFlow]);

  const handleToggleDisable = useCallback(() => {
    if (contextMenu) {
      setFlowDisabled(contextMenu.flowId, !contextMenu.flowDisabled);
    }
  }, [contextMenu, setFlowDisabled]);

  // ----- Drag reorder -----

  const handleDragStart = useCallback(
    (e: MouseEvent, index: number) => {
      // Use a data transfer to identify this is a tab drag
      (e as unknown as React.DragEvent).dataTransfer?.setData(
        "text/plain",
        String(index),
      );
      setDragIndex(index);
    },
    [],
  );

  const handleDragOver = useCallback(
    (e: MouseEvent, index: number) => {
      e.preventDefault();
      setDragOverIndex(index);
    },
    [],
  );

  const handleDrop = useCallback(
    (e: MouseEvent, toIndex: number) => {
      e.preventDefault();
      if (dragIndex !== null && dragIndex !== toIndex) {
        reorderFlows(dragIndex, toIndex);
      }
      setDragIndex(null);
      setDragOverIndex(null);
    },
    [dragIndex, reorderFlows],
  );

  const handleDragEnd = useCallback(() => {
    setDragIndex(null);
    setDragOverIndex(null);
  }, []);

  // ----- Render -----

  return (
    <div className="relative flex items-end h-8 bg-gray-900 border-t border-gray-700 select-none">
      {/* Scrollable tab container */}
      <div
        ref={scrollContainerRef}
        className="flex items-end overflow-x-auto overflow-y-hidden flex-1 scrollbar-thin"
        style={{ scrollbarWidth: "thin" }}
      >
        {flows.map((flow, index) => {
          const isActive = flow.id === activeFlowId;
          const isEditing = flow.id === editingId;
          const isDragging = dragIndex === index;
          const isDragOver = dragOverIndex === index && dragIndex !== index;

          return (
            <div
              key={flow.id}
              role="tab"
              aria-selected={isActive}
              tabIndex={0}
              draggable={!isEditing}
              onClick={() => handleTabClick(flow.id)}
              onDoubleClick={() => handleDoubleClick(flow.id)}
              onContextMenu={(e) => handleContextMenu(e, flow.id)}
              onDragStart={(e) => handleDragStart(e as unknown as MouseEvent, index)}
              onDragOver={(e) => handleDragOver(e as unknown as MouseEvent, index)}
              onDrop={(e) => handleDrop(e as unknown as MouseEvent, index)}
              onDragEnd={handleDragEnd}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleTabClick(flow.id);
              }}
              className={`
                group relative flex items-center gap-1 px-3 h-7 text-xs cursor-pointer
                border-r border-gray-700 whitespace-nowrap shrink-0
                transition-colors duration-100
                ${isActive
                  ? "bg-gray-700 text-gray-100"
                  : "bg-gray-800 text-gray-400 hover:bg-gray-750 hover:text-gray-200"
                }
                ${flow.disabled ? "opacity-50 italic" : ""}
                ${isDragging ? "opacity-40" : ""}
                ${isDragOver ? "border-l-2 border-l-blue-400" : ""}
              `}
              style={{ minWidth: 0 }}
              title={flow.info ?? flow.label}
            >
              {/* Tab label or inline edit */}
              {isEditing ? (
                <InlineEdit
                  value={flow.label}
                  onSave={(label) => handleRename(flow.id, label)}
                  onCancel={() => setEditingId(null)}
                />
              ) : (
                <span className="truncate max-w-[120px]">{flow.label}</span>
              )}

              {/* Close button */}
              {!isEditing && (
                <button
                  type="button"
                  aria-label={`Close ${flow.label}`}
                  className="flex items-center justify-center w-3.5 h-3.5 rounded-sm
                    opacity-0 group-hover:opacity-100 hover:bg-gray-600
                    transition-opacity duration-100"
                  onClick={(e) => handleClose(flow.id, e)}
                >
                  <X size={10} />
                </button>
              )}
            </div>
          );
        })}
      </div>

      {/* Add flow button */}
      <button
        type="button"
        aria-label="Add new flow"
        className="flex items-center justify-center w-8 h-7 shrink-0
          text-gray-400 hover:text-gray-100 hover:bg-gray-700
          transition-colors duration-100"
        onClick={handleAddFlow}
      >
        <Plus size={14} />
      </button>

      {/* Context menu */}
      {contextMenu && (
        <TabContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          flowId={contextMenu.flowId}
          flowLabel={contextMenu.flowLabel}
          flowDisabled={contextMenu.flowDisabled}
          onRename={handleRenameFromMenu}
          onDelete={handleDeleteFromMenu}
          onToggleDisable={handleToggleDisable}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
