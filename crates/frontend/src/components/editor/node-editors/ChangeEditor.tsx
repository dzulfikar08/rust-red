/**
 * ChangeEditor -- Custom editor for Node-RED's "change" node.
 *
 * Provides a rule builder where each rule has:
 *   - Action type: set, change, delete, move
 *   - Target property (TypedInput: msg/flow/global)
 *   - For "set": value (TypedInput with full types) + deep copy checkbox
 *   - For "change": search value (TypedInput) + replace value (TypedInput)
 *   - For "move": destination (TypedInput: msg/flow/global)
 *   - For "delete": no additional fields
 *
 * Add/remove rule buttons with sortable list.
 */

import { useCallback } from "react";
import { TypedInput, type TypedInputType } from "../../common/TypedInput";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Change rule as stored in node properties */
export interface ChangeRule {
  /** Action type: set | change | delete | move */
  t: string;
  /** Target property name */
  p: string;
  /** Target property type */
  pt: string;
  /** Value (for set/move destination) */
  to?: string;
  /** Value type */
  tot?: string;
  /** Search value (for change) */
  from?: string;
  /** Search value type */
  fromt?: string;
  /** Deep copy flag (for set) */
  dc?: boolean;
}

type ActionType = "set" | "change" | "delete" | "move";

const ACTIONS: { value: ActionType; label: string }[] = [
  { value: "set", label: "Set" },
  { value: "change", label: "Change" },
  { value: "delete", label: "Delete" },
  { value: "move", label: "Move" },
];

const PROPERTY_TYPES: TypedInputType[] = ["msg", "flow", "global"];
const VALUE_TYPES: TypedInputType[] = [
  "msg", "flow", "global", "str", "num", "bool", "json", "bin", "env",
];
const SEARCH_TYPES: TypedInputType[] = [
  "msg", "flow", "global", "str", "num", "bool", "env",
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ChangeEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const rules = (values.rules as ChangeRule[]) ?? [
    { t: "set", p: "payload", pt: "msg", to: "", tot: "str" },
  ];

  const updateRule = useCallback(
    (index: number, updates: Partial<ChangeRule>) => {
      const newRules = [...rules];
      newRules[index] = { ...newRules[index], ...updates };
      onChange("rules", newRules);
    },
    [rules, onChange],
  );

  const addRule = useCallback(() => {
    const newRule: ChangeRule = { t: "set", p: "payload", pt: "msg", to: "", tot: "str" };
    onChange("rules", [...rules, newRule]);
  }, [rules, onChange]);

  const removeRule = useCallback(
    (index: number) => {
      onChange("rules", rules.filter((_, i) => i !== index));
    },
    [rules, onChange],
  );

  return (
    <div className="space-y-3" data-testid="change-editor">
      {/* Rules header */}
      <div className="text-xs font-semibold text-gray-600 dark:text-gray-300">
        Rules
      </div>

      {/* Rules list */}
      <div className="space-y-2" data-testid="change-rules-list">
        {rules.map((rule, index) => (
          <ChangeRuleRow
            key={index}
            rule={rule}
            index={index}
            onChange={updateRule}
            onRemove={removeRule}
          />
        ))}
      </div>

      {/* Add rule button */}
      <button
        type="button"
        className="px-3 py-1 text-xs rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
        onClick={addRule}
        data-testid="change-add-rule"
      >
        + Add rule
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ChangeRuleRow sub-component
// ---------------------------------------------------------------------------

interface ChangeRuleRowProps {
  rule: ChangeRule;
  index: number;
  onChange: (index: number, updates: Partial<ChangeRule>) => void;
  onRemove: (index: number) => void;
}

function ChangeRuleRow({ rule, index, onChange, onRemove }: ChangeRuleRowProps) {
  const action = rule.t as ActionType;
  const showTo = action === "set" || action === "move";
  const showFrom = action === "change";
  const showDeepCopy = action === "set";
  const showReplace = action === "change";

  const handleActionChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const newAction = e.target.value as ActionType;
      const updates: Partial<ChangeRule> = { t: newAction };

      // Set sensible defaults for value types
      if (newAction === "set") {
        updates.tot = "str";
        updates.to = "";
      } else if (newAction === "move") {
        updates.tot = "msg";
        updates.to = "";
      } else if (newAction === "change") {
        updates.fromt = "str";
        updates.from = "";
        updates.tot = "str";
        updates.to = "";
      }

      onChange(index, updates);
    },
    [index, onChange],
  );

  const handlePropertyChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange(index, { p: value, pt: type });
    },
    [index, onChange],
  );

  const handleToChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange(index, { to: value, tot: type });
    },
    [index, onChange],
  );

  const handleFromChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange(index, { from: value, fromt: type });
    },
    [index, onChange],
  );

  const handleDeepCopyChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(index, { dc: e.target.checked });
    },
    [index, onChange],
  );

  return (
    <div
      className="p-2 border border-gray-200 dark:border-gray-600 rounded bg-white dark:bg-gray-800 space-y-2"
      data-testid={`change-rule-${index}`}
    >
      {/* Row 1: Action + Property */}
      <div className="flex items-center gap-2">
        <select
          value={action}
          onChange={handleActionChange}
          className="border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 min-w-[80px]"
          data-testid={`change-rule-action-${index}`}
        >
          {ACTIONS.map((a) => (
            <option key={a.value} value={a.value}>
              {a.label}
            </option>
          ))}
        </select>

        <div className="flex-1">
          <TypedInput
            value={rule.p}
            type={(rule.pt as TypedInputType) ?? "msg"}
            onChange={handlePropertyChange}
            types={PROPERTY_TYPES}
          />
        </div>

        <button
          type="button"
          className="text-gray-400 hover:text-red-500 text-sm leading-none shrink-0"
          onClick={() => onRemove(index)}
          aria-label={`Remove rule ${index + 1}`}
          data-testid={`change-remove-rule-${index}`}
        >
          &times;
        </button>
      </div>

      {/* Row 2: "to" value (for set / move) */}
      {showTo && (
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500 w-[100px] text-right shrink-0">
            {action === "set" ? "to" : "to"}
          </span>
          <div className="flex-1">
            <TypedInput
              value={rule.to ?? ""}
              type={(rule.tot as TypedInputType) ?? (action === "move" ? "msg" : "str")}
              onChange={handleToChange}
              types={action === "move" ? PROPERTY_TYPES : VALUE_TYPES}
            />
          </div>
        </div>
      )}

      {/* Deep copy checkbox (for set) */}
      {showDeepCopy && (
        <div className="pl-[108px]">
          <label className="flex items-center gap-1 text-xs text-gray-600 dark:text-gray-300">
            <input
              type="checkbox"
              checked={rule.dc ?? false}
              onChange={handleDeepCopyChange}
              data-testid={`change-rule-dc-${index}`}
            />
            deep copy
          </label>
        </div>
      )}

      {/* Row 3: "search" value (for change) */}
      {showFrom && (
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500 w-[100px] text-right shrink-0">
            search
          </span>
          <div className="flex-1">
            <TypedInput
              value={rule.from ?? ""}
              type={(rule.fromt as TypedInputType) ?? "str"}
              onChange={handleFromChange}
              types={SEARCH_TYPES}
            />
          </div>
        </div>
      )}

      {/* Row 4: "replace" value (for change) */}
      {showReplace && (
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500 w-[100px] text-right shrink-0">
            replace
          </span>
          <div className="flex-1">
            <TypedInput
              value={rule.to ?? ""}
              type={(rule.tot as TypedInputType) ?? "str"}
              onChange={handleToChange}
              types={VALUE_TYPES}
            />
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("change", ChangeEditor);
