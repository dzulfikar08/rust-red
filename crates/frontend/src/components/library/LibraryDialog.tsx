/**
 * LibraryDialog — modal dialog for the flow/function/template library.
 *
 * Replaces Node-RED's RED.library UI.
 * Features:
 *  - Left panel: tree/list of saved library entries with search/filter
 *  - Right panel: preview of selected entry (formatted JSON or description)
 *  - Actions: Save, Load (insert into current flow), Delete, Rename
 *  - Accessible from header menu or context menu
 */

import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import {
  useLibraryStore,
  type LibraryEntry,
} from "../../store/library-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LibraryDialogProps {
  /** Whether the dialog is open. */
  isOpen: boolean;
  /** Callback to close the dialog. */
  onClose: () => void;
  /** Callback when user loads an entry into the flow. */
  onLoad?: (entry: LibraryEntry) => void;
  /** Optional pre-filled content for "save" mode. */
  initialContent?: string;
  /** Optional initial type for "save" mode. */
  initialType?: LibraryEntry["type"];
}

// ---------------------------------------------------------------------------
// Type badge
// ---------------------------------------------------------------------------

const TYPE_COLORS: Record<LibraryEntry["type"], string> = {
  flow: "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300",
  function:
    "bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300",
  template:
    "bg-purple-100 text-purple-800 dark:bg-purple-900/40 dark:text-purple-300",
};

function TypeBadge({ type }: { type: LibraryEntry["type"] }) {
  return (
    <span
      className={`inline-block px-1.5 py-0.5 rounded text-[10px] font-medium uppercase tracking-wide ${TYPE_COLORS[type] ?? ""}`}
      data-testid={`library-type-badge-${type}`}
    >
      {type}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Format preview content
// ---------------------------------------------------------------------------

function formatPreview(entry: LibraryEntry | null): string {
  if (!entry) return "";
  try {
    const parsed = JSON.parse(entry.content);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return entry.content;
  }
}

// ---------------------------------------------------------------------------
// Entry list item
// ---------------------------------------------------------------------------

function EntryListItem({
  entry,
  isSelected,
  onClick,
}: {
  entry: LibraryEntry;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`w-full text-left px-3 py-2 border-b border-gray-200 dark:border-gray-700 transition-colors ${
        isSelected
          ? "bg-blue-100 dark:bg-blue-900/40"
          : "hover:bg-gray-50 dark:hover:bg-gray-800"
      }`}
      onClick={onClick}
      data-testid={`library-entry-${entry.id}`}
    >
      <div className="flex items-center gap-2">
        <span className="font-medium text-sm truncate text-gray-900 dark:text-gray-100">
          {entry.name}
        </span>
        <TypeBadge type={entry.type} />
      </div>
      <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">
        {entry.path || "/"}
      </div>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Save form
// ---------------------------------------------------------------------------

function SaveForm({
  onSave,
  onCancel,
  initialContent,
  initialType,
}: {
  onSave: (name: string, type: LibraryEntry["type"], path: string, content: string) => void;
  onCancel: () => void;
  initialContent?: string;
  initialType?: LibraryEntry["type"];
}) {
  const [name, setName] = useState("");
  const [type, setType] = useState<LibraryEntry["type"]>(initialType ?? "flow");
  const [path, setPath] = useState("/");
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    nameRef.current?.focus();
  }, []);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (name.trim().length === 0) return;
      onSave(name.trim(), type, path.trim() || "/", initialContent ?? "{}");
    },
    [name, type, path, initialContent, onSave],
  );

  return (
    <form onSubmit={handleSubmit} className="p-3 space-y-3" data-testid="library-save-form">
      <div>
        <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
          Name
        </label>
        <input
          ref={nameRef}
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 outline-none focus:ring-1 focus:ring-blue-500"
          placeholder="My flow"
          data-testid="library-save-name"
        />
      </div>
      <div>
        <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
          Type
        </label>
        <select
          value={type}
          onChange={(e) => setType(e.target.value as LibraryEntry["type"])}
          className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 outline-none focus:ring-1 focus:ring-blue-500"
          data-testid="library-save-type"
        >
          <option value="flow">Flow</option>
          <option value="function">Function</option>
          <option value="template">Template</option>
        </select>
      </div>
      <div>
        <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
          Path
        </label>
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 outline-none focus:ring-1 focus:ring-blue-500"
          placeholder="/"
          data-testid="library-save-path"
        />
      </div>
      <div className="flex gap-2 justify-end pt-1">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-1.5 text-sm rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          data-testid="library-save-cancel"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={name.trim().length === 0}
          className="px-3 py-1.5 text-sm rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          data-testid="library-save-submit"
        >
          Save
        </button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// LibraryDialog component
// ---------------------------------------------------------------------------

export function LibraryDialog({
  isOpen,
  onClose,
  onLoad,
  initialContent,
  initialType,
}: LibraryDialogProps) {
  const entries = useLibraryStore((s) => s.entries);
  const saveToLibrary = useLibraryStore((s) => s.saveToLibrary);
  const deleteFromLibrary = useLibraryStore((s) => s.deleteFromLibrary);
  const updateEntry = useLibraryStore((s) => s.updateEntry);
  const searchStore = useLibraryStore((s) => s.search);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filterQuery, setFilterQuery] = useState("");
  const [mode, setMode] = useState<"browse" | "save">("browse");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const dialogRef = useRef<HTMLDivElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  // Filter entries based on search query
  const filteredEntries = useMemo(() => {
    if (filterQuery.trim().length === 0) return entries;
    return searchStore(filterQuery);
  }, [entries, filterQuery, searchStore]);

  // Selected entry object
  const selectedEntry = useMemo(
    () => entries.find((e) => e.id === selectedId) ?? null,
    [entries, selectedId],
  );

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (isOpen) {
      setSelectedId(null);
      setFilterQuery("");
      setMode("browse");
      setRenamingId(null);
    }
  }, [isOpen]);

  // Focus rename input
  useEffect(() => {
    if (renamingId && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renamingId]);

  // Keyboard handling
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (renamingId) {
          setRenamingId(null);
        } else if (mode === "save") {
          setMode("browse");
        } else {
          onClose();
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose, renamingId, mode]);

  // Handle save
  const handleSave = useCallback(
    (name: string, type: LibraryEntry["type"], path: string, content: string) => {
      saveToLibrary({ name, type, path, content });
      setMode("browse");
    },
    [saveToLibrary],
  );

  // Handle load
  const handleLoad = useCallback(() => {
    if (selectedEntry && onLoad) {
      onLoad(selectedEntry);
      onClose();
    }
  }, [selectedEntry, onLoad, onClose]);

  // Handle delete
  const handleDelete = useCallback(() => {
    if (selectedId) {
      deleteFromLibrary(selectedId);
      setSelectedId(null);
    }
  }, [selectedId, deleteFromLibrary]);

  // Handle rename
  const handleRenameStart = useCallback(() => {
    if (selectedEntry) {
      setRenamingId(selectedEntry.id);
      setRenameValue(selectedEntry.name);
    }
  }, [selectedEntry]);

  const handleRenameConfirm = useCallback(() => {
    if (renamingId && renameValue.trim().length > 0) {
      updateEntry(renamingId, { name: renameValue.trim() });
      setRenamingId(null);
    }
  }, [renamingId, renameValue, updateEntry]);

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        onClose();
      }
    },
    [onClose],
  );

  if (!isOpen) return null;

  const previewText = formatPreview(selectedEntry);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      onClick={handleBackdropClick}
      data-testid="library-dialog-backdrop"
    >
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/40" />

      {/* Dialog */}
      <div
        ref={dialogRef}
        className="relative w-full max-w-3xl h-[70vh] bg-white dark:bg-gray-800 rounded-lg shadow-2xl border border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden"
        data-testid="library-dialog"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
            Flow Library
          </h2>
          <div className="flex items-center gap-2">
            {mode === "browse" && (
              <button
                type="button"
                onClick={() => setMode("save")}
                className="px-3 py-1.5 text-xs font-medium rounded bg-blue-600 text-white hover:bg-blue-700 transition-colors"
                data-testid="library-save-button"
              >
                Save to Library
              </button>
            )}
            <button
              type="button"
              onClick={onClose}
              className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-500 dark:text-gray-400 transition-colors"
              aria-label="Close"
              data-testid="library-close-button"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </div>

        {/* Search bar */}
        <div className="px-4 py-2 border-b border-gray-200 dark:border-gray-700">
          <input
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Filter entries..."
            className="w-full px-2 py-1.5 text-sm bg-transparent text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none border border-gray-300 dark:border-gray-600 rounded"
            data-testid="library-filter-input"
          />
        </div>

        {/* Content */}
        <div className="flex-1 flex min-h-0">
          {mode === "browse" ? (
            <>
              {/* Left panel — entry list */}
              <div
                className="w-1/2 border-r border-gray-200 dark:border-gray-700 overflow-y-auto"
                data-testid="library-entry-list"
              >
                {filteredEntries.length === 0 && (
                  <div className="px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400" data-testid="library-empty">
                    {entries.length === 0
                      ? "Library is empty. Save a flow to get started."
                      : "No matching entries."}
                  </div>
                )}
                {filteredEntries.map((entry) => (
                  <div key={entry.id} className="relative">
                    {renamingId === entry.id ? (
                      <div className="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
                        <input
                          ref={renameInputRef}
                          type="text"
                          value={renameValue}
                          onChange={(e) => setRenameValue(e.target.value)}
                          onBlur={handleRenameConfirm}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleRenameConfirm();
                            if (e.key === "Escape") setRenamingId(null);
                          }}
                          className="w-full px-2 py-1 text-sm border border-blue-400 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 outline-none"
                          data-testid="library-rename-input"
                        />
                      </div>
                    ) : (
                      <EntryListItem
                        entry={entry}
                        isSelected={entry.id === selectedId}
                        onClick={() => setSelectedId(entry.id)}
                      />
                    )}
                  </div>
                ))}
              </div>

              {/* Right panel — preview */}
              <div
                className="w-1/2 overflow-y-auto p-4"
                data-testid="library-preview"
              >
                {selectedEntry ? (
                  <div>
                    <div className="flex items-center gap-2 mb-3">
                      <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                        {selectedEntry.name}
                      </h3>
                      <TypeBadge type={selectedEntry.type} />
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                      Path: {selectedEntry.path}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-4">
                      Updated: {new Date(selectedEntry.updatedAt).toLocaleString()}
                    </div>
                    <pre className="text-xs bg-gray-50 dark:bg-gray-900 rounded p-3 overflow-x-auto text-gray-800 dark:text-gray-200 border border-gray-200 dark:border-gray-700 whitespace-pre-wrap break-all">
                      {previewText}
                    </pre>
                  </div>
                ) : (
                  <div className="flex items-center justify-center h-full text-sm text-gray-400 dark:text-gray-500">
                    Select an entry to preview
                  </div>
                )}
              </div>
            </>
          ) : (
            /* Save mode */
            <div className="flex-1 overflow-y-auto">
              <SaveForm
                onSave={handleSave}
                onCancel={() => setMode("browse")}
                initialContent={initialContent}
                initialType={initialType}
              />
            </div>
          )}
        </div>

        {/* Footer — actions for selected entry */}
        {mode === "browse" && selectedEntry && (
          <div className="flex items-center gap-2 px-4 py-2 border-t border-gray-200 dark:border-gray-700">
            <button
              type="button"
              onClick={handleLoad}
              className="px-3 py-1.5 text-xs font-medium rounded bg-blue-600 text-white hover:bg-blue-700 transition-colors"
              data-testid="library-load-button"
            >
              Load
            </button>
            <button
              type="button"
              onClick={handleRenameStart}
              className="px-3 py-1.5 text-xs font-medium rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              data-testid="library-rename-button"
            >
              Rename
            </button>
            <button
              type="button"
              onClick={handleDelete}
              className="px-3 py-1.5 text-xs font-medium rounded border border-red-300 dark:border-red-700 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
              data-testid="library-delete-button"
            >
              Delete
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export default LibraryDialog;
