/**
 * DebugEditor
 *
 * Custom editor for the debug node.  Shows target selection, console
 * output checkbox, and the "complete payload" option.
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

export function DebugEditor({ values, onChange, errors }: NodeEditorProps) {
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    name: { value: "" },
    active: { value: true },
    tosidebar: { value: true },
    console: { value: false },
    tostatus: { value: false },
    complete: { value: "payload" },
    targetType: { value: "" },
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
registerNodeEditor("debug", DebugEditor);
