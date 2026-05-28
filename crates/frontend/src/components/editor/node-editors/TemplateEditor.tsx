/**
 * TemplateEditor -- Custom editor for Node-RED's "template" node.
 *
 * Provides:
 *   - Field (property) selector (TypedInput: msg/flow/global)
 *   - Template format select (mustache, html, json, css, etc.)
 *   - Template editor (CodeEditor with syntax highlighting)
 *   - Syntax mode (mustache / plain)
 *   - Output format (plain text, JSON, YAML)
 */

import { useCallback } from "react";
import { TypedInput, type TypedInputType } from "../../common/TypedInput";
import { CodeEditor, type CodeEditorLanguage } from "../code-editors/CodeEditor";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TemplateFormat = "handlebars" | "html" | "json" | "javascript" | "css" | "markdown" | "yaml" | "text";
type SyntaxMode = "mustache" | "plain";
type OutputFormat = "str" | "json" | "yaml";

interface FormatOption {
  value: TemplateFormat;
  label: string;
}

const FORMAT_OPTIONS: FormatOption[] = [
  { value: "handlebars", label: "Mustache" },
  { value: "html", label: "HTML" },
  { value: "json", label: "JSON" },
  { value: "javascript", label: "JavaScript" },
  { value: "css", label: "CSS" },
  { value: "markdown", label: "Markdown" },
  { value: "yaml", label: "YAML" },
  { value: "text", label: "None" },
];

const OUTPUT_OPTIONS: { value: OutputFormat; label: string }[] = [
  { value: "str", label: "Plain Text" },
  { value: "json", label: "Parsed JSON" },
  { value: "yaml", label: "Parsed YAML" },
];

/**
 * Map template format to CodeEditor language for syntax highlighting.
 */
function formatToLanguage(format: TemplateFormat): CodeEditorLanguage {
  switch (format) {
    case "handlebars":
    case "html":
      return "html";
    case "json":
      return "json";
    case "javascript":
      return "javascript";
    case "css":
      return "css";
    case "markdown":
      return "plaintext";
    case "yaml":
      return "plaintext";
    case "text":
    default:
      return "plaintext";
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TemplateEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const field = (values.field as string) ?? "payload";
  const fieldType = (values.fieldType as string) ?? "msg";
  const format = (values.format as TemplateFormat) ?? "handlebars";
  const syntax = (values.syntax as SyntaxMode) ?? "mustache";
  const template = (values.template as string) ?? "This is the payload: {{payload}} !";
  const output = (values.output as OutputFormat) ?? "str";

  const handleFieldChange = useCallback(
    (value: string, type: TypedInputType) => {
      onChange("field", value);
      onChange("fieldType", type);
    },
    [onChange],
  );

  const handleFormatChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("format", e.target.value);
    },
    [onChange],
  );

  const handleSyntaxChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("syntax", e.target.value);
    },
    [onChange],
  );

  const handleTemplateChange = useCallback(
    (value: string) => {
      onChange("template", value);
    },
    [onChange],
  );

  const handleOutputChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("output", e.target.value);
    },
    [onChange],
  );

  return (
    <div className="space-y-3" data-testid="template-editor">
      {/* Field (property to set) */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Property
        </label>
        <TypedInput
          value={field}
          type={fieldType as TypedInputType}
          onChange={handleFieldChange}
          types={["msg", "flow", "global"]}
        />
      </div>

      {/* Template editor with format selector */}
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="text-xs font-semibold text-gray-600 dark:text-gray-300">
            Template
          </label>
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">Format:</span>
            <select
              value={format}
              onChange={handleFormatChange}
              className="border border-gray-300 dark:border-gray-600 rounded px-1.5 py-0.5 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              data-testid="template-format"
            >
              {FORMAT_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
        </div>
        <CodeEditor
          value={template}
          language={formatToLanguage(format)}
          onChange={handleTemplateChange}
          height="250px"
        />
      </div>

      {/* Syntax mode */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Syntax
        </label>
        <select
          value={syntax}
          onChange={handleSyntaxChange}
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          data-testid="template-syntax"
        >
          <option value="mustache">Mustache</option>
          <option value="plain">Plain</option>
        </select>
      </div>

      {/* Output format */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Output
        </label>
        <select
          value={output}
          onChange={handleOutputChange}
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          data-testid="template-output"
        >
          {OUTPUT_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("template", TemplateEditor);
