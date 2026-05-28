/**
 * ConfigTab -- Configuration node management panel.
 *
 * Lists all configuration nodes grouped by type, mirroring Node-RED's
 * RED.sidebar.config tab. Config nodes are identified by the `_isConfig`
 * flag on their ReactFlow node data.
 */

import { useState, useMemo, useCallback } from "react";
import { Search, ChevronRight, X } from "lucide-react";
import { useFlowStore } from "../../../store/flow-store";
import { useEditorStore } from "../../../store/editor-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ConfigNodeEntry {
  id: string;
  type: string;
  label: string;
  users: string[];
}

interface ConfigGroup {
  type: string;
  nodes: ConfigNodeEntry[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Extract config nodes from the flow store. */
function getConfigNodes(nodes: ReturnType<typeof useFlowStore.getState>["nodes"]): ConfigNodeEntry[] {
  const result: ConfigNodeEntry[] = [];
  for (const n of nodes) {
    const d = n.data as Record<string, unknown>;
    if (d?._isConfig) {
      result.push({
        id: n.id,
        type: (d.type as string) || "unknown",
        label: (d.label as string) || (d.type as string) || n.id,
        users: Array.isArray(d._users) ? (d._users as string[]) : [],
      });
    }
  }
  return result;
}

/** Group config nodes: used ones by type, unused at the bottom. */
function groupConfigNodes(configs: ConfigNodeEntry[]): ConfigGroup[] {
  const usedByType = new Map<string, ConfigNodeEntry[]>();
  const unused: ConfigNodeEntry[] = [];

  for (const cfg of configs) {
    if (cfg.users.length === 0) {
      unused.push(cfg);
    } else {
      const list = usedByType.get(cfg.type) ?? [];
      list.push(cfg);
      usedByType.set(cfg.type, list);
    }
  }

  const groups: ConfigGroup[] = [];

  // Sort used groups alphabetically by type
  const sortedTypes = Array.from(usedByType.keys()).sort();
  for (const type of sortedTypes) {
    groups.push({ type, nodes: usedByType.get(type)! });
  }

  // Add unused group at the bottom if there are any
  if (unused.length > 0) {
    groups.push({ type: "(unused)", nodes: unused });
  }

  return groups;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function ConfigGroupRow({
  group,
  expanded,
  onToggle,
  onSelectNode,
  onDeleteNode,
  isUnused,
}: {
  group: ConfigGroup;
  expanded: boolean;
  onToggle: () => void;
  onSelectNode: (id: string) => void;
  onDeleteNode: (id: string) => void;
  isUnused: boolean;
}) {
  return (
    <div data-testid="config-group" data-group-type={group.type}>
      {/* Group header */}
      <button
        className="flex items-center w-full px-2 py-1.5 text-xs font-semibold text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 select-none"
        onClick={onToggle}
        data-testid="config-group-header"
      >
        <ChevronRight
          size={14}
          className={`mr-1 shrink-0 transition-transform ${
            expanded ? "rotate-90" : ""
          }`}
        />
        <span className="truncate">{group.type}</span>
        <span className="ml-auto text-gray-400 dark:text-gray-500 tabular-nums">
          ({group.nodes.length})
        </span>
      </button>

      {/* Config node items */}
      {expanded && (
        <ul data-testid="config-group-list">
          {group.nodes.map((node) => (
            <li
              key={node.id}
              className="flex items-center px-3 py-1 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-750 cursor-pointer group"
              data-testid="config-node-item"
              data-node-id={node.id}
              onClick={() => onSelectNode(node.id)}
            >
              <span className="truncate flex-1" data-testid="config-node-label">
                {node.label}
              </span>

              {/* User count badge */}
              <span
                className="ml-2 shrink-0 text-gray-400 dark:text-gray-500 tabular-nums"
                data-testid="config-node-user-count"
              >
                {isUnused ? "unused" : `${node.users.length} user${node.users.length !== 1 ? "s" : ""}`}
              </span>

              {/* Delete button for unused config nodes */}
              {isUnused && (
                <button
                  className="ml-1 shrink-0 p-0.5 rounded text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDeleteNode(node.id);
                  }}
                  data-testid="config-node-delete"
                  aria-label={`Delete ${node.label}`}
                >
                  <X size={12} />
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function ConfigTab() {
  const nodes = useFlowStore((s) => s.nodes);
  const removeNode = useFlowStore((s) => s.removeNode);
  const selectNode = useEditorStore((s) => s.selectNode);

  const [searchQuery, setSearchQuery] = useState("");
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());

  // Derive config nodes from the flow store
  const configNodes = useMemo(() => getConfigNodes(nodes), [nodes]);

  // Filter by search query
  const filteredNodes = useMemo(() => {
    if (!searchQuery.trim()) return configNodes;
    const q = searchQuery.toLowerCase();
    return configNodes.filter(
      (n) =>
        n.type.toLowerCase().includes(q) ||
        n.label.toLowerCase().includes(q),
    );
  }, [configNodes, searchQuery]);

  // Group the filtered nodes
  const groups = useMemo(() => groupConfigNodes(filteredNodes), [filteredNodes]);

  const toggleGroup = useCallback((type: string) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  }, []);

  const handleSelectNode = useCallback(
    (id: string) => {
      selectNode(id);
    },
    [selectNode],
  );

  const handleDeleteNode = useCallback(
    (id: string) => {
      removeNode(id);
    },
    [removeNode],
  );

  // Auto-expand all groups when search is active
  const effectiveExpanded = searchQuery.trim()
    ? new Set(groups.map((g) => g.type))
    : expandedGroups;

  return (
    <div
      className="flex flex-col h-full"
      data-testid="sidebar-tab-content-config"
    >
      {/* Search filter */}
      <div className="px-2 py-1.5 border-b border-gray-200 dark:border-gray-700">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400"
          />
          <input
            type="text"
            className="w-full pl-7 pr-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded focus:outline-none focus:ring-1 focus:ring-blue-400 text-gray-700 dark:text-gray-300 placeholder-gray-400"
            placeholder="Search configs..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            data-testid="config-search-input"
          />
        </div>
      </div>

      {/* Config groups */}
      <div className="flex-1 overflow-y-auto">
        {groups.length === 0 ? (
          <div
            className="flex items-center justify-center py-8 text-xs text-gray-400"
            data-testid="config-empty-state"
          >
            No configuration nodes
          </div>
        ) : (
          groups.map((group) => (
            <ConfigGroupRow
              key={group.type}
              group={group}
              expanded={effectiveExpanded.has(group.type)}
              onToggle={() => toggleGroup(group.type)}
              onSelectNode={handleSelectNode}
              onDeleteNode={handleDeleteNode}
              isUnused={group.type === "(unused)"}
            />
          ))
        )}
      </div>
    </div>
  );
}
