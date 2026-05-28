/**
 * TriggerEditor
 *
 * Custom editor for the trigger node.  Provides duration, repeat,
 * output value configuration, and reset trigger.
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

export function TriggerEditor({ values, onChange, errors }: NodeEditorProps) {
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    name: { value: "" },
    op1: { value: "" },
    op2: { value: "" },
    op1type: { value: "pay" },
    op2type: { value: "nul" },
    duration: { value: "250" },
    extend: { value: false },
    units: { value: "ms" },
    reset: { value: "" },
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
registerNodeEditor("trigger", TriggerEditor);
