/**
 * Node Editor barrel export
 *
 * Re-exports the registry API and all built-in editor components.
 * Importing this module triggers the side-effect registration of each
 * built-in editor via their top-level `registerNodeEditor()` calls.
 */

// Registry API
export {
  registerNodeEditor,
  getNodeEditor,
  hasCustomEditor,
  unregisterNodeEditor,
  clearNodeEditors,
} from "./registry";
export type { NodeEditorProps } from "./registry";

// Built-in editors (importing triggers self-registration)
export { SchemaDefaultEditor } from "./SchemaDefaultEditor";
export { InjectEditor } from "./InjectEditor";
export { DebugEditor } from "./DebugEditor";
export { CommentEditor } from "./CommentEditor";
export { DelayEditor } from "./DelayEditor";
export { TriggerEditor } from "./TriggerEditor";
export { LinkInEditor } from "./LinkInEditor";
export { LinkOutEditor } from "./LinkOutEditor";

// Complex node editors (Phase 3 Task 6)
export { FunctionEditor } from "./FunctionEditor";
export { SwitchEditor } from "./SwitchEditor";
export type { SwitchRule } from "./SwitchEditor";
export { ChangeEditor } from "./ChangeEditor";
export type { ChangeRule } from "./ChangeEditor";
export { TemplateEditor } from "./TemplateEditor";
export { HttpRequestEditor } from "./HttpRequestEditor";
