/**
 * OutlinerTab -- Hierarchical tree view of all nodes in the current flow.
 *
 * Groups nodes by their palette category (common, function, network, etc.)
 * with collapsible sections, search filtering, and click-to-select.
 *
 * Replaces Node-RED's RED.sidebar.info.outliner (~711 lines).
 */

import { useState, useMemo, useCallback } from "react";
import { ChevronDown, ChevronRight, Search } from "lucide-react";
import { useFlowStore } from "../../../store/flow-store";
import { useWorkspaceStore } from "../../../store/workspace-store";
import { useEditorStore } from "../../../store/editor-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { eventBus } from "../../../red/core/events";

// ---------------------------------------------------------------------------
// Category ordering and display names (matching Node-RED palette order)
// ---------------------------------------------------------------------------

const CATEGORY_ORDER = [
  "common",
  "function",
  "network",
  "storage",
  "parser",
  "sequence",
] as const;

const CATEGORY_LABELS: Record<string, string> = {
  common: "common",
  function: "function",
  network: "network",
  storage: "storage",
  parser: "parser",
  sequence: "sequence",
};

/** Category colors fallbacks when node registry has no color */
const CATEGORY_COLORS: Record<string, string> = {
  common: "#a6bbcf",
  function: "#e2d96e",
  network: "#e2d96e",
  storage: "#e2d96e",
  parser: "#e2d96e",
  sequence: "#e2d96e",
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NodeItem {
  id: string;
  label: string;
  type: string;
  color: string;
  category: string;
}

interface CategoryGroup {
  category: string;
  label: string;
  nodes: NodeItem[];
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function OutlinerTab() {
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(CATEGORY_ORDER),
  );

  const flowNodes = useFlowStore((s) => s.nodes);
  const activeFlowId = useWorkspaceStore((s) => s.activeFlowId);
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);

  // -----------------------------------------------------------------------
  // Build node list for the active flow
  // -----------------------------------------------------------------------

  const groups = useMemo<CategoryGroup[]>(() => {
    // Filter nodes belonging to the active flow
    const flowFilter = activeFlowId ?? "";
    const activeNodes = flowNodes.filter((n) => {
      // Nodes created from palette have data.flowId or are in the current workspace
      const nodeFlow = (n.data as Record<string, unknown>)?.flowId as string | undefined;
      if (nodeFlow) return nodeFlow === flowFilter;
      // If no flowId on data, include all when no active flow, otherwise skip
      return !activeFlowId;
    });

    // Map nodes to our simpler NodeItem shape
    const items: NodeItem[] = activeNodes.map((n) => {
      const data = n.data as Record<string, unknown>;
      const type = (data?.type as string) ?? "unknown";
      const def = nodeRegistry.getType(type);
      const category = def?.category ?? "common";
      const color = def?.color ?? (CATEGORY_COLORS[category] ?? "#a6bbcf");

      // Resolve label: use data.label, then name from data, then type
      let label: string;
      if (typeof data?.label === "string" && data.label.length > 0) {
        label = data.label;
      } else if (typeof data?.name === "string" && (data.name as string).length > 0) {
        label = data.name as string;
      } else if (def?.paletteLabel && def.paletteLabel.length > 0) {
        label = def.paletteLabel;
      } else {
        label = type;
      }

      return { id: n.id, label, type, color, category };
    });

    // Apply search filter
    const filtered = searchQuery.trim()
      ? items.filter((item) =>
          item.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
          item.type.toLowerCase().includes(searchQuery.toLowerCase()),
        )
      : items;

    // Group by category
    const categoryMap = new Map<string, NodeItem[]>();
    for (const item of filtered) {
      const cat = item.category;
      let list = categoryMap.get(cat);
      if (!list) {
        list = [];
        categoryMap.set(cat, list);
      }
      list.push(item);
    }

    // Sort categories by the predefined order, then alphabetically for unknown ones
    const sortedCategories = Array.from(categoryMap.keys()).sort((a, b) => {
      const idxA = CATEGORY_ORDER.indexOf(
        a as (typeof CATEGORY_ORDER)[number],
      );
      const idxB = CATEGORY_ORDER.indexOf(
        b as (typeof CATEGORY_ORDER)[number],
      );
      if (idxA !== -1 && idxB !== -1) return idxA - idxB;
      if (idxA !== -1) return -1;
      if (idxB !== -1) return 1;
      return a.localeCompare(b);
    });

    return sortedCategories.map((cat) => ({
      category: cat,
      label: CATEGORY_LABELS[cat] ?? cat,
      nodes: categoryMap.get(cat) ?? [],
    }));
  }, [flowNodes, activeFlowId, searchQuery]);

  // -----------------------------------------------------------------------
  // Expand / collapse
  // -----------------------------------------------------------------------

  const toggleCategory = useCallback((category: string) => {
    setExpandedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  }, []);

  // -----------------------------------------------------------------------
  // Node click -> select on canvas
  // -----------------------------------------------------------------------

  const handleNodeClick = useCallback(
    (nodeId: string) => {
      useEditorStore.getState().selectNode(nodeId);
      eventBus.emit("node:select-requested", { id: nodeId });
    },
    [],
  );

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  const isEmpty = groups.length === 0;

  return (
    <div
      className="flex flex-col h-full text-sm"
      data-testid="sidebar-tab-content-outliner"
    >
      {/* Search bar */}
      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-gray-200 dark:border-gray-700">
        <Search className="w-3.5 h-3.5 text-gray-400 shrink-0" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Filter nodes..."
          className="flex-1 bg-transparent text-xs outline-none placeholder:text-gray-400 text-gray-800 dark:text-gray-200"
          data-testid="outliner-search-input"
        />
      </div>

      {/* Tree content */}
      <div className="flex-1 overflow-y-auto">
        {isEmpty ? (
          <div className="flex items-center justify-center p-4 h-full">
            <p className="text-xs text-gray-400" data-testid="outliner-empty-state">
              {searchQuery.trim()
                ? "No matching nodes"
                : "No nodes in this flow"}
            </p>
          </div>
        ) : (
          <ul className="py-1" data-testid="outliner-tree">
            {groups.map((group) => {
              const isExpanded = expandedCategories.has(group.category);
              return (
                <li key={group.category} data-testid={`outliner-category-${group.category}`}>
                  {/* Category header */}
                  <button
                    type="button"
                    className="flex items-center gap-1 w-full px-2 py-1 text-left text-xs font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                    onClick={() => toggleCategory(group.category)}
                    data-testid={`outliner-category-toggle-${group.category}`}
                    aria-expanded={isExpanded}
                  >
                    {isExpanded ? (
                      <ChevronDown className="w-3 h-3 shrink-0" />
                    ) : (
                      <ChevronRight className="w-3 h-3 shrink-0" />
                    )}
                    <span className="flex-1 truncate">
                      {group.label} ({group.nodes.length})
                    </span>
                  </button>

                  {/* Node items */}
                  {isExpanded && (
                    <ul data-testid={`outliner-category-items-${group.category}`}>
                      {group.nodes.map((node) => {
                        const isSelected = node.id === selectedNodeId;
                        return (
                          <li key={node.id}>
                            <button
                              type="button"
                              className={`flex items-center gap-2 w-full px-2 py-1 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-800 ${
                                isSelected
                                  ? "bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
                                  : "text-gray-700 dark:text-gray-300"
                              }`}
                              onClick={() => handleNodeClick(node.id)}
                              data-testid={`outliner-node-${node.id}`}
                            >
                              <span
                                className="w-2.5 h-2.5 rounded-full shrink-0"
                                style={{ backgroundColor: node.color }}
                                data-testid={`outliner-node-dot-${node.id}`}
                              />
                              <span className="truncate">{node.label}</span>
                            </button>
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
