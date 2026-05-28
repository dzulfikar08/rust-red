/**
 * ActionList -- command palette (like VS Code's Ctrl-Shift-P).
 *
 * Shows all registered actions with fuzzy search filtering.
 * Arrow keys navigate, Enter executes, Escape closes.
 *
 * Replaces Node-RED's RED.actionList (~234 lines).
 */

import {
  useState,
  useCallback,
  useEffect,
  useRef,
  useMemo,
} from "react";
import { useActionStore } from "../../store/action-store";
import type { Action } from "../../store/action-store";
import { useKeyboardStore } from "../../store/keyboard-store";

// ---------------------------------------------------------------------------
// Fuzzy search
// ---------------------------------------------------------------------------

/**
 * Very small fuzzy matcher: each char of `query` must appear in order in
 * `text`, but may have gaps.  Returns a score (higher = better match) or
 * -1 if no match.
 */
function fuzzyScore(text: string, query: string): number {
  const lower = text.toLowerCase();
  const q = query.toLowerCase();
  let ti = 0;
  let score = 0;
  let prevMatchIdx = -1;

  for (let qi = 0; qi < q.length; qi++) {
    const ch = q[qi];
    // Find the next occurrence of ch
    while (ti < lower.length && lower[ti] !== ch) {
      ti++;
    }
    if (ti >= lower.length) return -1; // no match

    // Bonus for consecutive characters
    if (prevMatchIdx === ti - 1) {
      score += 10;
    }
    // Bonus for matching at word boundary
    if (ti === 0 || lower[ti - 1] === " " || lower[ti - 1] === ":") {
      score += 5;
    }
    score += 1;
    prevMatchIdx = ti;
    ti++;
  }

  return score;
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface ActionListProps {
  /** Called when the palette should close */
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ActionList({ onClose }: ActionListProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const actionsMap = useActionStore((s) => s.actions);
  const invokeAction = useActionStore((s) => s.invokeAction);
  const bindings = useKeyboardStore((s) => s.bindings);

  // Derive the action list from the map (stable reference via useMemo)
  const actions = useMemo(
    () => Array.from(actionsMap.values()),
    [actionsMap],
  );

  // Build a map of action-id -> shortcut label from the keyboard bindings
  const shortcutLabels = useMemo(() => {
    const map = new Map<string, string>();
    for (const list of bindings.values()) {
      for (const b of list) {
        if (!map.has(b.action)) {
          const parts: string[] = [];
          if (b.modifiers?.ctrl || b.modifiers?.meta) parts.push("Ctrl");
          if (b.modifiers?.shift) parts.push("Shift");
          if (b.modifiers?.alt) parts.push("Alt");
          // Format key for display
          let keyLabel = b.key;
          if (keyLabel === " ") keyLabel = "Space";
          else if (keyLabel.length === 1) keyLabel = keyLabel.toUpperCase();
          parts.push(keyLabel);
          map.set(b.action, parts.join("-"));
        }
      }
    }
    return map;
  }, [bindings]);

  // Filter and sort actions by fuzzy score
  const filtered = useMemo(() => {
    if (!query.trim()) {
      // No query: show all, sorted alphabetically by name
      return [...actions].sort((a, b) => a.name.localeCompare(b.name));
    }

    type Scored = { action: Action; score: number };
    const scored: Scored[] = [];
    for (const action of actions) {
      const nameScore = fuzzyScore(action.name, query);
      const idScore = fuzzyScore(action.id, query);
      const best = Math.max(nameScore, idScore);
      if (best >= 0) {
        scored.push({ action, score: best });
      }
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.map((s) => s.action);
  }, [actions, query]);

  // Reset selected index when filtered list changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [filtered.length]);

  // Auto-focus the input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Scroll the selected item into view
  useEffect(() => {
    const listEl = listRef.current;
    if (!listEl) return;
    const selected = listEl.children[selectedIndex] as HTMLElement | undefined;
    selected?.scrollIntoView?.({ block: "nearest" });
  }, [selectedIndex]);

  // Execute the currently selected action
  const execute = useCallback(
    (index: number) => {
      const action = filtered[index];
      if (!action) return;
      if (action.enabled && !action.enabled()) return;
      invokeAction(action.id);
      onClose();
    },
    [filtered, invokeAction, onClose],
  );

  // Keyboard handler for the palette
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev < filtered.length - 1 ? prev + 1 : 0,
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev > 0 ? prev - 1 : filtered.length - 1,
          );
          break;
        case "Enter":
          e.preventDefault();
          execute(selectedIndex);
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [filtered.length, selectedIndex, execute, onClose],
  );

  // Click on backdrop to close
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        onClose();
      }
    },
    [onClose],
  );

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh]"
      onClick={handleBackdropClick}
      data-testid="action-list-backdrop"
    >
      {/* Semi-transparent dark overlay */}
      <div className="absolute inset-0 bg-black/40" />

      {/* Palette dialog */}
      <div
        className="relative w-[480px] max-h-[50vh] flex flex-col rounded-lg shadow-2xl border border-[#555] bg-[#2a2a2a] overflow-hidden"
        onKeyDown={handleKeyDown}
        data-testid="action-list"
      >
        {/* Search input */}
        <div className="flex items-center px-3 border-b border-[#555]">
          <svg
            className="shrink-0 text-gray-400 mr-2"
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <circle cx="7" cy="7" r="4.5" />
            <line x1="10.5" y1="10.5" x2="14" y2="14" />
          </svg>
          <input
            ref={inputRef}
            type="text"
            className="flex-1 bg-transparent py-2.5 text-sm text-gray-100 outline-none placeholder-gray-500"
            placeholder="Search actions..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            data-testid="action-list-input"
          />
        </div>

        {/* Action list */}
        <div
          ref={listRef}
          className="flex-1 overflow-y-auto py-1"
          data-testid="action-list-items"
        >
          {filtered.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-gray-500">
              No actions found
            </div>
          )}
          {filtered.map((action, i) => {
            const shortcut = shortcutLabels.get(action.id) ?? action.key;
            const disabled = action.enabled ? !action.enabled() : false;
            return (
              <button
                key={action.id}
                type="button"
                className={
                  "w-full flex items-center justify-between px-4 py-2 text-left text-sm " +
                  "transition-colors outline-none " +
                  (i === selectedIndex
                    ? "bg-[#094771] text-white"
                    : disabled
                      ? "text-gray-600"
                      : "text-gray-200 hover:bg-[#333]") +
                  (disabled ? " cursor-default" : " cursor-pointer")
                }
                onClick={() => {
                  if (!disabled) execute(i);
                }}
                onMouseEnter={() => setSelectedIndex(i)}
                data-testid={`action-item-${action.id}`}
                data-action-id={action.id}
              >
                <span className="truncate">{action.name}</span>
                {shortcut && (
                  <span className="ml-4 shrink-0 text-xs text-gray-500 font-mono">
                    {shortcut}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default ActionList;
