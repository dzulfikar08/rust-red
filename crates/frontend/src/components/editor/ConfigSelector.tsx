/**
 * ConfigSelector -- Dropdown for selecting configuration node instances.
 *
 * When a node property references a config node type (e.g. `broker` on an
 * mqtt-out node referencing `mqtt-broker`), this component renders a
 * dropdown listing existing config nodes of that type plus an "Add new..."
 * option. An edit (pencil) button next to the dropdown allows editing the
 * currently selected config.
 */

import { useId, useMemo, useState, useCallback } from "react";
import { useFlowStore } from "../../store/flow-store";
import { nodeRegistry } from "../../red/nodes/registry";
import { ConfigEditorOverlay } from "./ConfigEditorOverlay";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ConfigSelectorProps {
  /** Config node type (e.g. 'mqtt-broker') */
  nodeType: string;
  /** Currently selected config node ID */
  value: string;
  /** Callback when the selected config changes */
  onChange: (configId: string) => void;
  /** Optional field label */
  label?: string;
  /** Whether the selector is disabled */
  disabled?: boolean;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface ConfigNodeOption {
  id: string;
  label: string;
}

/**
 * Extract config nodes of a given type from the flow store.
 */
function getConfigNodesByType(
  nodes: ReturnType<typeof useFlowStore.getState>["nodes"],
  configType: string,
): ConfigNodeOption[] {
  const result: ConfigNodeOption[] = [];
  for (const n of nodes) {
    const d = n.data as Record<string, unknown>;
    if (d?._isConfig && (d.type as string) === configType) {
      const displayLabel =
        (d.label as string) ||
        (d.name as string) ||
        (d.type as string) ||
        n.id;
      result.push({ id: n.id, label: displayLabel });
    }
  }
  return result;
}

/** Sentinel value used for the "Add new..." option */
export const ADD_NEW_SENTINEL = "__add_new__";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ConfigSelector({
  nodeType,
  value,
  onChange,
  label,
  disabled,
}: ConfigSelectorProps) {
  const htmlId = useId();
  const nodes = useFlowStore((s) => s.nodes);

  // Existing config nodes of this type
  const configOptions = useMemo(
    () => getConfigNodesByType(nodes, nodeType),
    [nodes, nodeType],
  );

  // Palette label for the config type (for "Add new ..." text)
  const configDef = nodeRegistry.getType(nodeType);
  const typeName = configDef?.paletteLabel ?? configDef?.name ?? nodeType;

  // Overlay state for adding / editing config nodes
  const [overlayOpen, setOverlayOpen] = useState(false);
  const [editingConfigId, setEditingConfigId] = useState<string | null>(null);

  const handleSelectChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const selected = e.target.value;
      if (selected === ADD_NEW_SENTINEL) {
        setEditingConfigId(null);
        setOverlayOpen(true);
      } else {
        onChange(selected);
      }
    },
    [onChange],
  );

  const handleEditClick = useCallback(() => {
    if (value) {
      setEditingConfigId(value);
      setOverlayOpen(true);
    }
  }, [value]);

  const handleOverlaySave = useCallback(
    (configId: string) => {
      onChange(configId);
      setOverlayOpen(false);
      setEditingConfigId(null);
    },
    [onChange],
  );

  const handleOverlayCancel = useCallback(() => {
    setOverlayOpen(false);
    setEditingConfigId(null);
  }, []);

  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label
          htmlFor={htmlId}
          className="text-xs font-medium text-gray-300"
        >
          {label}
        </label>
      )}

      <div className="flex items-stretch gap-0">
        <select
          id={htmlId}
          className={
            "flex-1 rounded-l border border-[#555] bg-[#444] px-2 py-1.5 " +
            "text-sm text-gray-100 outline-none transition-colors " +
            "focus:border-[#777] focus:ring-1 focus:ring-[#557da0] " +
            "disabled:opacity-50 disabled:cursor-not-allowed"
          }
          value={value}
          onChange={handleSelectChange}
          disabled={disabled}
          data-testid="config-selector"
        >
          <option value="">-- select --</option>
          {configOptions.map((opt) => (
            <option key={opt.id} value={opt.id}>
              {opt.label}
            </option>
          ))}
          <option value={ADD_NEW_SENTINEL}>Add new {typeName}...</option>
        </select>

        {/* Edit button */}
        <button
          type="button"
          className={
            "inline-flex items-center justify-center rounded-r border border-l-0 " +
            "border-[#555] bg-[#3a3a3a] px-2 text-gray-400 hover:text-gray-200 " +
            "transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          }
          onClick={handleEditClick}
          disabled={disabled || !value}
          aria-label={`Edit ${typeName} config`}
          title={`Edit selected ${typeName}`}
          data-testid="config-selector-edit-btn"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
            <path d="m15 5 4 4" />
          </svg>
        </button>
      </div>

      {/* Config editor overlay */}
      {overlayOpen && (
        <ConfigEditorOverlay
          configType={nodeType}
          configId={editingConfigId}
          onSave={handleOverlaySave}
          onCancel={handleOverlayCancel}
        />
      )}
    </div>
  );
}
