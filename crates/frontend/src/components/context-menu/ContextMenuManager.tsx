/**
 * ContextMenuManager — wires the context menu store to the ContextMenu component.
 *
 * Builds the correct menu items depending on whether the right-click
 * happened on a node (or selection) or on empty canvas, and connects
 * actions to the clipboard store, flow store, and action store.
 */

import { useCallback, useEffect } from "react";
import type { MenuItem } from "../../store/context-menu-store";
import { useContextMenuStore } from "../../store/context-menu-store";
import { useClipboardStore } from "../../store/clipboard-store";
import { useFlowStore } from "../../store/flow-store";
import { useEditorStore } from "../../store/editor-store";
import { useGroupStore } from "../../store/group-store";
import { eventBus } from "../../red/core/events";
import { ContextMenu } from "./ContextMenu";
import {
  Scissors,
  Copy,
  ClipboardPaste,
  Trash2,
  Duplicate,
  Edit3,
  Group,
  Download,
  Import,
  CheckSquare,
  Layers,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Delete selected nodes and their connected edges */
function deleteSelectedNodes() {
  const { nodes, edges } = useFlowStore.getState();
  const selectedNodes = nodes.filter((n) => n.selected);
  if (selectedNodes.length === 0) return;

  const ids = new Set(selectedNodes.map((n) => n.id));
  const remainingNodes = nodes.filter((n) => !ids.has(n.id));
  const remainingEdges = edges.filter(
    (e) => !ids.has(e.source) && !ids.has(e.target),
  );

  useFlowStore.setState({ nodes: remainingNodes, edges: remainingEdges });
  for (const node of selectedNodes) {
    eventBus.emit("nodes:removed", { id: node.id });
  }
}

/** Duplicate selected nodes via clipboard */
function duplicateSelectedNodes() {
  const { nodes } = useFlowStore.getState();
  const selectedIds = nodes.filter((n) => n.selected).map((n) => n.id);
  if (selectedIds.length === 0) return;

  useClipboardStore.getState().copy(selectedIds);
  useClipboardStore.getState().paste(20, 20);
}

/** Select all nodes */
function selectAllNodes() {
  const { nodes } = useFlowStore.getState();
  useFlowStore.setState({
    nodes: nodes.map((n) => ({ ...n, selected: true })),
  });
}

/** Export selected nodes (or all if none selected) */
function exportNodes() {
  const { nodes } = useFlowStore.getState();
  const selectedIds = nodes.filter((n) => n.selected).map((n) => n.id);
  const json = useClipboardStore
    .getState()
    .exportToJson(selectedIds.length > 0 ? selectedIds : undefined);

  // Create a blob and trigger download
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "flows.json";
  a.click();
  URL.revokeObjectURL(url);
}

/** Import from JSON — trigger the flows:imported flow via a file dialog */
function importNodes() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.onchange = (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        useClipboardStore.getState().importFromJson(reader.result as string);
      } catch {
        // Silently fail — could add notification
      }
    };
    reader.readAsText(file);
  };
  input.click();
}

/** Convert selected nodes into a subflow */
function createSubflowFromNodes(selectedNodeIds: string[]) {
  if (selectedNodeIds.length < 2) return;

  // Import inline to avoid circular deps at module level
  const { useSubflowStore } = require("../../store/subflow-store") as {
    useSubflowStore: {
      getState: () => {
        convertToSubflow: (ids: string[]) => { subflow: { id: string; name: string } };
      };
    };
  };
  const result = useSubflowStore.getState().convertToSubflow(selectedNodeIds);

  // Remove the original nodes from the canvas — they now live inside the subflow
  const { nodes, edges } = useFlowStore.getState();
  const ids = new Set(selectedNodeIds);
  const remainingNodes = nodes.filter((n) => !ids.has(n.id));
  const remainingEdges = edges.filter(
    (e) => !ids.has(e.source) && !ids.has(e.target),
  );
  useFlowStore.setState({ nodes: remainingNodes, edges: remainingEdges });

  // Add a subflow instance node to the canvas
  useFlowStore.getState().addNode({
    id: result.subflow.id,
    type: "subflow",
    position: { x: 100, y: 100 },
    data: {
      label: result.subflow.name,
      subflowId: result.subflow.id,
    },
  });

  eventBus.emit("subflow:created", {
    id: result.subflow.id,
    name: result.subflow.name,
  });
}

/** Group selected nodes into a new visual group */
function groupSelectedNodes(selectedNodeIds: string[]) {
  if (selectedNodeIds.length < 2) return;

  const groupId = useGroupStore.getState().createGroup(selectedNodeIds);
  const group = useGroupStore.getState().groups.find((g) => g.id === groupId);
  if (!group) return;

  // Add a group node to the flow store so React Flow renders it
  useFlowStore.getState().addNode({
    id: groupId,
    type: "group",
    position: { x: group.x, y: group.y },
    data: {
      label: group.label,
      nodes: group.nodes,
      fill: group.style?.fill,
      stroke: group.style?.stroke,
    },
    style: { width: group.w, height: group.h },
    selectable: true,
    draggable: false, // We handle drag ourselves
    zIndex: -1, // Render behind regular nodes
  });
}

// ---------------------------------------------------------------------------
// Menu item builders
// ---------------------------------------------------------------------------

function buildNodeMenuItems(selectedNodeIds: string[]): MenuItem[] {
  const multipleSelected = selectedNodeIds.length > 1;
  const clipboard = useClipboardStore.getState();

  return [
    {
      label: "Cut",
      shortcut: "Ctrl-X",
      icon: <Scissors size={14} />,
      action: () => clipboard.cut(selectedNodeIds),
    },
    {
      label: "Copy",
      shortcut: "Ctrl-C",
      icon: <Copy size={14} />,
      action: () => clipboard.copy(selectedNodeIds),
    },
    {
      label: "Paste",
      shortcut: "Ctrl-V",
      icon: <ClipboardPaste size={14} />,
      action: () => clipboard.paste(),
      disabled: !clipboard.hasClipboard,
    },
    { label: "", separator: true },
    {
      label: "Duplicate",
      icon: <Duplicate size={14} />,
      action: duplicateSelectedNodes,
    },
    {
      label: "Delete",
      icon: <Trash2 size={14} />,
      action: deleteSelectedNodes,
    },
    { label: "", separator: true },
    {
      label: "Edit",
      shortcut: "Double-click",
      icon: <Edit3 size={14} />,
      action: () => {
        if (selectedNodeIds.length === 1) {
          useEditorStore.getState().selectNode(selectedNodeIds[0]);
          eventBus.emit("node:edit-requested", { id: selectedNodeIds[0] });
        }
      },
      disabled: selectedNodeIds.length !== 1,
    },
    { label: "", separator: true },
    ...(multipleSelected
      ? [
          {
            label: "Create Subflow",
            icon: <Layers size={14} />,
            action: () => createSubflowFromNodes(selectedNodeIds),
          } as MenuItem,
          {
            label: "Group",
            icon: <Group size={14} />,
            action: () => groupSelectedNodes(selectedNodeIds),
          } as MenuItem,
          { label: "", separator: true } as MenuItem,
        ]
      : []),
    {
      label: "Export",
      icon: <Download size={14} />,
      action: exportNodes,
    },
  ];
}

function buildCanvasMenuItems(): MenuItem[] {
  const clipboard = useClipboardStore.getState();

  return [
    {
      label: "Paste",
      shortcut: "Ctrl-V",
      icon: <ClipboardPaste size={14} />,
      action: () => clipboard.paste(),
      disabled: !clipboard.hasClipboard,
    },
    { label: "", separator: true },
    {
      label: "Select All",
      shortcut: "Ctrl-A",
      icon: <CheckSquare size={14} />,
      action: selectAllNodes,
    },
    { label: "", separator: true },
    {
      label: "Import",
      icon: <Import size={14} />,
      action: importNodes,
    },
  ];
}

// ---------------------------------------------------------------------------
// Manager component
// ---------------------------------------------------------------------------

export function ContextMenuManager() {
  const visible = useContextMenuStore((s) => s.visible);
  const position = useContextMenuStore((s) => s.position);
  const selectedNodeIds = useContextMenuStore((s) => s.selectedNodeIds);
  const isOnNode = useContextMenuStore((s) => s.isOnNode);
  const hide = useContextMenuStore((s) => s.hide);

  // Refresh items reactively: rebuild when visibility or selection changes
  // Items are computed on each render so they always reflect latest store state
  const items = isOnNode
    ? buildNodeMenuItems(selectedNodeIds)
    : buildCanvasMenuItems();

  const handleClose = useCallback(() => {
    hide();
  }, [hide]);

  // Also close on Escape globally
  useEffect(() => {
    if (!visible) return;

    function handleEscape(e: KeyboardEvent) {
      if (e.key === "Escape") {
        hide();
      }
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [visible, hide]);

  if (!visible) return null;

  return (
    <ContextMenu
      x={position.x}
      y={position.y}
      items={items}
      onClose={handleClose}
    />
  );
}
