/**
 * InfoTab -- Node information panel matching Node-RED's RED.sidebar.info.
 *
 * States:
 *   1. No selection -- shows flow info or "select a node" placeholder
 *   2. Single node selected -- shows node type, properties, description
 *   3. Multiple selection -- shows "[N] nodes selected"
 *
 * Reads selected node from editor-store and node data from flow-store.
 * Node type definitions come from the node registry.
 */

import { useEffect, useState, useCallback } from "react";
import { useEditorStore } from "../../../store/editor-store";
import { useFlowStore } from "../../../store/flow-store";
import { useWorkspaceStore } from "../../../store/workspace-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { eventBus } from "../../../red/core/events";
import type { Node } from "@xyflow/react";
import type { NodeDefinition } from "../../../red/nodes/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format a node property value for display. */
function formatValue(val: unknown): string {
  if (val === undefined || val === null) return "";
  if (typeof val === "string") return val || "";
  if (Array.isArray(val)) {
    if (val.length === 0) return "[]";
    // Show array of arrays (like wires) in compact form
    return JSON.stringify(val);
  }
  if (typeof val === "object") return JSON.stringify(val);
  return String(val);
}

/** Get the node color from the registry, falling back to a default. */
function getNodeColor(type: string): string {
  return nodeRegistry.getNodeColor(type) || "#aaaaaa";
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Node icon badge -- colored circle with type initial. */
function NodeIconBadge({ type }: { type: string }) {
  const color = getNodeColor(type);
  const initial = type.charAt(0).toUpperCase();

  return (
    <div
      className="w-6 h-6 rounded flex items-center justify-center text-[10px] font-bold text-white shrink-0"
      style={{ backgroundColor: color }}
      data-testid="info-node-icon"
    >
      {initial}
    </div>
  );
}

/** Header bar showing the selected node type and label. */
function InfoHeader({
  label,
  type,
}: {
  label: string;
  type: string;
}) {
  return (
    <div
      className="flex items-center gap-2 px-3 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900"
      data-testid="info-header"
    >
      <NodeIconBadge type={type} />
      <div className="min-w-0 flex-1">
        <div className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
          {label}
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400">
          {type}
        </div>
      </div>
    </div>
  );
}

/** Row in the properties table. */
function PropertyRow({ name, value }: { name: string; value: string }) {
  return (
    <tr className="border-b border-gray-100 dark:border-gray-700/50">
      <td className="px-3 py-1 text-xs text-gray-500 dark:text-gray-400 align-top whitespace-nowrap w-1/3">
        {name}
      </td>
      <td
        className="px-3 py-1 text-xs text-gray-800 dark:text-gray-200 break-all"
        data-testid={`info-property-${name}`}
      >
        {value}
      </td>
    </tr>
  );
}

/** Properties section showing a key-value table of node properties. */
function PropertiesSection({
  node,
  definition,
}: {
  node: Node;
  definition?: NodeDefinition;
}) {
  const defaults = definition?.defaults;
  const entries: Array<{ key: string; value: string }> = [];

  // Show "id" and "type" first
  entries.push({ key: "id", value: node.id });
  entries.push({ key: "type", value: String(node.data?.type ?? node.type) });

  // If we have defaults, iterate them
  if (defaults) {
    for (const [key, def] of Object.entries(defaults)) {
      // Skip name/info -- name shown in header, info shown in description section
      if (key === "name" || key === "info") continue;
      const val = (node.data as Record<string, unknown>)?.[key];
      entries.push({ key, value: formatValue(val ?? def.value) });
    }
  } else {
    // No definition -- show all data keys except internal ones
    const skip = new Set(["id", "type", "x", "y", "z", "wires", "name", "info"]);
    for (const [key, val] of Object.entries(node.data as Record<string, unknown>)) {
      if (skip.has(key)) continue;
      entries.push({ key, value: formatValue(val) });
    }
  }

  return (
    <div data-testid="info-properties">
      <div className="px-3 py-1.5 text-[10px] font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider border-b border-gray-200 dark:border-gray-700">
        Properties
      </div>
      <table className="w-full">
        <tbody>
          {entries.map(({ key, value }) => (
            <PropertyRow key={key} name={key} value={value} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Editable description textarea for the node. */
function DescriptionSection({
  node,
  onUpdate,
}: {
  node: Node;
  onUpdate: (text: string) => void;
}) {
  const description = String(
    (node.data as Record<string, unknown>)?.info ?? ""
  );
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(description);

  // Sync text when node changes
  useEffect(() => {
    setText(description);
    setEditing(false);
  }, [description]);

  const handleBlur = useCallback(() => {
    setEditing(false);
    if (text !== description) {
      onUpdate(text);
    }
  }, [text, description, onUpdate]);

  return (
    <div data-testid="info-description">
      <div className="px-3 py-1.5 text-[10px] font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider border-b border-gray-200 dark:border-gray-700">
        Description
      </div>
      <div className="px-3 py-2">
        {editing ? (
          <textarea
            className="w-full h-24 text-xs bg-gray-100 dark:bg-gray-900 text-gray-800 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded p-2 resize-y focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={text}
            onChange={(e) => setText(e.target.value)}
            onBlur={handleBlur}
            autoFocus
            data-testid="info-description-textarea"
          />
        ) : (
          <div
            className="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap min-h-[2rem] cursor-text rounded p-2 hover:bg-gray-100 dark:hover:bg-gray-900"
            onClick={() => setEditing(true)}
            data-testid="info-description-text"
          >
            {description || (
              <span className="text-gray-400 dark:text-gray-500 italic">
                No description. Click to edit.
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/** "No selection" state -- shows flow info. */
function NoSelectionState() {
  const flows = useWorkspaceStore((s) => s.flows);
  const activeFlowId = useWorkspaceStore((s) => s.activeFlowId);
  const nodes = useFlowStore((s) => s.nodes);
  const activeFlow = flows.find((f) => f.id === activeFlowId);

  // Count nodes in active flow
  const flowNodeCount = activeFlowId
    ? nodes.filter((n) => n.data?.z === activeFlowId || (n.parentId === undefined && activeFlowId === n.data?.z)).length
    : 0;

  return (
    <div className="flex-1 flex flex-col" data-testid="info-no-selection">
      {/* Flow info */}
      {activeFlow ? (
        <div>
          <InfoHeader label={activeFlow.label} type="flow" />
          <div className="px-3 py-2">
            <div className="text-xs text-gray-500 dark:text-gray-400">
              Flow Information
            </div>
            <table className="w-full mt-1">
              <tbody>
                <tr className="border-b border-gray-100 dark:border-gray-700/50">
                  <td className="px-1 py-1 text-xs text-gray-500 dark:text-gray-400 w-1/3">Name</td>
                  <td className="px-1 py-1 text-xs text-gray-800 dark:text-gray-200">
                    {activeFlow.label}
                  </td>
                </tr>
                <tr className="border-b border-gray-100 dark:border-gray-700/50">
                  <td className="px-1 py-1 text-xs text-gray-500 dark:text-gray-400">ID</td>
                  <td className="px-1 py-1 text-xs text-gray-800 dark:text-gray-200 font-mono text-[10px]">
                    {activeFlow.id}
                  </td>
                </tr>
                <tr className="border-b border-gray-100 dark:border-gray-700/50">
                  <td className="px-1 py-1 text-xs text-gray-500 dark:text-gray-400">Nodes</td>
                  <td className="px-1 py-1 text-xs text-gray-800 dark:text-gray-200">
                    {flowNodeCount}
                  </td>
                </tr>
                {activeFlow.info && (
                  <tr>
                    <td className="px-1 py-1 text-xs text-gray-500 dark:text-gray-400 align-top">Info</td>
                    <td className="px-1 py-1 text-xs text-gray-800 dark:text-gray-200 whitespace-pre-wrap">
                      {activeFlow.info}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center p-4">
          <div className="text-center">
            <p className="text-sm text-gray-400 dark:text-gray-500">
              Select a node to view its information.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

/** Multiple selection state. */
function MultipleSelectionState({ count }: { count: number }) {
  return (
    <div className="flex-1 flex flex-col" data-testid="info-multiple-selection">
      <InfoHeader label={`${count} nodes selected`} type="_selection_" />
      <div className="flex-1 flex items-center justify-center p-4">
        <p className="text-sm text-gray-400 dark:text-gray-500">
          {count} nodes selected
        </p>
      </div>
    </div>
  );
}

/** Single node selected -- the main info view. */
function SingleNodeInfo({ node }: { node: Node }) {
  const updateNodeData = useFlowStore((s) => s.updateNodeData);

  const nodeType = String(node.data?.type ?? node.type);
  const definition = nodeRegistry.getType(nodeType);

  // Resolve label
  const label =
    definition && typeof definition.label === "function"
      ? definition.label(node.data as Record<string, unknown>)
      : (node.data?.name as string) ||
        (definition?.paletteLabel as string) ||
        nodeType;

  const handleDescriptionUpdate = useCallback(
    (text: string) => {
      updateNodeData(node.id, { info: text });
    },
    [node.id, updateNodeData]
  );

  return (
    <div className="flex-1 flex flex-col" data-testid="info-single-node">
      <InfoHeader label={label} type={nodeType} />

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto">
        <PropertiesSection node={node} definition={definition} />
        <DescriptionSection node={node} onUpdate={handleDescriptionUpdate} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main InfoTab component
// ---------------------------------------------------------------------------

export function InfoTab() {
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const nodes = useFlowStore((s) => s.nodes);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  // Listen for selection events from the event bus to support multi-select
  useEffect(() => {
    const unsub = eventBus.on("editor:select", (data) => {
      // The event bus carries the current single selection id.
      // For multi-select, the data could be extended in the future.
      setSelectedIds(data?.id ? [data.id as string] : []);
    });
    return unsub;
  }, []);

  // Derive state from selectedNodeId (primary) or event-driven selectedIds
  const effectiveId = selectedNodeId;

  // Find selected nodes
  const selectedNodes = effectiveId
    ? nodes.filter((n) => n.id === effectiveId)
    : [];

  // Determine which state to render
  if (selectedNodes.length === 0) {
    return (
      <div
        className="flex-1 flex flex-col h-full"
        data-testid="sidebar-tab-content-info"
      >
        <NoSelectionState />
      </div>
    );
  }

  if (selectedNodes.length > 1) {
    return (
      <div
        className="flex-1 flex flex-col h-full"
        data-testid="sidebar-tab-content-info"
      >
        <MultipleSelectionState count={selectedNodes.length} />
      </div>
    );
  }

  return (
    <div
      className="flex-1 flex flex-col h-full"
      data-testid="sidebar-tab-content-info"
    >
      <SingleNodeInfo node={selectedNodes[0]} />
    </div>
  );
}
