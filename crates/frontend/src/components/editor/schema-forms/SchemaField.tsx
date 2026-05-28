import type { NodeDefault } from "@/red/nodes/types";
import { TextField } from "./fields/TextField";
import { NumberField } from "./fields/NumberField";
import { BooleanField } from "./fields/BooleanField";
import { TypedInputField } from "./fields/TypedInputField";
import { PasswordField } from "./fields/PasswordField";

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

  // 2. TypedInput fields (schema has a `type` property)
  if (schema.type) {
    return <TypedInputField {...sharedProps} />;
  }

  // 3. Boolean fields
  if (typeof schema.value === "boolean") {
    return <BooleanField {...sharedProps} />;
  }

  // 4. Number fields
  if (
    typeof schema.value === "number" ||
    (typeof schema.value === "string" && schema.value === "num")
  ) {
    return <NumberField {...sharedProps} />;
  }

  // 5. Text fields – multiline for code/script keys
  if (typeof schema.value === "string" || schema.value === "") {
    return <TextField {...sharedProps} multiline={isCodeField(name)} />;
  }

  // 6. Fallback: text input
  return <TextField {...sharedProps} />;
}
