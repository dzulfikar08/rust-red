/**
 * CommentEditor
 *
 * Minimal editor for the comment node -- just name and description (info).
 */

import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

export function CommentEditor({ values, onChange, errors }: NodeEditorProps) {
  const schema: Record<string, import("@/red/nodes/types").NodeDefault> = {
    name: { value: "" },
    info: { value: "" },
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
registerNodeEditor("comment", CommentEditor);
