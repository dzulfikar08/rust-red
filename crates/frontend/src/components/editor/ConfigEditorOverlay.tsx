/**
 * ConfigEditorOverlay -- Sub-dialog for creating / editing config nodes.
 *
 * Renders as a modal overlay on top of the main node editor. Contains a
 * SchemaForm with the config node type's property defaults and Cancel /
 * Add (or Update) buttons.
 */

import { useCallback, useMemo, useState } from "react";
import { useFlowStore } from "../../store/flow-store";
import { nodeRegistry } from "../../red/nodes/registry";
import { SchemaForm } from "./schema-forms/SchemaForm";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ConfigEditorOverlayProps {
  /** Config node type to create/edit (e.g. 'mqtt-broker') */
  configType: string;
  /** ID of an existing config to edit, or null to create new */
  configId: string | null;
  /** Called with the (possibly new) config ID after save */
  onSave: (configId: string) => void;
  /** Called when the user cancels */
  onCancel: () => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ConfigEditorOverlay({
  configType,
  configId,
  onSave,
  onCancel,
}: ConfigEditorOverlayProps) {
  const nodes = useFlowStore((s) => s.nodes);
  const addNode = useFlowStore((s) => s.addNode);

  // Resolve the config type definition from the registry
  const configDef = nodeRegistry.getType(configType);
  const defaults = configDef?.defaults ?? {};
  const typeName =
    configDef?.paletteLabel ?? configDef?.name ?? configType;
  const isEditing = configId !== null;

  // Look up existing config data if editing
  const existingNode = useMemo(() => {
    if (!configId) return null;
    return nodes.find((n) => n.id === configId) ?? null;
  }, [nodes, configId]);

  const existingData = useMemo(() => {
    if (!existingNode) return {};
    const d = existingNode.data as Record<string, unknown>;
    // Build initial values from defaults + existing data
    const initial: Record<string, unknown> = {};
    for (const [key, schema] of Object.entries(defaults)) {
      initial[key] = d[key] ?? schema.value;
    }
    // Also preserve the name
    if (d.name !== undefined) initial.name = d.name;
    return initial;
  }, [existingNode, defaults]);

  // Form state
  const [formData, setFormData] = useState<Record<string, unknown>>(() => {
    if (isEditing) return existingData;
    // New config: initialise from defaults
    const initial: Record<string, unknown> = {};
    for (const [key, schema] of Object.entries(defaults)) {
      initial[key] = schema.value;
    }
    return initial;
  });

  const handleChange = useCallback((key: string, value: unknown) => {
    setFormData((prev) => ({ ...prev, [key]: value }));
  }, []);

  const handleSave = useCallback(() => {
    let id = configId;

    if (!isEditing || !id) {
      // Create a new config node
      id = `config-${configType}-${Date.now()}`;
      addNode({
        id,
        type: "nrNode",
        position: { x: 0, y: 0 },
        data: {
          _isConfig: true,
          type: configType,
          _users: [],
          ...formData,
        },
      });
    } else {
      // Update existing config node
      useFlowStore.getState().updateNodeData(id, {
        ...formData,
        _isConfig: true,
        type: configType,
      });
    }

    onSave(id);
  }, [configId, configType, isEditing, formData, addNode, onSave]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
      }
    },
    [onCancel],
  );

  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50"
      onKeyDown={handleKeyDown}
      data-testid="config-editor-overlay"
    >
      <div
        className="w-[420px] max-h-[70vh] flex flex-col bg-[#2a2a2a] border border-[#555] rounded-lg shadow-2xl overflow-hidden"
        data-testid="config-editor-dialog"
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-4 py-2 border-b border-[#555]"
          data-testid="config-editor-header"
        >
          <span className="text-sm font-semibold text-gray-100">
            {isEditing ? "Edit" : "Add"} {typeName} config
          </span>
          <button
            type="button"
            className="text-gray-400 hover:text-gray-200 text-lg leading-none"
            onClick={onCancel}
            aria-label="Close"
            data-testid="config-editor-btn-close"
          >
            &times;
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-4 py-3">
          {/* Name field */}
          <div className="mb-3">
            <label
              className="block text-xs font-medium text-gray-300 mb-1"
              htmlFor="config-editor-name"
            >
              Name
            </label>
            <input
              id="config-editor-name"
              type="text"
              className={
                "w-full border border-[#555] rounded bg-[#444] px-2 py-1.5 " +
                "text-sm text-gray-100 outline-none focus:border-[#777] " +
                "focus:ring-1 focus:ring-[#557da0]"
              }
              value={String(formData.name ?? "")}
              onChange={(e) => handleChange("name", e.target.value)}
              data-testid="config-editor-field-name"
            />
          </div>

          {/* Schema-driven property fields */}
          <SchemaForm
            schema={defaults}
            values={formData}
            onChange={handleChange}
          />
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-2 border-t border-[#555]">
          <button
            type="button"
            className={
              "px-3 py-1 text-sm rounded border border-[#555] text-gray-300 " +
              "hover:bg-[#333] transition-colors"
            }
            onClick={onCancel}
            data-testid="config-editor-btn-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            className={
              "px-3 py-1 text-sm rounded bg-blue-600 text-white " +
              "hover:bg-blue-700 transition-colors"
            }
            onClick={handleSave}
            data-testid="config-editor-btn-save"
          >
            {isEditing ? "Update" : "Add"}
          </button>
        </div>
      </div>
    </div>
  );
}
