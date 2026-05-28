/**
 * EnvVarEditor -- Inline table editor for environment variables.
 *
 * Columns: Name, Type (dropdown), Value, Actions (delete).
 * Features: add row, inline editing, duplicate-name validation.
 */

import { useCallback, useState } from "react";
import { useEnvVarStore, type EnvVarType } from "../../store/env-var-store";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TYPES: EnvVarType[] = ["str", "num", "bool"];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function EnvVarEditor() {
  const vars = useEnvVarStore((s) => s.vars);
  const addVar = useEnvVarStore((s) => s.addVar);
  const removeVar = useEnvVarStore((s) => s.removeVar);
  const updateVar = useEnvVarStore((s) => s.updateVar);

  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState<EnvVarType>("str");
  const [newValue, setNewValue] = useState("");
  const [dupError, setDupError] = useState<string | null>(null);

  // ----- Determine whether the "add" name is a duplicate -----

  const isDuplicate = useCallback(
    (name: string) => vars.some((v) => v.name === name),
    [vars],
  );

  // ----- Handlers -----

  const handleAdd = useCallback(() => {
    const trimmed = newName.trim();
    if (!trimmed) return;
    if (isDuplicate(trimmed)) {
      setDupError(`"${trimmed}" already exists`);
      return;
    }
    addVar(trimmed, newValue, newType);
    setNewName("");
    setNewValue("");
    setNewType("str");
    setDupError(null);
  }, [newName, newValue, newType, addVar, isDuplicate]);

  const handleNameKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  const handleRename = useCallback(
    (oldName: string, newNameRaw: string) => {
      const trimmed = newNameRaw.trim();
      if (!trimmed || trimmed === oldName) return;
      if (vars.some((v) => v.name === trimmed)) return;
      // Remove old, add with new name (preserving type & value)
      const existing = vars.find((v) => v.name === oldName);
      if (!existing) return;
      removeVar(oldName);
      addVar(trimmed, existing.value, existing.type);
    },
    [vars, removeVar, addVar],
  );

  // ----- Render -----

  return (
    <div data-testid="env-var-editor">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-semibold text-gray-600 dark:text-gray-300">
          Environment Variables
        </span>
      </div>

      {/* Table */}
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr className="border-b border-gray-200 dark:border-gray-700 text-xs text-gray-500 dark:text-gray-400">
            <th className="text-left py-1 pr-2 font-medium">Name</th>
            <th className="text-left py-1 pr-2 font-medium w-20">Type</th>
            <th className="text-left py-1 pr-2 font-medium">Value</th>
            <th className="w-8" />
          </tr>
        </thead>
        <tbody>
          {vars.map((v) => (
            <VarRow
              key={v.name}
              varItem={v}
              onRemove={removeVar}
              onUpdate={updateVar}
              onRename={handleRename}
            />
          ))}
          {vars.length === 0 && (
            <tr>
              <td
                colSpan={4}
                className="py-3 text-center text-xs text-gray-400 dark:text-gray-500"
              >
                No environment variables defined
              </td>
            </tr>
          )}
        </tbody>
      </table>

      {/* Add row */}
      <div className="flex items-center gap-2 mt-2">
        <input
          type="text"
          placeholder="VAR_NAME"
          className="flex-1 min-w-0 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
          value={newName}
          onChange={(e) => {
            setNewName(e.target.value);
            setDupError(null);
          }}
          onKeyDown={handleNameKeyDown}
          data-testid="env-var-add-name"
          aria-label="New variable name"
        />
        <select
          className="border border-gray-300 dark:border-gray-600 rounded px-1 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
          value={newType}
          onChange={(e) => setNewType(e.target.value as EnvVarType)}
          data-testid="env-var-add-type"
          aria-label="New variable type"
        >
          {TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <input
          type="text"
          placeholder="value"
          className="flex-1 min-w-0 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          onKeyDown={handleNameKeyDown}
          data-testid="env-var-add-value"
          aria-label="New variable value"
        />
        <button
          type="button"
          className="px-2 py-1 text-xs rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-40"
          onClick={handleAdd}
          disabled={!newName.trim()}
          data-testid="env-var-add-btn"
        >
          Add
        </button>
      </div>

      {/* Duplicate name error */}
      {dupError && (
        <span
          className="text-xs text-red-500 mt-1 block"
          data-testid="env-var-dup-error"
        >
          {dupError}
        </span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// VarRow -- one table row for an existing variable
// ---------------------------------------------------------------------------

interface VarRowProps {
  varItem: {
    name: string;
    value: string;
    type: EnvVarType;
  };
  onRemove: (name: string) => void;
  onUpdate: (name: string, updates: Partial<{ value: string; type: EnvVarType }>) => void;
  onRename: (oldName: string, newName: string) => void;
}

function VarRow({ varItem, onRemove, onUpdate, onRename }: VarRowProps) {
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState(varItem.name);

  const commitName = useCallback(() => {
    setEditingName(false);
    onRename(varItem.name, nameDraft);
  }, [varItem.name, nameDraft, onRename]);

  return (
    <tr
      className="border-b border-gray-100 dark:border-gray-700/50"
      data-testid={`env-var-row-${varItem.name}`}
    >
      {/* Name */}
      <td className="py-1 pr-2">
        {editingName ? (
          <input
            type="text"
            className="w-full border border-gray-300 dark:border-gray-600 rounded px-1 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={nameDraft}
            onChange={(e) => setNameDraft(e.target.value)}
            onBlur={commitName}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitName();
              if (e.key === "Escape") {
                setNameDraft(varItem.name);
                setEditingName(false);
              }
            }}
            data-testid={`env-var-name-input-${varItem.name}`}
            autoFocus
          />
        ) : (
          <span
            className="cursor-pointer hover:text-blue-600 dark:hover:text-blue-400 text-xs font-mono"
            onDoubleClick={() => {
              setNameDraft(varItem.name);
              setEditingName(true);
            }}
            title="Double-click to rename"
            data-testid={`env-var-name-${varItem.name}`}
          >
            {varItem.name}
          </span>
        )}
      </td>

      {/* Type */}
      <td className="py-1 pr-2">
        <select
          className="border border-gray-300 dark:border-gray-600 rounded px-1 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
          value={varItem.type}
          onChange={(e) => onUpdate(varItem.name, { type: e.target.value as EnvVarType })}
          data-testid={`env-var-type-${varItem.name}`}
          aria-label={`Type for ${varItem.name}`}
        >
          {TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </td>

      {/* Value */}
      <td className="py-1 pr-2">
        <input
          type="text"
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-1 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
          value={varItem.value}
          onChange={(e) => onUpdate(varItem.name, { value: e.target.value })}
          data-testid={`env-var-value-${varItem.name}`}
          aria-label={`Value for ${varItem.name}`}
        />
      </td>

      {/* Delete */}
      <td className="py-1">
        <button
          type="button"
          className="text-gray-400 hover:text-red-500 text-xs leading-none"
          onClick={() => onRemove(varItem.name)}
          data-testid={`env-var-delete-${varItem.name}`}
          aria-label={`Delete ${varItem.name}`}
        >
          &times;
        </button>
      </td>
    </tr>
  );
}
