/**
 * NodeEditorPanel -- Node property editor rendered in the sidebar or as a
 * pop-out modal dialog.
 *
 * Layout:
 *   ┌──────────────────────────────────┐
 *   │ X  Edit <type> node        [^]  │  Header
 *   ├──────────────────────────────────┤
 *   │ Node Properties                  │
 *   │  Name:   [____________]          │
 *   │  <dynamic fields from defaults>  │
 *   │ Description                      │
 *   │  [______________________]        │
 *   ├──────────────────────────────────┤
 *   │              [Cancel]  [Done]    │  Footer
 *   └──────────────────────────────────┘
 */

import { useCallback, useEffect } from "react";
import { useEditorPanelStore } from "../../store/editor-panel-store";
import { useFlowStore } from "../../store/flow-store";
import { useSidebarStore } from "../../store/sidebar-store";
import { useEditorStore } from "../../store/editor-store";
import { nodeRegistry } from "../../red/nodes/registry";
import { eventBus } from "../../red/core/events";
import type { NodeDefault } from "../../red/nodes/types";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function NodeEditorPanel() {
  // All hooks must be called unconditionally (before any early return).
  const isOpen = useEditorPanelStore((s) => s.isOpen);
  const isModal = useEditorPanelStore((s) => s.isModal);
  const nodeId = useEditorPanelStore((s) => s.nodeId);
  const nodeType = useEditorPanelStore((s) => s.nodeType);
  const formData = useEditorPanelStore((s) => s.formData);
  const isDirty = useEditorPanelStore((s) => s.isDirty);
  const closeEditor = useEditorPanelStore((s) => s.closeEditor);
  const toggleModal = useEditorPanelStore((s) => s.toggleModal);
  const updateField = useEditorPanelStore((s) => s.updateField);

  // Derive display info from registry (safe even when closed)
  const def = nodeType ? nodeRegistry.getType(nodeType) : undefined;
  const nodeColor = def?.color ?? "#a6bbcf";
  const nodeIcon = def?.icon;
  const paletteLabel = def?.paletteLabel ?? nodeType ?? "node";
  const defaults = def?.defaults ?? {};

  // ----- Handlers (must be defined before any conditional return) -----

  const handleClose = useCallback(() => {
    if (useEditorPanelStore.getState().isDirty) {
      // eslint-disable-next-line no-alert
      const ok = window.confirm(
        "You have unsaved changes. Discard and close?",
      );
      if (!ok) return;
    }
    closeEditor();
  }, [closeEditor]);

  const handleCancel = useCallback(() => {
    closeEditor();
  }, [closeEditor]);

  const handleDone = useCallback(() => {
    const currentId = useEditorPanelStore.getState().nodeId;
    const currentFormData = useEditorPanelStore.getState().formData;
    if (!currentId) return;

    // Persist form data back to the flow store
    useFlowStore
      .getState()
      .updateNodeData(currentId, currentFormData);

    // Mark the panel as clean (saved)
    useEditorPanelStore.getState().markClean();

    // Emit node-edited event
    eventBus.emit("nodes:changed", { id: currentId });

    // Close the editor
    closeEditor();
  }, [closeEditor]);

  // ----- Keyboard: Escape to close -----

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        handleClose();
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [handleClose]);

  // ----- Early return AFTER all hooks -----

  if (!isOpen || !nodeId) return null;

  // ----- Body content -----

  const panel = (
    <div
      className="flex flex-col h-full bg-white dark:bg-gray-800"
      data-testid="node-editor-panel"
    >
      {/* Header */}
      <EditorHeader
        nodeColor={nodeColor}
        nodeIcon={nodeIcon}
        paletteLabel={paletteLabel}
        isModal={isModal}
        onClose={handleClose}
        onToggleModal={toggleModal}
      />

      {/* Body (scrollable) */}
      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-3">
        {/* Node name */}
        <div>
          <label
            className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
            htmlFor="node-editor-name"
          >
            Name
          </label>
          <input
            id="node-editor-name"
            type="text"
            className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={(formData.name as string) ?? ""}
            onChange={(e) => updateField("name", e.target.value)}
            data-testid="node-editor-field-name"
          />
        </div>

        {/* Dynamic property fields from node definition defaults */}
        {Object.entries(defaults).map(([key, defVal]) => (
          <DefaultField
            key={key}
            propKey={key}
            nodeDefault={defVal}
            value={formData[key]}
            onChange={(v) => updateField(key, v)}
          />
        ))}

        {/* Description */}
        <div>
          <label
            className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
            htmlFor="node-editor-description"
          >
            Description
          </label>
          <textarea
            id="node-editor-description"
            className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500 resize-y min-h-[60px]"
            value={(formData.info as string) ?? ""}
            onChange={(e) => updateField("info", e.target.value)}
            placeholder="Optional description..."
            data-testid="node-editor-field-info"
          />
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-end gap-2 px-3 py-2 border-t border-gray-200 dark:border-gray-700">
        <button
          type="button"
          className="px-3 py-1 text-sm rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
          onClick={handleCancel}
          data-testid="node-editor-btn-cancel"
        >
          Cancel
        </button>
        <button
          type="button"
          className="px-3 py-1 text-sm rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
          onClick={handleDone}
          data-testid="node-editor-btn-done"
        >
          Done
        </button>
      </div>
    </div>
  );

  // ----- Render: sidebar or modal -----

  if (isModal) {
    return (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
        data-testid="node-editor-modal-backdrop"
      >
        <div className="w-[480px] max-h-[80vh] rounded-lg shadow-xl overflow-hidden border border-gray-300 dark:border-gray-600">
          {panel}
        </div>
      </div>
    );
  }

  // Sidebar inline mode
  return panel;
}

// ---------------------------------------------------------------------------
// Header sub-component
// ---------------------------------------------------------------------------

interface EditorHeaderProps {
  nodeColor: string;
  nodeIcon: string | undefined;
  paletteLabel: string;
  isModal: boolean;
  onClose: () => void;
  onToggleModal: () => void;
}

function EditorHeader({
  nodeColor,
  nodeIcon,
  paletteLabel,
  isModal,
  onClose,
  onToggleModal,
}: EditorHeaderProps) {
  return (
    <div
      className="flex items-center gap-2 px-3 py-2 border-b border-gray-200 dark:border-gray-700"
      data-testid="node-editor-header"
    >
      {/* Close button */}
      <button
        type="button"
        className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 text-lg leading-none"
        onClick={onClose}
        aria-label="Close editor"
        data-testid="node-editor-btn-close"
      >
        &times;
      </button>

      {/* Type colour swatch + icon */}
      <span
        className="inline-flex items-center justify-center w-6 h-6 rounded text-xs"
        style={{ backgroundColor: nodeColor }}
      >
        {nodeIcon ? (
          <i className={`${nodeIcon} text-[10px]`} />
        ) : (
          <span className="w-2 h-2 rounded-sm bg-white/60" />
        )}
      </span>

      {/* Title */}
      <span className="text-sm font-semibold text-gray-800 dark:text-gray-100 flex-1 truncate">
        Edit {paletteLabel} node
      </span>

      {/* Pop-out / pop-in toggle */}
      <button
        type="button"
        className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 text-sm"
        onClick={onToggleModal}
        aria-label={isModal ? "Dock to sidebar" : "Pop out to modal"}
        title={isModal ? "Dock to sidebar" : "Pop out to modal"}
        data-testid="node-editor-btn-toggle-modal"
      >
        {isModal ? "\u21A9" : "\u2922"}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// DefaultField -- simple renderer for node defaults
// ---------------------------------------------------------------------------

interface DefaultFieldProps {
  propKey: string;
  nodeDefault: NodeDefault;
  value: unknown;
  onChange: (value: unknown) => void;
}

function DefaultField({ propKey, nodeDefault, value, onChange }: DefaultFieldProps) {
  // Render a basic text input for now.
  // The schema-form engine (Phase 3 Task 3) will replace this with richer
  // controls (typed-input, select, checkbox, etc.).
  const strValue = value === undefined || value === null
    ? String(nodeDefault.value ?? "")
    : String(value);

  return (
    <div>
      <label
        className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
        htmlFor={`node-editor-field-${propKey}`}
      >
        {propKey.charAt(0).toUpperCase() + propKey.slice(1)}
      </label>
      <input
        id={`node-editor-field-${propKey}`}
        type="text"
        className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
        value={strValue}
        onChange={(e) => onChange(e.target.value)}
        data-testid={`node-editor-field-${propKey}`}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Integration hook: listen for edit-requested events
// ---------------------------------------------------------------------------

/**
 * Mount this once at app root to automatically open the editor when
 * `node:edit-requested` fires.
 */
export function useNodeEditorIntegration() {
  const openEditor = useEditorPanelStore((s) => s.openEditor);

  useEffect(() => {
    const unsub = eventBus.on("node:edit-requested", (data) => {
      const { id } = data as { id: string };

      // Look up the node in the flow store
      const node = useFlowStore.getState().nodes.find((n) => n.id === id);
      if (!node) return;

      // Determine the node type from data or fall back
      const nType = (node.data?.type as string) ?? node.type ?? "unknown";

      // Open the sidebar if not already open
      useSidebarStore.getState().open();

      // Select the node
      useEditorStore.getState().selectNode(id);

      // Build form data from node.data (which holds the NR properties)
      const nodeData: Record<string, unknown> = { ...node.data };

      openEditor(id, nType, nodeData);
    });

    return unsub;
  }, [openEditor]);
}
