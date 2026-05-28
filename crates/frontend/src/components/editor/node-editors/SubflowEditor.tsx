/**
 * SubflowEditor -- Custom editor for subflow node properties.
 *
 * Provides fields for:
 *   - Name, category, color, icon, description
 *   - Input/output port count
 *   - Environment variable definitions
 *
 * Registered as the custom editor for the "subflow" node type.
 */

import { useState, useCallback } from "react";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";
import type { SubflowEnvVar } from "../../../store/subflow-store";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CATEGORIES = [
  "common",
  "function",
  "network",
  "sequence",
  "parser",
  "storage",
] as const;

const COLORS = [
  "#da9aaa",
  "#a6bbcf",
  "#bfaaaf",
  "#c0c0c0",
  "#ad98d3",
  "#87a780",
  "#ffffa0",
  "#e6e0f8",
  "#e2d96e",
  "#d8d8d8",
] as const;

const ICONS = [
  "debugger",
  "function",
  "arrow-right",
  "random",
  "swap",
  "sort-amount-down",
  "feed",
  "database",
  "file-code-o",
  "cog",
] as const;

const ENV_VAR_TYPES: SubflowEnvVar["type"][] = [
  "str",
  "num",
  "bool",
  "json",
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SubflowEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const name = (values.name as string) ?? "";
  const category = (values.category as string) ?? "function";
  const color = (values.color as string) ?? "#da9aaa";
  const icon = (values.icon as string) ?? "debugger";
  const description = (values.description as string) ?? "";
  const inCount = (values.in as number) ?? 1;
  const outCount = (values.out as number) ?? 1;
  const env = (values.env as SubflowEnvVar[]) ?? [];

  // Track whether we are showing the env editor panel
  const [showEnv, setShowEnv] = useState(false);

  // -- Handlers --

  const handleNameChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) =>
      onChange("name", e.target.value),
    [onChange],
  );

  const handleCategoryChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) =>
      onChange("category", e.target.value),
    [onChange],
  );

  const handleColorChange = useCallback(
    (c: string) => onChange("color", c),
    [onChange],
  );

  const handleIconChange = useCallback(
    (ic: string) => onChange("icon", ic),
    [onChange],
  );

  const handleDescriptionChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) =>
      onChange("description", e.target.value),
    [onChange],
  );

  const handleInChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.max(0, Math.min(20, parseInt(e.target.value, 10) || 0));
      onChange("in", v);
    },
    [onChange],
  );

  const handleOutChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.max(0, Math.min(20, parseInt(e.target.value, 10) || 0));
      onChange("out", v);
    },
    [onChange],
  );

  // -- Env var handlers --

  const addEnvVar = useCallback(() => {
    const next = [...env, { name: "", value: "", type: "str" as const }];
    onChange("env", next);
  }, [env, onChange]);

  const removeEnvVar = useCallback(
    (index: number) => {
      const next = env.filter((_, i) => i !== index);
      onChange("env", next);
    },
    [env, onChange],
  );

  const updateEnvVar = useCallback(
    (index: number, updates: Partial<SubflowEnvVar>) => {
      const next = env.map((v, i) => (i === index ? { ...v, ...updates } : v));
      onChange("env", next);
    },
    [env, onChange],
  );

  // -- Render --

  return (
    <div className="space-y-4" data-testid="subflow-editor">
      {/* Name */}
      <div>
        <label
          className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
          htmlFor="subflow-name"
        >
          Name
        </label>
        <input
          id="subflow-name"
          type="text"
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          value={name}
          onChange={handleNameChange}
          data-testid="subflow-name"
        />
      </div>

      {/* Category */}
      <div>
        <label
          className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
          htmlFor="subflow-category"
        >
          Category
        </label>
        <select
          id="subflow-category"
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          value={category}
          onChange={handleCategoryChange}
          data-testid="subflow-category"
        >
          {CATEGORIES.map((cat) => (
            <option key={cat} value={cat}>
              {cat.charAt(0).toUpperCase() + cat.slice(1)}
            </option>
          ))}
        </select>
      </div>

      {/* Color swatches */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Color
        </label>
        <div className="flex flex-wrap gap-1.5" data-testid="subflow-colors">
          {COLORS.map((c) => (
            <button
              key={c}
              type="button"
              className={`w-6 h-6 rounded border-2 transition-all ${
                color === c
                  ? "border-gray-900 dark:border-white scale-110"
                  : "border-transparent hover:border-gray-400"
              }`}
              style={{ backgroundColor: c }}
              onClick={() => handleColorChange(c)}
              data-testid={`subflow-color-${c.replace("#", "")}`}
              aria-label={`Color ${c}`}
            />
          ))}
        </div>
      </div>

      {/* Icon */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Icon
        </label>
        <div className="flex flex-wrap gap-1.5" data-testid="subflow-icons">
          {ICONS.map((ic) => (
            <button
              key={ic}
              type="button"
              className={`px-2 py-1 text-xs rounded border transition-all ${
                icon === ic
                  ? "border-blue-500 bg-blue-50 dark:bg-blue-900 text-blue-600 dark:text-blue-300"
                  : "border-gray-300 dark:border-gray-600 hover:border-gray-400"
              }`}
              onClick={() => handleIconChange(ic)}
              data-testid={`subflow-icon-${ic}`}
            >
              {ic}
            </button>
          ))}
        </div>
      </div>

      {/* Description */}
      <div>
        <label
          className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
          htmlFor="subflow-description"
        >
          Description
        </label>
        <textarea
          id="subflow-description"
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 min-h-[60px]"
          value={description}
          onChange={handleDescriptionChange}
          data-testid="subflow-description"
        />
      </div>

      {/* Inputs / Outputs */}
      <div className="flex items-center gap-4">
        <div>
          <label
            className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
            htmlFor="subflow-in"
          >
            Inputs
          </label>
          <input
            id="subflow-in"
            type="number"
            min={0}
            max={20}
            className="w-20 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            value={inCount}
            onChange={handleInChange}
            data-testid="subflow-in"
          />
        </div>
        <div>
          <label
            className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
            htmlFor="subflow-out"
          >
            Outputs
          </label>
          <input
            id="subflow-out"
            type="number"
            min={0}
            max={20}
            className="w-20 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            value={outCount}
            onChange={handleOutChange}
            data-testid="subflow-out"
          />
        </div>
      </div>

      {/* Environment Variables */}
      <div>
        <button
          type="button"
          className="flex items-center gap-1 text-xs font-semibold text-gray-600 dark:text-gray-300 hover:text-gray-800 dark:hover:text-gray-100"
          onClick={() => setShowEnv(!showEnv)}
          data-testid="subflow-env-toggle"
        >
          <span
            className={`transform transition-transform text-[10px] ${
              showEnv ? "rotate-90" : ""
            }`}
          >
            ▶
          </span>
          Environment Variables ({env.length})
        </button>

        {showEnv && (
          <div className="mt-2 space-y-2" data-testid="subflow-env-panel">
            {env.map((v, i) => (
              <div
                key={i}
                className="flex items-center gap-2"
                data-testid={`subflow-env-row-${i}`}
              >
                <input
                  type="text"
                  placeholder="name"
                  className="w-24 border border-gray-300 dark:border-gray-600 rounded px-1.5 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  value={v.name}
                  onChange={(e) =>
                    updateEnvVar(i, { name: e.target.value })
                  }
                  data-testid={`subflow-env-name-${i}`}
                />
                <input
                  type="text"
                  placeholder="value"
                  className="flex-1 border border-gray-300 dark:border-gray-600 rounded px-1.5 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  value={v.value}
                  onChange={(e) =>
                    updateEnvVar(i, { value: e.target.value })
                  }
                  data-testid={`subflow-env-value-${i}`}
                />
                <select
                  className="border border-gray-300 dark:border-gray-600 rounded px-1 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  value={v.type}
                  onChange={(e) =>
                    updateEnvVar(i, {
                      type: e.target.value as SubflowEnvVar["type"],
                    })
                  }
                  data-testid={`subflow-env-type-${i}`}
                >
                  {ENV_VAR_TYPES.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="text-red-500 hover:text-red-700 text-xs px-1"
                  onClick={() => removeEnvVar(i)}
                  data-testid={`subflow-env-remove-${i}`}
                >
                  ✕
                </button>
              </div>
            ))}
            <button
              type="button"
              className="text-xs text-blue-500 hover:text-blue-700"
              onClick={addEnvVar}
              data-testid="subflow-env-add"
            >
              + Add variable
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("subflow", SubflowEditor);
