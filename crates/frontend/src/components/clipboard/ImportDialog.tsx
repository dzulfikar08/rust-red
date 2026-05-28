/**
 * ImportDialog — modal dialog for importing Node-RED flow JSON.
 *
 * Features:
 *  - Large textarea for pasting JSON
 *  - Drag-and-drop file support
 *  - JSON validation with error display
 *  - Import button / Cancel button
 */

import { useState, useRef, useCallback } from "react";
import { X, Upload, FileJson } from "lucide-react";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface ImportDialogProps {
  /** Whether the dialog is open */
  open: boolean;
  /** Close the dialog */
  onClose: () => void;
  /** Callback with the JSON string to import */
  onImport: (json: string) => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ImportDialog({ open, onClose, onImport }: ImportDialogProps) {
  const [jsonText, setJsonText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleImport = useCallback(() => {
    const trimmed = jsonText.trim();
    if (!trimmed) {
      setError("Please paste or drop a JSON flow");
      return;
    }

    try {
      JSON.parse(trimmed);
      setError(null);
      onImport(trimmed);
      setJsonText("");
    } catch (e) {
      setError(
        `Invalid JSON: ${e instanceof Error ? e.message : "parse error"}`,
      );
    }
  }, [jsonText, onImport]);

  const handleClose = useCallback(() => {
    setJsonText("");
    setError(null);
    setIsDragOver(false);
    onClose();
  }, [onClose]);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);

      const file = e.dataTransfer.files[0];
      if (file) {
        readFile(file);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) {
        readFile(file);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  function readFile(file: File) {
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setJsonText(text);
      setError(null);
    };
    reader.onerror = () => {
      setError("Failed to read file");
    };
    reader.readAsText(file);
  }

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        className="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[600px] max-h-[80vh] flex flex-col"
        data-testid="import-dialog"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <FileJson size={18} className="text-gray-500" />
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Import Flows
            </h2>
          </div>
          <button
            type="button"
            onClick={handleClose}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-500"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-auto p-4">
          <div
            className={`relative rounded border-2 transition-colors ${
              isDragOver
                ? "border-blue-400 bg-blue-50 dark:bg-blue-900/20"
                : "border-gray-200 dark:border-gray-600"
            }`}
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
          >
            <textarea
              ref={textareaRef}
              className="w-full h-64 p-3 text-xs font-mono bg-transparent resize-none focus:outline-none text-gray-900 dark:text-gray-100 placeholder-gray-400"
              placeholder='Paste your flow JSON here, or drag & drop a file...\n[{"id":"abc123","type":"inject",...}]'
              value={jsonText}
              onChange={(e) => {
                setJsonText(e.target.value);
                setError(null);
              }}
              data-testid="import-textarea"
            />
            {isDragOver && (
              <div className="absolute inset-0 flex items-center justify-center bg-blue-50/80 dark:bg-blue-900/30 rounded pointer-events-none">
                <div className="flex flex-col items-center gap-1 text-blue-600 dark:text-blue-400">
                  <Upload size={24} />
                  <span className="text-sm font-medium">Drop file here</span>
                </div>
              </div>
            )}
          </div>

          {/* File input */}
          <div className="mt-2 flex items-center gap-2">
            <input
              ref={fileInputRef}
              type="file"
              accept=".json,.flow"
              className="hidden"
              onChange={handleFileSelect}
              data-testid="import-file-input"
            />
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 underline"
            >
              Choose a file...
            </button>
          </div>

          {/* Error */}
          {error && (
            <div
              className="mt-2 px-3 py-2 rounded bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-400 text-xs"
              data-testid="import-error"
            >
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-gray-200 dark:border-gray-700">
          <button
            type="button"
            onClick={handleClose}
            className="px-3 py-1.5 text-xs rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            data-testid="import-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleImport}
            className="px-3 py-1.5 text-xs rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={!jsonText.trim()}
            data-testid="import-button"
          >
            Import
          </button>
        </div>
      </div>
    </div>
  );
}

export default ImportDialog;
