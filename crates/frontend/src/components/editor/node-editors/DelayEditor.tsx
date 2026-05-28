/**
 * DelayEditor
 *
 * Custom editor for the delay node.  Provides a delay-type selector
 * (delay, rate, random, etc.), delay value, and units.
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

export function DelayEditor({ values, onChange, errors }: NodeEditorProps) {
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    name: { value: "" },
    pauseType: { value: "delay" },
    timeout: { value: "5" },
    timeoutUnits: { value: "seconds" },
    rate: { value: "1" },
    nbRateUnits: { value: "1" },
    rateUnits: { value: "second" },
    randomFirst: { value: "1" },
    randomLast: { value: "5" },
    randomUnits: { value: "seconds" },
    drop: { value: false },
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
registerNodeEditor("delay", DelayEditor);
