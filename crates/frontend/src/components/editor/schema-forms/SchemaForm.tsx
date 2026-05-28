import { useCallback } from "react";
import type { NodeDefault } from "@/red/nodes/types";
import { SchemaField } from "./SchemaField";

/**
 * Props for the SchemaForm component.
 */
export interface SchemaFormProps {
  /** Schema from node definition's `defaults` object */
  schema: Record<string, NodeDefault>;
  /** Current node property values */
  values: Record<string, unknown>;
  /** Called when a single property changes */
  onChange: (key: string, value: unknown) => void;
  /** Validation errors keyed by property name */
  errors?: Record<string, string>;
  /** Disable all fields */
  disabled?: boolean;
}

/**
 * Sort order for form fields. "name" always comes first, then alphabetical.
 */
function sortFields(entries: [string, NodeDefault][]): [string, NodeDefault][] {
  return entries.sort(([a], [b]) => {
    if (a === "name") return -1;
    if (b === "name") return 1;
    return a.localeCompare(b);
  });
}

/**
 * SchemaForm renders a node property editor form by iterating over a node
 * definition's `defaults` and delegating each entry to SchemaField, which
 * maps the type to the correct field component.
 *
 * Usage:
 * ```tsx
 * <SchemaForm
 *   schema={nodeDefinition.defaults}
 *   values={currentNodeProps}
 *   onChange={(key, val) => updateNode({ [key]: val })}
 * />
 * ```
 */
export function SchemaForm({
  schema,
  values,
  onChange,
  errors,
  disabled,
}: SchemaFormProps) {
  const handleChange = useCallback(
    (key: string) => (value: unknown) => {
      onChange(key, value);
    },
    [onChange],
  );

  const entries = sortFields(Object.entries(schema));

  return (
    <div className="flex flex-col gap-3">
      {entries.map(([key, fieldSchema]) => (
        <SchemaField
          key={key}
          name={key}
          schema={fieldSchema}
          value={values[key]}
          onChange={handleChange(key)}
          error={errors?.[key]}
          disabled={disabled}
        />
      ))}
    </div>
  );
}
