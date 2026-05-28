/**
 * Event Log store -- replaces Node-RED's RED.eventLog.
 *
 * A Zustand store for managing a chronological log of application events
 * captured from the event bus. Supports filtering, max-entry limits, and
 * level-based categorization.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LogEntry {
  id: string;
  timestamp: number;
  /** Event type (e.g. nodes:added, flow:switched, deploy:success) */
  type: string;
  message: string;
  level: "info" | "warn" | "error";
  data?: Record<string, unknown>;
}

export type LogEntryInput = Omit<LogEntry, "id" | "timestamp">;

export interface EventLogStore {
  entries: LogEntry[];
  filter: string;
  maxEntries: number;

  addEntry: (input: LogEntryInput) => void;
  clearEntries: () => void;
  setFilter: (filter: string) => void;
  getFiltered: () => LogEntry[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let nextId = 0;
function uid(): string {
  return `elog-${Date.now()}-${++nextId}`;
}

// ---------------------------------------------------------------------------
// Selector helpers (exported for use in components)
// ---------------------------------------------------------------------------

export function selectFilteredEntries(state: EventLogStore): LogEntry[] {
  const { entries, filter } = state;
  if (!filter) return entries;
  const lower = filter.toLowerCase();
  return entries.filter(
    (e) =>
      e.type.toLowerCase().includes(lower) ||
      e.message.toLowerCase().includes(lower),
  );
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useEventLogStore = create<EventLogStore>()((set, get) => ({
  entries: [],
  filter: "",
  maxEntries: 200,

  addEntry: (input) => {
    const entry: LogEntry = {
      ...input,
      id: uid(),
      timestamp: Date.now(),
    };
    set((state) => {
      const next = [...state.entries, entry];
      // Enforce max entries limit
      if (next.length > state.maxEntries) {
        return { entries: next.slice(next.length - state.maxEntries) };
      }
      return { entries: next };
    });
  },

  clearEntries: () => {
    set({ entries: [] });
  },

  setFilter: (filter) => {
    set({ filter });
  },

  getFiltered: () => {
    return selectFilteredEntries(get());
  },
}));

export default useEventLogStore;
