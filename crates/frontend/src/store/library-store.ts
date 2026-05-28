/**
 * Library store — replaces Node-RED's RED.library (~890 lines).
 *
 * A Zustand store with localStorage persistence for flow/function/template
 * library entries. Provides save/load/delete/update/search/import/export.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";
import { eventBus } from "../red/core/events";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LibraryEntry {
  id: string;
  name: string;
  path: string;
  type: "flow" | "function" | "template";
  content: string; // JSON string
  createdAt: number;
  updatedAt: number;
}

export interface LibraryState {
  entries: LibraryEntry[];
  isLoading: boolean;

  /** Save a new entry to the library. Returns the generated ID. */
  saveToLibrary: (
    entry: Omit<LibraryEntry, "id" | "createdAt" | "updatedAt">,
  ) => string;

  /** Load an entry by ID. Returns undefined if not found. */
  loadFromLibrary: (id: string) => LibraryEntry | undefined;

  /** Delete an entry by ID. */
  deleteFromLibrary: (id: string) => void;

  /** Update an existing entry by ID. Only the supplied fields are changed. */
  updateEntry: (id: string, updates: Partial<LibraryEntry>) => void;

  /** Search entries by name, path, type, or content (case-insensitive). */
  search: (query: string) => LibraryEntry[];

  /** Bulk import entries from a JSON string (merges, does not replace). */
  importLibrary: (json: string) => void;

  /** Export all entries as a JSON string. */
  exportLibrary: () => string;

  /** Set loading state (used internally during hydration). */
  setLoading: (loading: boolean) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Generate a random hex string of the given length.
 */
function generateId(length = 12): string {
  const chars = "0123456789abcdef";
  let result = "";
  for (let i = 0; i < length; i++) {
    result += chars[Math.floor(Math.random() * chars.length)];
  }
  return result;
}

/**
 * Check if a string contains a query (case-insensitive).
 */
function matchesQuery(text: string, query: string): boolean {
  return text.toLowerCase().includes(query.toLowerCase());
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useLibraryStore = create<LibraryState>()(
  persist(
    (set, get) => ({
      entries: [],
      isLoading: false,

      saveToLibrary: (
        entry: Omit<LibraryEntry, "id" | "createdAt" | "updatedAt">,
      ): string => {
        const id = generateId(12);
        const now = Date.now();
        const newEntry: LibraryEntry = {
          ...entry,
          id,
          createdAt: now,
          updatedAt: now,
        };

        set((state) => ({
          entries: [...state.entries, newEntry],
        }));

        eventBus.emit("library:saved", { id, name: entry.name, type: entry.type });

        return id;
      },

      loadFromLibrary: (id: string): LibraryEntry | undefined => {
        return get().entries.find((e) => e.id === id);
      },

      deleteFromLibrary: (id: string): void => {
        const entry = get().entries.find((e) => e.id === id);
        set((state) => ({
          entries: state.entries.filter((e) => e.id !== id),
        }));

        if (entry) {
          eventBus.emit("library:deleted", { id, name: entry.name, type: entry.type });
        }
      },

      updateEntry: (id: string, updates: Partial<LibraryEntry>): void => {
        set((state) => ({
          entries: state.entries.map((e) =>
            e.id === id
              ? { ...e, ...updates, updatedAt: Date.now() }
              : e,
          ),
        }));

        eventBus.emit("library:updated", { id });
      },

      search: (query: string): LibraryEntry[] => {
        const trimmed = query.trim();
        if (trimmed.length === 0) return get().entries;

        return get().entries.filter(
          (e) =>
            matchesQuery(e.name, trimmed) ||
            matchesQuery(e.path, trimmed) ||
            matchesQuery(e.type, trimmed) ||
            matchesQuery(e.content, trimmed),
        );
      },

      importLibrary: (json: string): void => {
        try {
          const parsed = JSON.parse(json);
          const incomingEntries: LibraryEntry[] = Array.isArray(parsed)
            ? parsed
            : [parsed];

          const now = Date.now();
          const existingIds = new Set(get().entries.map((e) => e.id));

          // Filter out duplicates by ID, assign new IDs to avoid collisions
          const newEntries: LibraryEntry[] = incomingEntries
            .filter((entry) => {
              // Basic validation
              return (
                entry &&
                typeof entry.name === "string" &&
                typeof entry.content === "string"
              );
            })
            .map((entry) => ({
              ...entry,
              id: existingIds.has(entry.id) ? generateId(12) : entry.id,
              type: ["flow", "function", "template"].includes(entry.type)
                ? (entry.type as LibraryEntry["type"])
                : "flow",
              createdAt: entry.createdAt ?? now,
              updatedAt: now,
            }));

          set((state) => ({
            entries: [...state.entries, ...newEntries],
          }));

          eventBus.emit("library:imported", { count: newEntries.length });
        } catch {
          eventBus.emit("library:import-error", { error: "Invalid JSON" });
        }
      },

      exportLibrary: (): string => {
        return JSON.stringify(get().entries, null, 2);
      },

      setLoading: (loading: boolean): void => {
        set({ isLoading: loading });
      },
    }),
    {
      name: "rust-red-library",
      // Only persist the entries array
      partialize: (state) => ({ entries: state.entries }) as LibraryState,
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.isLoading = false;
        }
      },
    },
  ),
);

export default useLibraryStore;
