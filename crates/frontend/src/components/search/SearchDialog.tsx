/**
 * SearchDialog - Global search overlay for searching across all flows.
 *
 * Opens with Ctrl+F. Searches nodes by name, type, and property values.
 * Shows results with highlighted matches. Clicking a result navigates
 * to the correct flow tab, selects the node, and centers the viewport.
 */

import { useEffect, useRef, useCallback } from "react";
import { useSearchStore, type SearchResult } from "../../store/search-store";

// ---------------------------------------------------------------------------
// Highlighted text helper
// ---------------------------------------------------------------------------

function HighlightedText({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>;

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const idx = lowerText.indexOf(lowerQuery);

  if (idx === -1) return <>{text}</>;

  return (
    <>
      {text.slice(0, idx)}
      <mark className="bg-yellow-300 dark:bg-yellow-600 text-inherit rounded-sm px-0.5">
        {text.slice(idx, idx + query.length)}
      </mark>
      {text.slice(idx + query.length)}
    </>
  );
}

// ---------------------------------------------------------------------------
// Result item
// ---------------------------------------------------------------------------

function ResultItem({
  result,
  isSelected,
  query,
  onClick,
}: {
  result: SearchResult;
  isSelected: boolean;
  query: string;
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
      data-testid={`search-result-${result.nodeId}`}
    >
      <div className="flex items-center gap-2">
        <span
          className="inline-block w-3 h-3 rounded-sm flex-shrink-0"
          style={{ backgroundColor: "var(--color-primary, #8aa)" }}
          title={result.nodeType}
        />
        <span className="font-medium text-sm truncate text-gray-900 dark:text-gray-100">
          <HighlightedText text={result.nodeLabel} query={query} />
        </span>
        <span className="text-xs text-gray-500 dark:text-gray-400 ml-auto flex-shrink-0">
          {result.nodeType}
        </span>
      </div>
      <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">
        <HighlightedText text={result.matchContext} query={query} />
      </div>
      <div className="text-xs text-gray-400 dark:text-gray-500 mt-0.5">
        {result.flowLabel} &middot; {result.matchField}
      </div>
    </button>
  );
}

// ---------------------------------------------------------------------------
// SearchDialog component
// ---------------------------------------------------------------------------

export function SearchDialog() {
  const isOpen = useSearchStore((s) => s.isOpen);
  const query = useSearchStore((s) => s.query);
  const results = useSearchStore((s) => s.results);
  const isSearching = useSearchStore((s) => s.isSearching);
  const selectedIndex = useSearchStore((s) => s.selectedIndex);

  const close = useSearchStore((s) => s.close);
  const search = useSearchStore((s) => s.search);
  const selectResult = useSearchStore((s) => s.selectResult);
  const nextResult = useSearchStore((s) => s.nextResult);
  const prevResult = useSearchStore((s) => s.prevResult);

  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);

  // Auto-focus input when dialog opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isOpen]);

  // Scroll selected result into view
  useEffect(() => {
    if (!resultsRef.current) return;
    const selected = resultsRef.current.querySelector(
      `[data-testid="search-result-${results[selectedIndex]?.nodeId}"]`,
    );
    if (selected && typeof selected.scrollIntoView === "function") {
      selected.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, results]);

  // Handle input change
  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      search(e.target.value);
    },
    [search],
  );

  // Handle keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        nextResult();
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        prevResult();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        if (results.length > 0) {
          selectResult(selectedIndex);
        }
        return;
      }
    },
    [close, nextResult, prevResult, selectResult, results.length, selectedIndex],
  );

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        close();
      }
    },
    [close],
  );

  if (!isOpen) return null;

  const resultCount = results.length;
  const isQueryEmpty = query.trim().length === 0;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-20"
      onClick={handleBackdropClick}
      data-testid="search-dialog-backdrop"
    >
      {/* Backdrop overlay */}
      <div className="absolute inset-0 bg-black/30" />

      {/* Dialog panel */}
      <div
        className="relative w-full max-w-lg bg-white dark:bg-gray-800 rounded-lg shadow-2xl border border-gray-200 dark:border-gray-700 overflow-hidden"
        data-testid="search-dialog"
      >
        {/* Search input */}
        <div className="flex items-center border-b border-gray-200 dark:border-gray-700 px-3 py-2">
          <svg
            className="w-4 h-4 text-gray-400 mr-2 flex-shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            ref={inputRef}
            type="text"
            className="flex-1 bg-transparent text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none"
            placeholder="Search nodes..."
            value={query}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            data-testid="search-input"
          />
          {isSearching && (
            <div className="ml-2 w-4 h-4 border-2 border-gray-300 dark:border-gray-600 border-t-blue-500 dark:border-t-blue-400 rounded-full animate-spin" />
          )}
        </div>

        {/* Results */}
        <div
          ref={resultsRef}
          className="max-h-80 overflow-y-auto"
          data-testid="search-results"
        >
          {!isQueryEmpty && !isSearching && resultCount === 0 && (
            <div className="px-3 py-6 text-center text-sm text-gray-500 dark:text-gray-400" data-testid="search-no-results">
              No results
            </div>
          )}
          {results.map((result, index) => (
            <ResultItem
              key={`${result.nodeId}-${result.matchField}`}
              result={result}
              isSelected={index === selectedIndex}
              query={query}
              onClick={() => selectResult(index)}
            />
          ))}
        </div>

        {/* Footer */}
        {!isQueryEmpty && (
          <div className="border-t border-gray-200 dark:border-gray-700 px-3 py-1.5 flex items-center justify-between text-xs text-gray-400 dark:text-gray-500">
            <span data-testid="search-result-count">
              {resultCount > 0 ? `${resultCount} result${resultCount !== 1 ? "s" : ""} found` : ""}
            </span>
            <span className="flex gap-3">
              <span>
                <kbd className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 font-mono text-[10px]">&uarr;&darr;</kbd> navigate
              </span>
              <span>
                <kbd className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 font-mono text-[10px]">Enter</kbd> select
              </span>
              <span>
                <kbd className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 font-mono text-[10px]">Esc</kbd> close
              </span>
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
