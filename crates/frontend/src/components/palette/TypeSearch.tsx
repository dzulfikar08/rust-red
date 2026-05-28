/**
 * TypeSearch Component
 *
 * Quick-search floating panel (Ctrl+Shift+Space) that shows a
 * searchable list of all node types. Supports arrow-key navigation,
 * Enter to select, and Escape to close.
 */

import {
  useState,
  useEffect,
  useRef,
  useMemo,
  useCallback,
} from "react";
import { Search } from "lucide-react";
import { nodeRegistry } from "../../red/nodes/registry";
import type { NodeDefinition } from "../../red/nodes/types";
import { eventBus } from "../../red/core/events";

interface TypeSearchProps {
  /** Whether the panel is visible. */
  open: boolean;
  /** Callback to close the panel. */
  onClose: () => void;
  /** Position of the panel (top-left corner). */
  x: number;
  /** Position of the panel (top-left corner). */
  y: number;
}

interface FlatEntry {
  node: NodeDefinition;
  category: string;
}

export function TypeSearch({ open, onClose, x, y }: TypeSearchProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const allEntries = useMemo<FlatEntry[]>(() => {
    const entries: FlatEntry[] = [];
    const categories = nodeRegistry.getCategories();
    for (const cat of categories) {
      const nodes = nodeRegistry.getTypesByCategory(cat);
      for (const node of nodes) {
        entries.push({ node, category: cat });
      }
    }
    return entries;
  }, []);

  const filtered = useMemo(() => {
    const q = query.toLowerCase().trim();
    if (!q) return allEntries;
    return allEntries.filter(
      (e) =>
        e.node.name.toLowerCase().includes(q) ||
        e.node.type.toLowerCase().includes(q) ||
        (e.node.paletteLabel && e.node.paletteLabel.toLowerCase().includes(q)),
    );
  }, [allEntries, query]);

  // Reset selection when filtered results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [filtered]);

  // Auto-focus input when opening
  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      // Small delay to ensure DOM is ready
      requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
    }
  }, [open]);

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector("[data-selected='true']");
    selected?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const handleSelect = useCallback(
    (entry: FlatEntry) => {
      eventBus.emit("node:add-requested", { type: entry.node.type });
      onClose();
    },
    [onClose],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev < filtered.length - 1 ? prev + 1 : prev,
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) => (prev > 0 ? prev - 1 : prev));
          break;
        case "Enter":
          e.preventDefault();
          if (filtered[selectedIndex]) {
            handleSelect(filtered[selectedIndex]);
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [filtered, selectedIndex, handleSelect, onClose],
  );

  if (!open) return null;

  // Clamp position to keep panel on screen
  const panelWidth = 280;
  const panelHeight = 320;
  const clampedX = Math.min(x, window.innerWidth - panelWidth - 8);
  const clampedY = Math.min(y, window.innerHeight - panelHeight - 8);

  return (
    <div
      className="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg shadow-xl overflow-hidden"
      style={{
        left: clampedX,
        top: clampedY,
        width: panelWidth,
        maxHeight: panelHeight,
      }}
      data-testid="type-search"
    >
      {/* Search input */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-200 dark:border-gray-700">
        <Search size={14} className="text-gray-400 flex-shrink-0" />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search node types..."
          className="flex-1 bg-transparent text-sm text-gray-700 dark:text-gray-300 placeholder-gray-400 outline-none"
          aria-label="Search node types"
          data-testid="type-search-input"
        />
      </div>

      {/* Results list */}
      <div
        ref={listRef}
        className="overflow-y-auto"
        style={{ maxHeight: panelHeight - 44 }}
      >
        {filtered.length === 0 && (
          <div className="px-3 py-4 text-sm text-gray-400 text-center">
            No matching nodes
          </div>
        )}
        {filtered.map((entry, index) => {
          const label = entry.node.paletteLabel ?? entry.node.name;
          const isSelected = index === selectedIndex;
          return (
            <div
              key={entry.node.type}
              data-selected={isSelected}
              className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer text-sm transition-colors ${
                isSelected
                  ? "bg-blue-100 dark:bg-blue-900/40"
                  : "hover:bg-gray-100 dark:hover:bg-gray-700"
              }`}
              onClick={() => handleSelect(entry)}
              role="option"
              aria-selected={isSelected}
            >
              <span
                className="w-3 h-3 rounded-sm flex-shrink-0 border border-black/10"
                style={{ backgroundColor: entry.node.color }}
              />
              <div className="flex flex-col min-w-0">
                <span className="truncate text-gray-800 dark:text-gray-200">
                  {label}
                </span>
                <span className="text-[10px] text-gray-400 truncate">
                  {entry.category} / {entry.node.type}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
