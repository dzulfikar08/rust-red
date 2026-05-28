/**
 * CodeEditor component -- a thin wrapper around `@monaco-editor/react` with
 * Node-RED defaults, theme sync, and autocomplete registration.
 *
 * Features:
 *   - Theme synced with the app theme store via `useEditorTheme`
 *   - Compact mode: smaller font, no minimap, reduced padding
 *   - Loading state with "Loading editor..." placeholder
 *   - Error fallback to a plain `<textarea>` if Monaco fails to load
 *   - Node-RED API autocomplete registered on mount
 */

import { useCallback, useRef, useState, Suspense, lazy } from "react";
import type { OnMount } from "@monaco-editor/react";
import type { editor as MonacoEditor } from "monaco-editor";
import { useEditorTheme } from "./use-editor-theme";
import { registerNRAutocomplete } from "./nr-autocomplete";

// ---------------------------------------------------------------------------
// Lazy loaded Monaco -- ~2 MB bundle; only fetched when editor is opened.
// ---------------------------------------------------------------------------

const MonacoEditor = lazy(() => import("@monaco-editor/react"));

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type CodeEditorLanguage =
  | "javascript"
  | "typescript"
  | "json"
  | "css"
  | "html"
  | "plaintext"
  | "python";

export interface CodeEditorProps {
  /** Current editor value */
  value: string;
  /** Language for syntax highlighting */
  language: CodeEditorLanguage;
  /** Called when the content changes */
  onChange: (value: string) => void;
  /** Read-only mode (default false) */
  readOnly?: boolean;
  /** Show line numbers (default true) */
  lineNumbers?: boolean;
  /** Show minimap (default false) */
  minimap?: boolean;
  /** Editor height (default "200px") */
  height?: string;
  /** Explicit theme override; if omitted the app theme store is used */
  theme?: "vs-dark" | "light";
  /** Placeholder text shown in the fallback textarea */
  placeholder?: string;
  /** Auto-focus the editor on mount (default false) */
  autoFocus?: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function CodeEditor({
  value,
  language,
  onChange,
  readOnly = false,
  lineNumbers = true,
  minimap = false,
  height = "200px",
  theme: themeOverride,
  placeholder = "",
  autoFocus = false,
}: CodeEditorProps) {
  const resolvedTheme = useEditorTheme();
  const theme = themeOverride ?? resolvedTheme;
  const editorRef = useRef<MonacoEditor.IStandaloneCodeEditor | null>(null);
  const [monacoError, setMonacoError] = useState(false);

  // ---- Monaco mount handler ----

  const handleMount: OnMount = useCallback(
    (editor, monaco) => {
      editorRef.current = editor;

      // Register Node-RED autocomplete for JS/TS languages
      if (language === "javascript" || language === "typescript") {
        registerNRAutocomplete(monaco);
      }

      if (autoFocus) {
        editor.focus();
      }
    },
    [language, autoFocus],
  );

  // ---- Change handler ----

  const handleChange = useCallback(
    (newValue: string | undefined) => {
      onChange(newValue ?? "");
    },
    [onChange],
  );

  // ---- Editor options matching Node-RED's behavior ----

  const options: MonacoEditor.IStandaloneEditorConstructionOptions = {
    readOnly,
    lineNumbers: lineNumbers ? "on" : "off",
    minimap: { enabled: minimap },
    wordWrap: "on",
    autoClosingBrackets: "always",
    formatOnPaste: true,
    scrollBeyondLastLine: false,
    fontSize: 12,
    fontFamily: "monospace",
    padding: { top: 4, bottom: 4 },
    tabSize: 2,
    renderLineHighlight: "line",
    contextmenu: true,
    folding: true,
    glyphMargin: false,
  };

  // ---- Error fallback: plain textarea ----

  if (monacoError) {
    return (
      <textarea
        data-testid="code-editor-fallback"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        readOnly={readOnly}
        placeholder={placeholder}
        style={{
          width: "100%",
          height,
          fontFamily: "monospace",
          fontSize: 12,
          padding: 4,
          border: "1px solid #ccc",
          borderRadius: 4,
          resize: "vertical",
          backgroundColor: theme === "vs-dark" ? "#1e1e1e" : "#fff",
          color: theme === "vs-dark" ? "#d4d4d4" : "#000",
        }}
      />
    );
  }

  // ---- Monaco Editor with Suspense boundary ----

  return (
    <div
      data-testid="code-editor-wrapper"
      style={{ height, width: "100%" }}
    >
      <Suspense
        fallback={
          <div
            data-testid="code-editor-loading"
            style={{
              height,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontFamily: "monospace",
              fontSize: 12,
              color: theme === "vs-dark" ? "#888" : "#999",
              border: "1px solid #ccc",
              borderRadius: 4,
            }}
          >
            Loading editor...
          </div>
        }
      >
        <MonacoEditor
          height={height}
          language={language}
          value={value}
          theme={theme}
          onChange={handleChange}
          onMount={handleMount}
          loading={
            <div
              data-testid="code-editor-monaco-loading"
              style={{
                height,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontFamily: "monospace",
                fontSize: 12,
                color: "#888",
              }}
            >
              Loading editor...
            </div>
          }
          options={options}
          onError={() => setMonacoError(true)}
        />
      </Suspense>
    </div>
  );
}
