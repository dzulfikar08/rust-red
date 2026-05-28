/**
 * Category Component
 *
 * Individual collapsible category in the node palette.
 * Renders a header with colored dot, category name, and
 * a grid of draggable node items.
 */

import { useCallback } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { NodeDefinition } from "../../red/nodes/types";
import { eventBus } from "../../red/core/events";

interface CategoryProps {
  /** Category name (e.g. "common", "function"). */
  name: string;
  /** Category color hex. */
  color: string;
  /** Node definitions in this category. */
  nodes: NodeDefinition[];
  /** Whether the category is expanded. */
  expanded: boolean;
  /** Callback to toggle expand/collapse. */
  onToggle: () => void;
}

export function Category({
  name,
  color,
  nodes,
  expanded,
  onToggle,
}: CategoryProps) {
  const handleDragStart = useCallback(
    (event: React.DragEvent, nodeType: string) => {
      event.dataTransfer.setData("application/reactflow", nodeType);
      event.dataTransfer.effectAllowed = "move";
    },
    [],
  );

  const handleNodeClick = useCallback((nodeType: string) => {
    eventBus.emit("node:add-requested", { type: nodeType });
  }, []);

  return (
    <div>
      <button
        type="button"
        onClick={onToggle}
        className="flex items-center gap-1.5 w-full px-2 py-1.5 text-xs font-semibold text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors select-none"
      >
        {expanded ? (
          <ChevronDown size={14} className="flex-shrink-0 text-gray-500" />
        ) : (
          <ChevronRight size={14} className="flex-shrink-0 text-gray-500" />
        )}
        <span
          className="w-2.5 h-2.5 rounded-full flex-shrink-0"
          style={{ backgroundColor: color }}
        />
        <span className="truncate">{name}</span>
      </button>
      {expanded && (
        <div className="px-2 pb-2 grid grid-cols-2 gap-1">
          {nodes.map((node) => {
            const label = node.paletteLabel ?? node.name;
            return (
              <div
                key={node.type}
                draggable
                onDragStart={(e) => handleDragStart(e, node.type)}
                onClick={() => handleNodeClick(node.type)}
                className="flex items-center gap-1.5 px-1.5 py-1 rounded cursor-grab text-[11px] hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors active:cursor-grabbing select-none"
                title={node.type}
              >
                <span
                  className="w-3 h-3 rounded-sm flex-shrink-0 border border-black/10"
                  style={{ backgroundColor: node.color }}
                />
                <span className="truncate">{label}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
