/**
 * LinkOutEditor
 *
 * Minimal editor for the link-out node -- just a name field and mode.
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

export function LinkOutEditor({ values, onChange, errors }: NodeEditorProps) {
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    name: { value: "" },
    mode: { value: "link" },
  };

  const filteredValues: Record<string, unknown> = {};
  for (const key of Object.keys(schema)) {
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
registerNodeEditor("link out", LinkOutEditor);
