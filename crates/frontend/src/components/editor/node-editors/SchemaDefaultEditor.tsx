/**
 * SchemaDefaultEditor
 *
 * The fallback editor for node types that do not have a custom editor
 * component registered. It renders a SchemaForm using the node type's
 * defaults from the node registry.
 */

import { nodeRegistry } from "@/red/nodes/registry";
import { SchemaForm } from "../schema-forms/SchemaForm";
import type { NodeEditorProps } from "./registry";

/**
 * Filter out internal / hidden properties that should not appear in the
 * default schema editor.  These are typically arrays of link IDs or
 * runtime-only state.
 */
const HIDDEN_KEYS = new Set(["links", "wires"]);

export function SchemaDefaultEditor({
  nodeType,
  values,
  onChange,
  errors,
}: NodeEditorProps) {
  const def = nodeRegistry.getType(nodeType);
  if (!def) {
    return (
      <div className="text-sm text-gray-500 italic">
        No schema found for node type &ldquo;{nodeType}&rdquo;.
      </div>
    );
  }

  // Filter schema to exclude hidden keys
  const schema = Object.fromEntries(
    Object.entries(def.defaults).filter(([key]) => !HIDDEN_KEYS.has(key)),
  );

  return (
    <SchemaForm
      schema={schema}
      values={values}
      onChange={onChange}
      errors={errors}
    />
  );
}
