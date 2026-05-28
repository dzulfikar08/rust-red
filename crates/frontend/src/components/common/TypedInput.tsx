/**
 * TypedInput Component
 *
 * A React port of Node-RED's TypedInput widget (~1640 lines in typedInput.js).
 * This is the most complex and widely-used widget in Node-RED, appearing in
 * nearly every node editor. It provides a smart input field that switches
 * between value types (msg, flow, global, str, num, bool, json, bin, env, node).
 *
 * Structure:
 *   [Type Selector (left)] [Value Input (right)]
 *   Both parts appear as a single combined field.
 *
 * Type behaviors:
 *   msg/flow/global  - prefix display + autocomplete for context paths
 *   str              - plain text input
 *   num              - number input with validation
 *   bool             - toggle between true/false (dropdown, not free text)
 *   json             - JSON input with validation
 *   bin              - binary/buffer input
 *   env              - environment variable input
 *   node             - node reference dropdown
 */

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { clsx } from "clsx";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TypedInputType =
  | "msg"
  | "flow"
  | "global"
  | "str"
  | "num"
  | "bool"
  | "json"
  | "bin"
  | "env"
  | "node";

export interface TypedInputProps {
  /** Current value */
  value: string;
  /** Current type */
  type: TypedInputType;
  /** Called when value or type changes */
  onChange: (value: string, type: TypedInputType) => void;
  /** Available types (default: all) */
  types?: TypedInputType[];
  /** Label shown above the field */
  label?: string;
  /** Placeholder for the value input */
  placeholder?: string;
  /** Disable the entire field */
  disabled?: boolean;
  /** Additional CSS class */
  className?: string;
}

// ---------------------------------------------------------------------------
// Type metadata
// ---------------------------------------------------------------------------

interface TypeMeta {
  label: string;
  prefix?: string;
  hasTextInput: boolean;
  inputType?: string;
  /** For types with fixed options (bool), displayed as dropdown */
  options?: { value: string; label: string }[];
  validate?: (v: string) => string | true;
}

const TYPE_META: Record<TypedInputType, TypeMeta> = {
  msg: {
    label: "msg.",
    prefix: "msg.",
    hasTextInput: true,
  },
  flow: {
    label: "flow.",
    prefix: "flow.",
    hasTextInput: true,
  },
  global: {
    label: "global.",
    prefix: "global.",
    hasTextInput: true,
  },
  str: {
    label: "string",
    hasTextInput: true,
  },
  num: {
    label: "number",
    hasTextInput: true,
    inputType: "text",
    validate: (v: string): string | true => {
      if (v === "") return true; // empty is allowed
      return !isNaN(Number(v)) || "Must be a valid number";
    },
  },
  bool: {
    label: "boolean",
    hasTextInput: false,
    options: [
      { value: "true", label: "true" },
      { value: "false", label: "false" },
    ],
  },
  json: {
    label: "JSON",
    hasTextInput: true,
    validate: (v: string): string | true => {
      if (v === "") return true;
      try {
        JSON.parse(v);
        return true;
      } catch {
        return "Invalid JSON";
      }
    },
  },
  bin: {
    label: "buffer",
    hasTextInput: true,
  },
  env: {
    label: "env.",
    prefix: "env.",
    hasTextInput: true,
  },
  node: {
    label: "node",
    hasTextInput: true,
  },
};

const ALL_TYPES: TypedInputType[] = [
  "msg",
  "flow",
  "global",
  "str",
  "num",
  "bool",
  "json",
  "bin",
  "env",
  "node",
];

// ---------------------------------------------------------------------------
// Validation helper
// ---------------------------------------------------------------------------

function validateValue(
  type: TypedInputType,
  value: string
): { valid: boolean; error?: string } {
  const validator = TYPE_META[type]?.validate;
  if (!validator) return { valid: true };
  const result = validator(value);
  if (result === true) return { valid: true };
  return { valid: false, error: result };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TypedInput({
  value,
  type,
  onChange,
  types,
  label,
  placeholder,
  disabled = false,
  className,
}: TypedInputProps) {
  const availableTypes = useMemo(() => types ?? ALL_TYPES, [types]);

  // ---- State ----
  const [menuOpen, setMenuOpen] = useState(false);
  const [optionMenuOpen, setOptionMenuOpen] = useState(false);
  const [focused, setFocused] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const optionMenuRef = useRef<HTMLDivElement>(null);

  const meta = TYPE_META[type];
  const validation = validateValue(type, value);

  // ---- Remember values per type when switching ----
  const oldValuesRef = useRef<Record<string, string>>({});

  // ---- Close menus on outside click ----
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        menuRef.current &&
        !menuRef.current.contains(e.target as Node) &&
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setMenuOpen(false);
      }
      if (
        optionMenuRef.current &&
        !optionMenuRef.current.contains(e.target as Node) &&
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOptionMenuOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // ---- Handlers ----
  const handleTypeChange = useCallback(
    (newType: TypedInputType) => {
      // Save current value for the old type
      oldValuesRef.current[type] = value;
      setMenuOpen(false);

      const newMeta = TYPE_META[newType];
      let newValue: string;

      // Restore previously stored value for the new type, or use sensible default
      if (oldValuesRef.current[newType] !== undefined) {
        newValue = oldValuesRef.current[newType];
      } else if (newMeta.options && !newMeta.hasTextInput) {
        // Option-only types: pick the first option
        newValue = newMeta.options[0].value;
      } else {
        newValue = "";
      }

      onChange(newValue, newType);
    },
    [type, value, onChange]
  );

  const handleValueChange = useCallback(
    (newValue: string) => {
      onChange(newValue, type);
    },
    [type, onChange]
  );

  const handleInputBlur = useCallback(() => {
    setFocused(false);
  }, []);

  // ---- Render: type selector dropdown ----
  const showTypeSelector = availableTypes.length > 1;

  return (
    <div
      className={clsx(
        // Outer wrapper for label + field
        "flex flex-col gap-1",
        className
      )}
    >
      {label && (
        <label className="text-xs font-medium text-gray-600">{label}</label>
      )}

      <div
        ref={containerRef}
        className={clsx(
          // Combined field container - looks like a single input
          "flex items-stretch",
          "h-9 rounded border",
          "text-sm",
          "transition-colors",
          focused
            ? "border-blue-400 ring-1 ring-blue-200"
            : "border-gray-300",
          !validation.valid && "border-red-400",
          disabled && "opacity-50 cursor-not-allowed bg-gray-50"
        )}
        data-testid="typedinput-container"
      >
        {/* ---- Type Selector Button ---- */}
        {showTypeSelector && (
          <button
            type="button"
            className={clsx(
              "flex items-center gap-1 px-2 shrink-0",
              "bg-gray-100 border-r border-gray-300",
              "hover:bg-gray-200 active:bg-gray-300",
              "text-xs text-gray-700 font-medium select-none",
              "rounded-l-[calc(theme(borderRadius.DEFAULT)-1px)]",
              "transition-colors",
              disabled && "pointer-events-none"
            )}
            onClick={() => {
              if (!disabled) {
                setMenuOpen((prev) => !prev);
                setOptionMenuOpen(false);
              }
            }}
            title={meta.label}
            aria-label={`Type: ${meta.label}`}
            aria-haspopup="listbox"
            aria-expanded={menuOpen}
            data-testid="typedinput-type-button"
          >
            <span>{meta.label}</span>
            <svg
              className="w-3 h-3 text-gray-500"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            >
              <path d="M3 5l3 3 3-3" />
            </svg>
          </button>
        )}

        {/* ---- Single-type label (when only one type available) ---- */}
        {!showTypeSelector && (
          <div
            className={clsx(
              "flex items-center px-2 shrink-0",
              "bg-gray-100 border-r border-gray-300",
              "text-xs text-gray-700 font-medium select-none",
              "rounded-l-[calc(theme(borderRadius.DEFAULT)-1px)]"
            )}
            data-testid="typedinput-single-type-label"
          >
            {meta.label}
          </div>
        )}

        {/* ---- Bool: dropdown selector ---- */}
        {type === "bool" && (
          <div className="relative flex-1">
            <button
              type="button"
              className={clsx(
                "w-full h-full px-3 text-left text-sm",
                "hover:bg-gray-50",
                "rounded-r-[calc(theme(borderRadius.DEFAULT)-1px)]",
                disabled && "pointer-events-none"
              )}
              onClick={() => {
                if (!disabled) {
                  setOptionMenuOpen((prev) => !prev);
                  setMenuOpen(false);
                }
              }}
              aria-label="Boolean value"
              aria-haspopup="listbox"
              aria-expanded={optionMenuOpen}
              data-testid="typedinput-bool-select"
            >
              {value || "true"}
            </button>

            {/* Bool options dropdown */}
            {optionMenuOpen && (
              <div
                ref={optionMenuRef}
                className={clsx(
                  "absolute left-0 top-full mt-1 z-50",
                  "bg-white border border-gray-200 rounded shadow-lg",
                  "min-w-[120px] py-1"
                )}
                role="listbox"
                data-testid="typedinput-bool-menu"
              >
                {TYPE_META.bool.options?.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    role="option"
                    aria-selected={value === opt.value}
                    className={clsx(
                      "w-full px-3 py-1.5 text-left text-sm hover:bg-gray-100",
                      value === opt.value && "bg-blue-50 text-blue-700 font-medium"
                    )}
                    onClick={() => {
                      handleValueChange(opt.value);
                      setOptionMenuOpen(false);
                    }}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ---- Text input for all other types ---- */}
        {type !== "bool" && meta.hasTextInput && (
          <div className="flex items-center flex-1 min-w-0">
            {/* Prefix display for msg/flow/global/env */}
            {meta.prefix && (
              <span
                className="shrink-0 px-1.5 text-xs text-gray-400 select-none font-mono"
                data-testid="typedinput-prefix"
              >
                {meta.prefix}
              </span>
            )}

            <input
              type={meta.inputType ?? "text"}
              value={value}
              onChange={(e) => handleValueChange(e.target.value)}
              onFocus={() => setFocused(true)}
              onBlur={handleInputBlur}
              disabled={disabled}
              placeholder={placeholder}
              className={clsx(
                "w-full h-full px-2 bg-transparent text-sm outline-none",
                "rounded-r-[calc(theme(borderRadius.DEFAULT)-1px)]",
                !showTypeSelector && !meta.prefix && "rounded-l-none",
                disabled && "cursor-not-allowed"
              )}
              aria-label={`Value input for ${meta.label}`}
              data-testid="typedinput-value-input"
            />
          </div>
        )}

        {/* ---- Validation error indicator ---- */}
        {!validation.valid && (
          <span
            className="shrink-0 pr-2 text-red-500 text-xs"
            title={validation.error}
            data-testid="typedinput-error"
          >
            !
          </span>
        )}
      </div>

      {/* ---- Type selector dropdown menu ---- */}
      {menuOpen && (
        <div
          ref={menuRef}
          className={clsx(
            "absolute z-50 mt-1",
            "bg-white border border-gray-200 rounded shadow-lg",
            "min-w-[140px] py-1",
            "max-h-64 overflow-y-auto"
          )}
          style={{
            // Position below the type button
            top: containerRef.current
              ? containerRef.current.getBoundingClientRect().bottom + 4
              : undefined,
            left: containerRef.current
              ? containerRef.current.getBoundingClientRect().left
              : undefined,
          }}
          role="listbox"
          data-testid="typedinput-type-menu"
        >
          {availableTypes.map((t) => {
            const tMeta = TYPE_META[t];
            return (
              <button
                key={t}
                type="button"
                role="option"
                aria-selected={type === t}
                className={clsx(
                  "w-full px-3 py-1.5 text-left text-sm hover:bg-gray-100",
                  "flex items-center gap-2",
                  type === t && "bg-blue-50 text-blue-700 font-medium"
                )}
                onClick={() => handleTypeChange(t)}
              >
                <span className="flex-1">{tMeta.label}</span>
                {type === t && (
                  <svg
                    className="w-3.5 h-3.5 text-blue-600"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                  >
                    <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z" />
                  </svg>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/**
 * Validate a value for a given TypedInput type.
 * Returns `{ valid: true }` or `{ valid: false, error: string }`.
 */
export { validateValue };

export default TypedInput;
