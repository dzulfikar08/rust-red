/**
 * Barrel export for code editor components.
 */

export { CodeEditor } from "./CodeEditor";
export type {
  CodeEditorProps,
  CodeEditorLanguage,
} from "./CodeEditor";

export { useEditorTheme } from "./use-editor-theme";
export type { EditorTheme } from "./use-editor-theme";

export {
  registerNRAutocomplete,
  _resetRegistration,
  _getCompletions,
} from "./nr-autocomplete";
