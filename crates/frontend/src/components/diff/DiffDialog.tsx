/**
 * DiffDialog -- modal dialog for visualizing flow diffs.
 *
 * Features:
 *  - Two-column layout: left = local, right = remote
 *  - Color coding: green = added, red = removed, yellow = modified, grey = unchanged
 *  - Property-level diff for modified nodes
 *  - Filter toggle: show all / changes only
 *  - "Merge" and "Overwrite" action buttons
 *  - Node count summary: "N added, N removed, N modified"
 */

import { useCallback } from "react";
import { X, GitCompare, Loader2 } from "lucide-react";
import {
  useDiffStore,
  type DiffFilter,
} from "../../store/diff-store";
import { useFlowStore } from "../../store/flow-store";
import { useDeployStore } from "../../store/deploy-store";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface DiffDialogProps {
  /** Whether the dialog is open */
  open: boolean;
  /** Close the dialog */
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function nodeLabel(node: { id: string; data?: { label?: string; type?: string } }): string {
  return node.data?.label || node.data?.type || node.id;
}

function nodeType(node: { data?: { type?: string } }): string {
  return node.data?.type || "unknown";
}

// ---------------------------------------------------------------------------
// Summary bar
// ---------------------------------------------------------------------------

function SummaryBar() {
  const diffResult = useDiffStore((s) => s.diffResult);

  if (!diffResult) return null;

  const { added, removed, modified } = diffResult;

  return (
    <div className="flex items-center gap-3 text-xs text-gray-600 dark:text-gray-400">
      {added.length > 0 && (
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-green-500" />
          {added.length} added
        </span>
      )}
      {removed.length > 0 && (
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-red-500" />
          {removed.length} removed
        </span>
      )}
      {modified.length > 0 && (
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-yellow-500" />
          {modified.length} modified
        </span>
      )}
      {added.length === 0 && removed.length === 0 && modified.length === 0 && (
        <span className="text-green-600 dark:text-green-400">No changes detected</span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Node row
// ---------------------------------------------------------------------------

type NodeStatus = "added" | "removed" | "modified" | "unchanged";

const statusColors: Record<NodeStatus, string> = {
  added: "bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700",
  removed: "bg-red-50 dark:bg-red-900/20 border-red-300 dark:border-red-700",
  modified: "bg-yellow-50 dark:bg-yellow-900/20 border-yellow-300 dark:border-yellow-700",
  unchanged: "bg-gray-50 dark:bg-gray-800/50 border-gray-200 dark:border-gray-600",
};

const statusBadgeColors: Record<NodeStatus, string> = {
  added: "bg-green-500 text-white",
  removed: "bg-red-500 text-white",
  modified: "bg-yellow-500 text-white",
  unchanged: "bg-gray-400 text-white",
};

interface NodeRowProps {
  node: { id: string; data?: { label?: string; type?: string } };
  status: NodeStatus;
  side: "local" | "remote";
  changes?: Record<string, { old: unknown; new: unknown }>;
}

function NodeRow({ node, status, side, changes }: NodeRowProps) {
  const label = nodeLabel(node);
  const type = nodeType(node);
  const colorClass = statusColors[status];
  const badgeClass = statusBadgeColors[status];

  return (
    <div
      className={`rounded border px-3 py-2 text-xs ${colorClass}`}
      data-testid={`diff-node-${status}`}
    >
      <div className="flex items-center gap-2">
        <span
          className={`inline-flex items-center justify-center px-1.5 py-0.5 rounded text-[10px] font-medium uppercase ${badgeClass}`}
        >
          {status}
        </span>
        <span className="font-medium text-gray-900 dark:text-gray-100">
          {label}
        </span>
        <span className="text-gray-400 dark:text-gray-500">
          ({type})
        </span>
        <span className="text-gray-400 dark:text-gray-500 ml-auto">
          {side}
        </span>
      </div>

      {/* Property-level diff for modified nodes */}
      {status === "modified" && changes && (
        <div className="mt-2 pl-4 space-y-1">
          {Object.entries(changes).map(([key, { old, new: newVal }]) => (
            <div
              key={key}
              className="flex items-start gap-2 text-[11px]"
              data-testid={`diff-property-${key}`}
            >
              <span className="font-mono text-gray-600 dark:text-gray-400 min-w-[60px]">
                {key}:
              </span>
              <span className="text-red-600 dark:text-red-400 line-through">
                {formatValue(old)}
              </span>
              <span className="text-gray-400">-&gt;</span>
              <span className="text-green-600 dark:text-green-400">
                {formatValue(newVal)}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function formatValue(val: unknown): string {
  if (val === undefined) return "(undefined)";
  if (val === null) return "(null)";
  if (typeof val === "object") {
    try {
      return JSON.stringify(val);
    } catch {
      return String(val);
    }
  }
  return String(val);
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function DiffDialog({ open, onClose }: DiffDialogProps) {
  const diffResult = useDiffStore((s) => s.diffResult);
  const isLoading = useDiffStore((s) => s.isLoading);
  const filter = useDiffStore((s) => s.filter);
  const setFilter = useDiffStore((s) => s.setFilter);
  const diffStoreClear = useDiffStore((s) => s.clear);

  const handleClose = useCallback(() => {
    diffStoreClear();
    onClose();
  }, [diffStoreClear, onClose]);

  const handleOverwrite = useCallback(() => {
    if (!diffResult) return;

    // Replace local with remote: load remote nodes into flow store
    const { remoteFlow } = useDiffStore.getState();
    if (remoteFlow) {
      useFlowStore.getState().setNodes(remoteFlow);
      useDeployStore.getState().setHasUnsavedChanges(true);
    }

    handleClose();
  }, [diffResult, handleClose]);

  const handleMerge = useCallback(() => {
    if (!diffResult) return;

    // Merge: keep added nodes from remote, keep local modifications
    const { remoteFlow, localFlow } = useDiffStore.getState();
    if (!remoteFlow || !localFlow) return;

    const remoteMap = new Map(remoteFlow.map((n) => [n.id, n]));
    const localMap = new Map(localFlow.map((n) => [n.id, n]));
    const merged: typeof localFlow = [];

    // Start with all local nodes
    for (const node of localFlow) {
      merged.push(node);
    }

    // Add remote nodes that are missing locally (were added on remote)
    for (const [id, remoteNode] of remoteMap) {
      if (!localMap.has(id)) {
        merged.push(remoteNode);
      }
    }

    useFlowStore.getState().setNodes(merged);
    useDeployStore.getState().setHasUnsavedChanges(true);
    handleClose();
  }, [diffResult, handleClose]);

  if (!open) return null;

  // Build filtered node lists
  const hasChanges =
    diffResult &&
    (diffResult.added.length > 0 ||
      diffResult.removed.length > 0 ||
      diffResult.modified.length > 0);

  const localNodes: NodeRowProps[] = [];
  const remoteNodes: NodeRowProps[] = [];

  if (diffResult) {
    // Added nodes: show in local column only
    for (const node of diffResult.added) {
      localNodes.push({ node, status: "added", side: "local" });
    }

    // Removed nodes: show in remote column only
    for (const node of diffResult.removed) {
      remoteNodes.push({ node, status: "removed", side: "remote" });
    }

    // Modified nodes: show in both columns
    for (const { node, changes } of diffResult.modified) {
      remoteNodes.push({
        node: useDiffStore.getState().remoteFlow?.find((n) => n.id === node.id) ?? node,
        status: "modified",
        side: "remote",
        changes,
      });
      localNodes.push({ node, status: "modified", side: "local", changes });
    }

    // Unchanged: show in both columns (when filter is "all")
    if (filter === "all") {
      for (const node of diffResult.unchanged) {
        localNodes.push({ node, status: "unchanged", side: "local" });
        remoteNodes.push({ node, status: "unchanged", side: "remote" });
      }
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        className="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[800px] max-h-[80vh] flex flex-col"
        data-testid="diff-dialog"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <GitCompare size={18} className="text-gray-500" />
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Compare with Deployed
            </h2>
          </div>
          <button
            type="button"
            onClick={handleClose}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-500"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        {/* Summary + filter bar */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-gray-100 dark:border-gray-700">
          <SummaryBar />
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setFilter("changes")}
              className={`px-2 py-1 text-xs rounded ${
                filter === "changes"
                  ? "bg-gray-200 dark:bg-gray-600 text-gray-900 dark:text-gray-100"
                  : "text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
              }`}
              data-testid="diff-filter-changes"
            >
              Changes only
            </button>
            <button
              type="button"
              onClick={() => setFilter("all")}
              className={`px-2 py-1 text-xs rounded ${
                filter === "all"
                  ? "bg-gray-200 dark:bg-gray-600 text-gray-900 dark:text-gray-100"
                  : "text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
              }`}
              data-testid="diff-filter-all"
            >
              Show all
            </button>
          </div>
        </div>

        {/* Two-column diff body */}
        <div className="flex-1 overflow-auto p-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="animate-spin text-gray-400" size={24} />
              <span className="ml-2 text-sm text-gray-500">Loading diff...</span>
            </div>
          ) : !diffResult ? (
            <div className="flex items-center justify-center py-12 text-sm text-gray-500">
              No diff data available
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-4">
              {/* Local column */}
              <div>
                <div className="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-2 uppercase tracking-wide">
                  Local (Editor)
                </div>
                <div className="space-y-2">
                  {localNodes.map((props) => (
                    <NodeRow key={`local-${props.node.id}`} {...props} />
                  ))}
                  {localNodes.length === 0 && (
                    <div className="text-xs text-gray-400 py-4 text-center">
                      No local changes
                    </div>
                  )}
                </div>
              </div>

              {/* Remote column */}
              <div>
                <div className="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-2 uppercase tracking-wide">
                  Remote (Deployed)
                </div>
                <div className="space-y-2">
                  {remoteNodes.map((props) => (
                    <NodeRow key={`remote-${props.node.id}`} {...props} />
                  ))}
                  {remoteNodes.length === 0 && (
                    <div className="text-xs text-gray-400 py-4 text-center">
                      No remote changes
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer with action buttons */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-gray-200 dark:border-gray-700">
          <button
            type="button"
            onClick={handleClose}
            className="px-3 py-1.5 text-xs rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            data-testid="diff-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleMerge}
            disabled={!hasChanges}
            className="px-3 py-1.5 text-xs rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            data-testid="diff-merge"
          >
            Merge
          </button>
          <button
            type="button"
            onClick={handleOverwrite}
            disabled={!diffResult?.removed?.length && !diffResult?.modified?.length}
            className="px-3 py-1.5 text-xs rounded bg-orange-600 text-white hover:bg-orange-700 disabled:opacity-50 disabled:cursor-not-allowed"
            data-testid="diff-overwrite"
          >
            Overwrite Local
          </button>
        </div>
      </div>
    </div>
  );
}

export default DiffDialog;
