import { useMemo } from "react";
import type { NodeDefault } from "@/red/nodes/types";
import { nodeRegistry } from "@/red/nodes/registry";
import { TextField } from "./fields/TextField";
import { NumberField } from "./fields/NumberField";
import { BooleanField } from "./fields/BooleanField";
import { TypedInputField } from "./fields/TypedInputField";
import { PasswordField } from "./fields/PasswordField";
import { ConfigSelector } from "../ConfigSelector";

/**
 * Props for a single schema-driven field rendered inside SchemaForm.
 */
export interface SchemaFieldProps {
  /** Property name (key from defaults) */
  name: string;
  /** The schema definition for this property */
  schema: NodeDefault;
  /** Current property value */
  value: unknown;
  /** Callback when value changes */
  onChange: (value: unknown) => void;
  /** Validation error for this field */
  error?: string;
  /** Disable the field */
  disabled?: boolean;
}

/**
 * Humanise a camelCase / snake_case key into a display label.
 * e.g. "onceDelay" -> "Once Delay", "topic" -> "Topic"
 */
function humanise(key: string): string {
  return key
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/[_-]+/g, " ")
    .replace(/^\w/, (c) => c.toUpperCase());
}

/**
 * Determines whether a key indicates a password field.
 */
function isPasswordField(key: string): boolean {
  const lower = key.toLowerCase();
  return lower.endsWith("password") || lower.endsWith("pass");
}

/**
 * Determines whether a key indicates a multiline (code/script) field.
 */
function isCodeField(key: string): boolean {
  const lower = key.toLowerCase();
  return (
    lower.includes("code") ||
    lower.includes("script") ||
    lower.includes("template") ||
    lower.includes("func")
  );
}

/**
 * Resolves the field type from the schema and key, then renders the
 * appropriate field component.
 */
export function SchemaField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: SchemaFieldProps) {
  const label = humanise(name);
  const required = schema.required ?? false;
  const fieldDisabled = disabled ?? false;

  // Check whether schema.type refers to a registered config node type
  const isConfigRef = useMemo(() => {
    if (!schema.type) return false;
    // A type is considered a config reference if it is registered in the
    // node registry as a node definition. Primitive TypedInput types like
    // "str", "num", "msg", "jsonata", "env" are NOT config refs.
    return nodeRegistry.getType(schema.type) !== undefined;
  }, [schema.type]);

  const sharedProps = {
    name,
    label,
    value,
    onChange,
    error,
    disabled: fieldDisabled,
    required,
    schema,
  };

  // 1. Password fields (based on key name)
  if (isPasswordField(name)) {
    return <PasswordField {...sharedProps} />;
  }

  // 2. Config node reference fields (schema.type references a config node type)
  if (isConfigRef) {
    return (
      <ConfigSelector
        nodeType={schema.type!}
        value={String(value ?? "")}
        onChange={(id) => onChange(id)}
        label={label}
        disabled={fieldDisabled}
      />
    );
  }

  // 3. TypedInput fields (schema has a `type` property but NOT a config ref)
  if (schema.type) {
    return <TypedInputField {...sharedProps} />;
  }

  // 4. Boolean fields
  if (typeof schema.value === "boolean") {
    return <BooleanField {...sharedProps} />;
  }

  // 5. Number fields
  if (
    typeof schema.value === "number" ||
    (typeof schema.value === "string" && schema.value === "num")
  ) {
    return <NumberField {...sharedProps} />;
  }

  // 6. Text fields – multiline for code/script keys
  if (typeof schema.value === "string" || schema.value === "") {
    return <TextField {...sharedProps} multiline={isCodeField(name)} />;
  }

  // 7. Fallback: text input
  return <TextField {...sharedProps} />;
}
