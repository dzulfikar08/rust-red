/**
 * EventLogPanel -- replaces Node-RED's RED.eventLog panel.
 *
 * Displays a reverse-chronological log of application events captured
 * from the event bus. Entries are color-coded by level, filterable,
 * and auto-scroll to show the newest entries.
 */

import { useCallback, useMemo, useRef, useEffect } from "react";
import { Trash2, Search, Info, AlertTriangle, XCircle } from "lucide-react";
import {
  useEventLogStore,
  selectFilteredEntries,
} from "../../store/event-log-store";
import type { LogEntry } from "../../store/event-log-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatTimestampFull(ts: number): string {
  return new Date(ts).toLocaleString();
}

const levelConfig: Record<
  LogEntry["level"],
  { icon: typeof Info; color: string; bg: string }
> = {
  info: {
    icon: Info,
    color: "text-blue-500 dark:text-blue-400",
    bg: "bg-blue-50 dark:bg-blue-900/20",
  },
  warn: {
    icon: AlertTriangle,
    color: "text-amber-500 dark:text-amber-400",
    bg: "bg-amber-50 dark:bg-amber-900/20",
  },
  error: {
    icon: XCircle,
    color: "text-red-500 dark:text-red-400",
    bg: "bg-red-50 dark:bg-red-900/20",
  },
};

// ---------------------------------------------------------------------------
// LogEntryRow
// ---------------------------------------------------------------------------

function LogEntryRow({ entry }: { entry: LogEntry }) {
  const config = levelConfig[entry.level] ?? levelConfig.info;
  const Icon = config.icon;

  return (
    <div
      className={`flex items-start gap-2 px-3 py-1.5 border-b border-gray-100 dark:border-gray-700 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-700/50 ${config.bg}`}
      data-testid="event-log-entry"
    >
      {/* Level icon */}
      <Icon size={12} className={`mt-0.5 flex-shrink-0 ${config.color}`} data-testid="event-log-level-icon" />

      {/* Timestamp */}
      <span
        className="text-gray-400 text-[10px] flex-shrink-0 mt-px"
        data-testid="event-log-timestamp"
        title={formatTimestampFull(entry.timestamp)}
      >
        {formatTimestamp(entry.timestamp)}
      </span>

      {/* Event type badge */}
      <span
        className={`text-[10px] font-mono px-1 py-px rounded flex-shrink-0 mt-px ${config.color} bg-gray-100 dark:bg-gray-700`}
        data-testid="event-log-type"
      >
        {entry.type}
      </span>

      {/* Message */}
      <span
        className="text-gray-700 dark:text-gray-200 text-xs truncate"
        data-testid="event-log-message"
      >
        {entry.message}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// EventLogPanel
// ---------------------------------------------------------------------------

export function EventLogPanel() {
  const entries = useEventLogStore((s) => s.entries);
  const filter = useEventLogStore((s) => s.filter);
  const setFilter = useEventLogStore((s) => s.setFilter);
  const clearEntries = useEventLogStore((s) => s.clearEntries);

  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = [useRef(true)].map((r) =>
    // Using a ref for auto-scroll state to avoid re-renders
    r,
  );
  const autoScrollRef = useRef(true);

  // Derive filtered entries (reversed for newest-first)
  const filteredEntries = useMemo(() => {
    const filtered = selectFilteredEntries(useEventLogStore.getState());
    return [...filtered].reverse();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries, filter]);

  // Auto-scroll to bottom (newest entries) when new entries arrive
  useEffect(() => {
    if (autoScrollRef.current && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [filteredEntries.length]);

  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    // If user scrolls more than 30px away from the bottom, disable auto-scroll
    if (scrollHeight - scrollTop - clientHeight > 30) {
      autoScrollRef.current = false;
    } else {
      autoScrollRef.current = true;
    }
  }, []);

  return (
    <div
      className="flex flex-col h-full bg-white dark:bg-gray-800 overflow-hidden"
      data-testid="event-log-panel"
    >
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
        {/* Filter input */}
        <div className="flex items-center gap-1 bg-gray-100 dark:bg-gray-700 rounded px-2 py-0.5 flex-1 min-w-0">
          <Search size={12} className="text-gray-400 flex-shrink-0" />
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter events..."
            className="bg-transparent text-xs text-gray-700 dark:text-gray-200 placeholder-gray-400 outline-none w-full"
            data-testid="event-log-filter-input"
          />
        </div>

        {/* Entry count */}
        <span
          className="text-[10px] text-gray-400 flex-shrink-0"
          data-testid="event-log-count"
        >
          {filteredEntries.length > 0 && (
            <>
              {filteredEntries.length}
              {filter ? `/${entries.length}` : ""}
            </>
          )}
        </span>

        {/* Clear button */}
        <button
          type="button"
          onClick={clearEntries}
          className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors flex-shrink-0"
          title="Clear event log"
          data-testid="event-log-clear-btn"
        >
          <Trash2 size={12} className="text-gray-500" />
        </button>
      </div>

      {/* Log entry list */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto font-mono text-xs"
        data-testid="event-log-list"
      >
        {filteredEntries.length === 0 ? (
          <div
            className="p-4 text-gray-400 text-center"
            data-testid="event-log-empty"
          >
            No events logged
          </div>
        ) : (
          filteredEntries.map((entry) => (
            <LogEntryRow key={entry.id} entry={entry} />
          ))
        )}
      </div>
    </div>
  );
}

export default EventLogPanel;
