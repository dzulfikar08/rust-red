/**
 * InjectEditor
 *
 * Custom editor for the inject node.  Uses SchemaForm with the inject
 * defaults but customises which fields are shown and in what order.
 *
 * Inject-specific tweaks:
 *  - topic and payload shown as TypedInput fields
 *  - repeat interval, once checkbox, onceDelay
 *  - hides crontab (rarely used in simple mode)
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

/** Keys to include in the inject editor, in display order. */
const INJECT_SCHEMA_KEYS = [
  "payload",
  "payloadType",
  "topic",
  "repeat",
  "once",
  "onceDelay",
] as const;

export function InjectEditor({ values, onChange, errors }: NodeEditorProps) {
  // Build a schema with only the keys we want, in order.
  // We derive the schema from the inject node definition defaults.
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    payload: { value: "" },
    payloadType: { value: "date" },
    topic: { value: "" },
    repeat: { value: "" },
    once: { value: false },
    onceDelay: { value: 0.1 },
  };

  // Filter values to only keys in our schema
  const filteredValues: Record<string, unknown> = {};
  for (const key of INJECT_SCHEMA_KEYS) {
    filteredValues[key] = values[key] ?? schema[key].value;
  }

  return (
    <SchemaForm
      schema={schema}
      values={filteredValues}
      onChange={onChange}
      errors={errors}
    />
  );
}

// Self-register
import { registerNodeEditor } from "./registry";
registerNodeEditor("inject", InjectEditor);
