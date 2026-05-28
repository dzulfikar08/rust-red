/**
 * SwitchEditor -- Custom editor for Node-RED's "switch" node.
 *
 * Provides a rule builder where each rule has:
 *   - Condition operator (==, !=, <, <=, >, >=, contains, regex, etc.)
 *   - Value (TypedInput)
 *   - Optional second value for "between" operator
 *   - Case-sensitive checkbox for regex
 *
 * Also includes:
 *   - Property field (TypedInput for msg/flow/global)
 *   - "Check all" vs "Stop at first" radio
 *   - Repair checkbox
 *   - Otherwise output shown at bottom
 */

import { useState, useCallback } from "react";
import { TypedInput, type TypedInputType } from "../../common/TypedInput";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Switch rule as stored in node properties */
export interface SwitchRule {
  /** Operator type */
  t: string;
  /** Value (first) */
  v?: string;
  /** Value type (first) */
  vt?: string;
  /** Value 2 (for between) */
  v2?: string;
  /** Value 2 type */
  v2t?: string;
  /** Case-sensitive (for regex) */
  case?: boolean;
}

interface OperatorDef {
  v: string;
  t: string;
}

/** Operators grouped by kind */
const VALUE_OPERATORS: OperatorDef[] = [
  { v: "eq", t: "==" },
  { v: "neq", t: "!=" },
  { v: "lt", t: "<" },
  { v: "lte", t: "<=" },
  { v: "gt", t: ">" },
  { v: "gte", t: ">=" },
  { v: "hask", t: "has key" },
  { v: "btwn", t: "between" },
  { v: "cont", t: "contains" },
  { v: "regex", t: "regex" },
  { v: "true", t: "true" },
  { v: "false", t: "false" },
  { v: "null", t: "null" },
  { v: "nnull", t: "not null" },
  { v: "istype", t: "is of type" },
  { v: "empty", t: "empty" },
  { v: "nempty", t: "not empty" },
];

const SEQUENCE_OPERATORS: OperatorDef[] = [
  { v: "head", t: "head" },
  { v: "index", t: "index" },
  { v: "tail", t: "tail" },
];

const EXPRESSION_OPERATORS: OperatorDef[] = [
  { v: "jsonata_exp", t: "expression" },
  { v: "else", t: "otherwise" },
];

const ALL_OPERATORS = [...VALUE_OPERATORS, ...SEQUENCE_OPERATORS, ...EXPRESSION_OPERATORS];

/** Operators that do NOT need a value field */
const NO_VALUE_OPERATORS = new Set([
  "true", "false", "null", "nnull", "empty", "nempty", "else",
]);

/** Operators that need two value fields (between) */
const TWO_VALUE_OPERATORS = new Set(["btwn", "index"]);

/** Default TypedInput types for rule values */
const RULE_VALUE_TYPES: TypedInputType[] = ["msg", "flow", "global", "str", "num", "env"];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SwitchEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const property = (values.property as string) ?? "payload";
  const propertyType = (values.propertyType as string) ?? "msg";
  const rules = (values.rules as SwitchRule[]) ?? [{ t: "eq", v: "", vt: "str" }];
  const checkall = (values.checkall as string) ?? "true";
  const repair = (values.repair as boolean) ?? false;

  // ---- Handlers ----

  const handlePropertyChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange("property", value);
      onChange("propertyType", type);
    },
    [onChange],
  );

  const updateRule = useCallback(
    (index: number, updates: Partial<SwitchRule>) => {
      const newRules = [...rules];
      newRules[index] = { ...newRules[index], ...updates };
      onChange("rules", newRules);
      onChange("outputs", newRules.length + 1); // +1 for "otherwise"
    },
    [rules, onChange],
  );

  const addRule = useCallback(() => {
    const lastRule = rules[rules.length - 1];
    const newRule: SwitchRule = {
      t: "eq",
      v: "",
      vt: lastRule?.vt ?? "str",
    };
    const newRules = [...rules, newRule];
    onChange("rules", newRules);
    onChange("outputs", newRules.length + 1);
  }, [rules, onChange]);

  const removeRule = useCallback(
    (index: number) => {
      const newRules = rules.filter((_, i) => i !== index);
      onChange("rules", newRules);
      onChange("outputs", Math.max(newRules.length + 1, 1));
    },
    [rules, onChange],
  );

  const handleCheckallChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("checkall", e.target.value);
    },
    [onChange],
  );

  const handleRepairChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("repair", e.target.checked);
    },
    [onChange],
  );

  // ---- Render ----

  return (
    <div className="space-y-3" data-testid="switch-editor">
      {/* Property field */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Property
        </label>
        <TypedInput
          value={property}
          type={propertyType as TypedInputType}
          onChange={handlePropertyChange}
          types={["msg", "flow", "global", "env"]}
        />
      </div>

      {/* Rules header */}
      <div className="text-xs font-semibold text-gray-600 dark:text-gray-300">
        Rules
      </div>

      {/* Rules list */}
      <div className="space-y-2" data-testid="switch-rules-list">
        {rules.map((rule, index) => (
          <RuleRow
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
        data-testid="switch-add-rule"
      >
        + Add rule
      </button>

      {/* Otherwise output */}
      <div
        className="flex items-center gap-2 px-3 py-2 bg-gray-50 dark:bg-gray-750 rounded border border-gray-200 dark:border-gray-600"
        data-testid="switch-otherwise"
      >
        <span className="text-xs font-medium text-gray-500">otherwise</span>
        <span className="text-xs text-gray-400">
          &rarr; {rules.length + 1}
        </span>
      </div>

      {/* Check all / Stop at first */}
      <div className="flex items-center gap-4" data-testid="switch-checkall">
        <label className="flex items-center gap-1.5 text-xs text-gray-600 dark:text-gray-300">
          <input
            type="radio"
            name="switch-checkall"
            value="true"
            checked={checkall === "true"}
            onChange={handleCheckallChange}
            data-testid="switch-checkall-true"
          />
          checking all rules
        </label>
        <label className="flex items-center gap-1.5 text-xs text-gray-600 dark:text-gray-300">
          <input
            type="radio"
            name="switch-checkall"
            value="false"
            checked={checkall === "false"}
            onChange={handleCheckallChange}
            data-testid="switch-checkall-false"
          />
          stop at first match
        </label>
      </div>

      {/* Repair checkbox */}
      <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
        <input
          type="checkbox"
          checked={repair}
          onChange={handleRepairChange}
          data-testid="switch-repair"
        />
        Reconstruct message sequence
      </label>
    </div>
  );
}

// ---------------------------------------------------------------------------
// RuleRow sub-component
// ---------------------------------------------------------------------------

interface RuleRowProps {
  rule: SwitchRule;
  index: number;
  onChange: (index: number, updates: Partial<SwitchRule>) => void;
  onRemove: (index: number) => void;
}

function RuleRow({ rule, index, onChange, onRemove }: RuleRowProps) {
  const needsValue = !NO_VALUE_OPERATORS.has(rule.t);
  const needsTwoValues = TWO_VALUE_OPERATORS.has(rule.t);
  const isRegex = rule.t === "regex";

  const handleOperatorChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const newOp = e.target.value;
      // Reset value fields when switching operator
      const updates: Partial<SwitchRule> = { t: newOp };
      if (NO_VALUE_OPERATORS.has(newOp)) {
        updates.v = undefined;
        updates.vt = undefined;
        updates.v2 = undefined;
        updates.v2t = undefined;
      }
      onChange(index, updates);
    },
    [index, onChange],
  );

  const handleValueChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange(index, { v: value, vt: type });
    },
    [index, onChange],
  );

  const handleValue2Change = useCallback(
    (value: string, type: TypedInputType) => {
      onChange(index, { v2: value, v2t: type });
    },
    [index, onChange],
  );

  const handleCaseChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(index, { case: e.target.checked });
    },
    [index, onChange],
  );

  return (
    <div
      className="flex items-start gap-2 p-2 border border-gray-200 dark:border-gray-600 rounded bg-white dark:bg-gray-800"
      data-testid={`switch-rule-${index}`}
    >
      {/* Rule number */}
      <span className="text-xs text-gray-400 pt-1 shrink-0">{index + 1}</span>

      <div className="flex-1 space-y-1.5">
        {/* Operator + value row */}
        <div className="flex items-center gap-2">
          <select
            value={rule.t}
            onChange={handleOperatorChange}
            className="border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 min-w-[100px]"
            data-testid={`switch-rule-op-${index}`}
          >
            <optgroup label="Value rules">
              {VALUE_OPERATORS.map((op) => (
                <option key={op.v} value={op.v}>
                  {op.t}
                </option>
              ))}
            </optgroup>
            <optgroup label="Sequence rules">
              {SEQUENCE_OPERATORS.map((op) => (
                <option key={op.v} value={op.v}>
                  {op.t}
                </option>
              ))}
            </optgroup>
            {EXPRESSION_OPERATORS.map((op) => (
              <option key={op.v} value={op.v}>
                {op.t}
              </option>
            ))}
          </select>

          {/* Value field (when applicable) */}
          {needsValue && (
            <div className="flex-1">
              <TypedInput
                value={rule.v ?? ""}
                type={(rule.vt as TypedInputType) ?? "str"}
                onChange={handleValueChange}
                types={RULE_VALUE_TYPES}
              />
            </div>
          )}
        </div>

        {/* Second value (between) */}
        {needsTwoValues && (
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-400">and</span>
            <div className="flex-1">
              <TypedInput
                value={rule.v2 ?? ""}
                type={(rule.v2t as TypedInputType) ?? "num"}
                onChange={handleValue2Change}
                types={RULE_VALUE_TYPES}
              />
            </div>
          </div>
        )}

        {/* Case-sensitive checkbox (regex) */}
        {isRegex && (
          <label className="flex items-center gap-1 text-xs text-gray-600 dark:text-gray-300">
            <input
              type="checkbox"
              checked={rule.case ?? false}
              onChange={handleCaseChange}
              data-testid={`switch-rule-case-${index}`}
            />
            ignore case
          </label>
        )}
      </div>

      {/* Output indicator */}
      <span className="text-xs text-gray-400 pt-1 shrink-0">
        &rarr; {index + 1}
      </span>

      {/* Remove button */}
      <button
        type="button"
        className="text-gray-400 hover:text-red-500 text-sm leading-none shrink-0"
        onClick={() => onRemove(index)}
        aria-label={`Remove rule ${index + 1}`}
        data-testid={`switch-remove-rule-${index}`}
      >
        &times;
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("switch", SwitchEditor);
