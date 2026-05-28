/**
 * FunctionEditor -- Custom editor for Node-RED's "function" node.
 *
 * Provides a multi-tab interface with:
 *   - On Message tab: Monaco code editor for the main function body
 *   - On Start tab: Code that runs when the node is deployed (initialize)
 *   - On Stop tab: Code that runs when the node is stopped (finalize)
 *   - Setup tab: Outputs count, timeout
 *
 * Uses the CodeEditor component for JavaScript editing.
 */

import { useState, useCallback } from "react";
import { CodeEditor } from "../code-editors/CodeEditor";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TabId = "body" | "init" | "finalize" | "setup";

interface TabDef {
  id: TabId;
  label: string;
}

const TABS: TabDef[] = [
  { id: "setup", label: "Setup" },
  { id: "init", label: "On Start" },
  { id: "body", label: "On Message" },
  { id: "finalize", label: "On Stop" },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function FunctionEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const [activeTab, setActiveTab] = useState<TabId>("body");

  const func = (values.func as string) ?? "\nreturn msg;";
  const initialize = (values.initialize as string) ?? "";
  const finalize = (values.finalize as string) ?? "";
  const outputs = (values.outputs as number) ?? 1;
  const timeout = (values.timeout as number) ?? 0;

  const handleFuncChange = useCallback(
    (v: string) => onChange("func", v),
    [onChange],
  );
  const handleInitChange = useCallback(
    (v: string) => onChange("initialize", v),
    [onChange],
  );
  const handleFinalizeChange = useCallback(
    (v: string) => onChange("finalize", v),
    [onChange],
  );
  const handleOutputsChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.max(0, Math.min(500, parseInt(e.target.value, 10) || 1));
      onChange("outputs", v);
    },
    [onChange],
  );
  const handleTimeoutChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.max(0, parseInt(e.target.value, 10) || 0);
      onChange("timeout", v);
    },
    [onChange],
  );

  return (
    <div className="space-y-2" data-testid="function-editor">
      {/* Tab bar */}
      <div
        className="flex border-b border-gray-200 dark:border-gray-700"
        role="tablist"
        data-testid="function-editor-tabs"
      >
        {TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`px-3 py-1.5 text-xs font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? "border-blue-500 text-blue-600 dark:text-blue-400"
                : "border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
            }`}
            onClick={() => setActiveTab(tab.id)}
            data-testid={`function-tab-${tab.id}`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="mt-2" data-testid="function-tab-content">
        {/* Setup tab */}
        {activeTab === "setup" && (
          <div className="space-y-3" data-testid="function-setup-panel">
            <div className="flex items-center gap-4">
              <div>
                <label
                  className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
                  htmlFor="func-outputs"
                >
                  Outputs
                </label>
                <input
                  id="func-outputs"
                  type="number"
                  min={0}
                  max={500}
                  className="w-20 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  value={outputs}
                  onChange={handleOutputsChange}
                  data-testid="function-outputs"
                />
              </div>
              <div>
                <label
                  className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1"
                  htmlFor="func-timeout"
                >
                  Timeout (s)
                </label>
                <input
                  id="func-timeout"
                  type="number"
                  min={0}
                  className="w-20 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  value={timeout}
                  onChange={handleTimeoutChange}
                  placeholder="0"
                  data-testid="function-timeout"
                />
              </div>
            </div>
          </div>
        )}

        {/* On Start tab */}
        {activeTab === "init" && (
          <div data-testid="function-init-panel">
            <CodeEditor
              value={initialize}
              language="javascript"
              onChange={handleInitChange}
              height="250px"
              placeholder="// Code to run when the node is deployed"
            />
          </div>
        )}

        {/* On Message tab */}
        {activeTab === "body" && (
          <div data-testid="function-body-panel">
            <CodeEditor
              value={func}
              language="javascript"
              onChange={handleFuncChange}
              height="250px"
            />
          </div>
        )}

        {/* On Stop tab */}
        {activeTab === "finalize" && (
          <div data-testid="function-finalize-panel">
            <CodeEditor
              value={finalize}
              language="javascript"
              onChange={handleFinalizeChange}
              height="250px"
              placeholder="// Code to run when the node is stopped"
            />
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("function", FunctionEditor);
