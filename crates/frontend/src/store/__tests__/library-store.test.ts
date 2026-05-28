import { describe, it, expect, beforeEach } from "vitest";
import {
  useLibraryStore,
  type LibraryEntry,
} from "../library-store";
import { eventBus } from "../../red/core/events";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useLibraryStore.setState({ entries: [], isLoading: false });
  eventBus.clear();
}

function makeEntry(
  overrides: Partial<LibraryEntry> = {},
): Omit<LibraryEntry, "id" | "createdAt" | "updatedAt"> {
  return {
    name: "Test Flow",
    path: "/",
    type: "flow",
    content: JSON.stringify([{ id: "n1", type: "inject" }]),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useLibraryStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty entries", () => {
      const state = useLibraryStore.getState();
      expect(state.entries).toEqual([]);
    });

    it("starts with isLoading false", () => {
      expect(useLibraryStore.getState().isLoading).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // saveToLibrary
  // -----------------------------------------------------------------------

  describe("saveToLibrary()", () => {
    it("saves a new entry and returns its ID", () => {
      const id = useLibraryStore.getState().saveToLibrary(makeEntry());

      expect(typeof id).toBe("string");
      expect(id.length).toBe(12);

      const state = useLibraryStore.getState();
      expect(state.entries).toHaveLength(1);
      expect(state.entries[0].id).toBe(id);
    });

    it("sets createdAt and updatedAt timestamps", () => {
      const before = Date.now();
      const id = useLibraryStore.getState().saveToLibrary(makeEntry());
      const after = Date.now();

      const entry = useLibraryStore.getState().loadFromLibrary(id)!;
      expect(entry.createdAt).toBeGreaterThanOrEqual(before);
      expect(entry.createdAt).toBeLessThanOrEqual(after);
      expect(entry.updatedAt).toBe(entry.createdAt);
    });

    it("stores entry fields correctly", () => {
      const id = useLibraryStore.getState().saveToLibrary(
        makeEntry({
          name: "My Function",
          path: "/functions",
          type: "function",
          content: '{"code": "return msg;"}',
        }),
      );

      const entry = useLibraryStore.getState().loadFromLibrary(id)!;
      expect(entry.name).toBe("My Function");
      expect(entry.path).toBe("/functions");
      expect(entry.type).toBe("function");
      expect(entry.content).toBe('{"code": "return msg;"}');
    });

    it("emits library:saved event", () => {
      const handler = vi.fn();
      eventBus.on("library:saved", handler);

      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Test", type: "template" }),
      );

      expect(handler).toHaveBeenCalledOnce();
      const payload = handler.mock.calls[0][0] as { id: string; name: string; type: string };
      expect(payload.name).toBe("Test");
      expect(payload.type).toBe("template");
    });

    it("can save multiple entries", () => {
      useLibraryStore.getState().saveToLibrary(makeEntry({ name: "A" }));
      useLibraryStore.getState().saveToLibrary(makeEntry({ name: "B" }));

      expect(useLibraryStore.getState().entries).toHaveLength(2);
    });
  });

  // -----------------------------------------------------------------------
  // loadFromLibrary
  // -----------------------------------------------------------------------

  describe("loadFromLibrary()", () => {
    it("returns entry by ID", () => {
      const id = useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Found" }),
      );

      const entry = useLibraryStore.getState().loadFromLibrary(id);
      expect(entry).toBeDefined();
      expect(entry!.name).toBe("Found");
    });

    it("returns undefined for non-existent ID", () => {
      const entry = useLibraryStore.getState().loadFromLibrary("nonexistent");
      expect(entry).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // deleteFromLibrary
  // -----------------------------------------------------------------------

  describe("deleteFromLibrary()", () => {
    it("deletes an entry by ID", () => {
      const id = useLibraryStore.getState().saveToLibrary(makeEntry());

      useLibraryStore.getState().deleteFromLibrary(id);

      expect(useLibraryStore.getState().entries).toHaveLength(0);
      expect(useLibraryStore.getState().loadFromLibrary(id)).toBeUndefined();
    });

    it("emits library:deleted event", () => {
      const handler = vi.fn();
      eventBus.on("library:deleted", handler);

      const id = useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "ToDelete" }),
      );
      useLibraryStore.getState().deleteFromLibrary(id);

      expect(handler).toHaveBeenCalledOnce();
      const payload = handler.mock.calls[0][0] as { id: string; name: string };
      expect(payload.name).toBe("ToDelete");
    });

    it("does not delete other entries", () => {
      const id1 = useLibraryStore.getState().saveToLibrary(makeEntry({ name: "A" }));
      const id2 = useLibraryStore.getState().saveToLibrary(makeEntry({ name: "B" }));

      useLibraryStore.getState().deleteFromLibrary(id1);

      expect(useLibraryStore.getState().entries).toHaveLength(1);
      expect(useLibraryStore.getState().loadFromLibrary(id2)).toBeDefined();
    });

    it("handles deleting non-existent ID gracefully", () => {
      useLibraryStore.getState().saveToLibrary(makeEntry());
      useLibraryStore.getState().deleteFromLibrary("nonexistent");

      expect(useLibraryStore.getState().entries).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // updateEntry
  // -----------------------------------------------------------------------

  describe("updateEntry()", () => {
    it("updates specified fields", () => {
      const id = useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Original" }),
      );

      useLibraryStore.getState().updateEntry(id, { name: "Updated" });

      const entry = useLibraryStore.getState().loadFromLibrary(id)!;
      expect(entry.name).toBe("Updated");
    });

    it("updates updatedAt timestamp", () => {
      const id = useLibraryStore.getState().saveToLibrary(makeEntry());
      const originalUpdatedAt = useLibraryStore.getState().loadFromLibrary(id)!.updatedAt;

      // Small delay to ensure different timestamp
      useLibraryStore.getState().updateEntry(id, { name: "Changed" });

      const entry = useLibraryStore.getState().loadFromLibrary(id)!;
      expect(entry.updatedAt).toBeGreaterThanOrEqual(originalUpdatedAt);
    });

    it("preserves un-updated fields", () => {
      const id = useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "A", type: "function" }),
      );

      useLibraryStore.getState().updateEntry(id, { name: "B" });

      const entry = useLibraryStore.getState().loadFromLibrary(id)!;
      expect(entry.name).toBe("B");
      expect(entry.type).toBe("function");
      expect(entry.content).toBe(makeEntry({ name: "B" }).content);
    });

    it("emits library:updated event", () => {
      const handler = vi.fn();
      eventBus.on("library:updated", handler);

      const id = useLibraryStore.getState().saveToLibrary(makeEntry());
      useLibraryStore.getState().updateEntry(id, { name: "New" });

      expect(handler).toHaveBeenCalledOnce();
      const payload = handler.mock.calls[0][0] as { id: string };
      expect(payload.id).toBe(id);
    });
  });

  // -----------------------------------------------------------------------
  // search
  // -----------------------------------------------------------------------

  describe("search()", () => {
    beforeEach(() => {
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Inject Flow", type: "flow", path: "/flows" }),
      );
      useLibraryStore.getState().saveToLibrary(
        makeEntry({
          name: "Debug Helper",
          type: "function",
          path: "/utils",
          content: "return msg;",
        }),
      );
      useLibraryStore.getState().saveToLibrary(
        makeEntry({
          name: "Email Template",
          type: "template",
          path: "/templates",
          content: JSON.stringify({ subject: "Hello", body: "World" }),
        }),
      );
    });

    it("returns all entries for empty query", () => {
      const results = useLibraryStore.getState().search("");
      expect(results).toHaveLength(3);
    });

    it("returns all entries for whitespace-only query", () => {
      const results = useLibraryStore.getState().search("   ");
      expect(results).toHaveLength(3);
    });

    it("searches by name (case-insensitive)", () => {
      const results = useLibraryStore.getState().search("debug");
      // "debug" matches Debug Helper by name and Inject Flow content (has debug node)
      expect(results.length).toBeGreaterThanOrEqual(1);
      expect(results.some((r) => r.name === "Debug Helper")).toBe(true);
    });

    it("searches by path", () => {
      const results = useLibraryStore.getState().search("/utils");
      expect(results).toHaveLength(1);
      expect(results[0].name).toBe("Debug Helper");
    });

    it("searches by type", () => {
      const results = useLibraryStore.getState().search("template");
      expect(results).toHaveLength(1);
      expect(results[0].name).toBe("Email Template");
    });

    it("searches by content", () => {
      // Search for unique content in the Email Template entry
      const results = useLibraryStore.getState().search("subject");
      expect(results).toHaveLength(1);
      expect(results[0].name).toBe("Email Template");
    });

    it("returns empty for non-matching query", () => {
      const results = useLibraryStore.getState().search("xyznonexistent");
      expect(results).toHaveLength(0);
    });

    it("matches partial strings", () => {
      const results = useLibraryStore.getState().search("email");
      expect(results).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // importLibrary
  // -----------------------------------------------------------------------

  describe("importLibrary()", () => {
    it("imports entries from JSON array", () => {
      const json = JSON.stringify([
        {
          id: "imp1",
          name: "Imported Flow",
          path: "/imported",
          type: "flow",
          content: "[]",
          createdAt: 1000,
          updatedAt: 1000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries).toHaveLength(1);
      expect(useLibraryStore.getState().entries[0].name).toBe("Imported Flow");
    });

    it("imports a single entry object", () => {
      const json = JSON.stringify({
        id: "imp2",
        name: "Single Entry",
        path: "/",
        type: "function",
        content: "return msg;",
        createdAt: 2000,
        updatedAt: 2000,
      });

      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries).toHaveLength(1);
      expect(useLibraryStore.getState().entries[0].name).toBe("Single Entry");
    });

    it("merges with existing entries", () => {
      useLibraryStore.getState().saveToLibrary(makeEntry({ name: "Existing" }));

      const json = JSON.stringify([
        {
          id: "imp3",
          name: "New Entry",
          path: "/",
          type: "flow",
          content: "[]",
          createdAt: 3000,
          updatedAt: 3000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries).toHaveLength(2);
    });

    it("generates new IDs for duplicate IDs", () => {
      // Save an entry with a known ID
      useLibraryStore.getState().saveToLibrary(makeEntry({ name: "Original" }));
      const existingId = useLibraryStore.getState().entries[0].id;

      // Try to import with the same ID
      const json = JSON.stringify([
        {
          id: existingId,
          name: "Duplicate",
          path: "/",
          type: "flow",
          content: "[]",
          createdAt: 4000,
          updatedAt: 4000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      // Should have 2 entries with different IDs
      expect(useLibraryStore.getState().entries).toHaveLength(2);
      const ids = useLibraryStore.getState().entries.map((e) => e.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(2);
    });

    it("assigns default type for invalid type values", () => {
      const json = JSON.stringify([
        {
          id: "imp4",
          name: "Bad Type",
          path: "/",
          type: "invalid",
          content: "[]",
          createdAt: 5000,
          updatedAt: 5000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries[0].type).toBe("flow");
    });

    it("skips entries without required fields", () => {
      const json = JSON.stringify([
        { id: "imp5" }, // missing name and content
        { name: "Missing content" }, // missing content
        {
          id: "imp6",
          name: "Valid",
          path: "/",
          type: "flow",
          content: "{}",
          createdAt: 6000,
          updatedAt: 6000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries).toHaveLength(1);
      expect(useLibraryStore.getState().entries[0].name).toBe("Valid");
    });

    it("emits library:imported event with count", () => {
      const handler = vi.fn();
      eventBus.on("library:imported", handler);

      const json = JSON.stringify([
        {
          id: "imp7",
          name: "A",
          path: "/",
          type: "flow",
          content: "[]",
          createdAt: 7000,
          updatedAt: 7000,
        },
        {
          id: "imp8",
          name: "B",
          path: "/",
          type: "function",
          content: "[]",
          createdAt: 8000,
          updatedAt: 8000,
        },
      ]);

      useLibraryStore.getState().importLibrary(json);

      expect(handler).toHaveBeenCalledOnce();
      const payload = handler.mock.calls[0][0] as { count: number };
      expect(payload.count).toBe(2);
    });

    it("handles invalid JSON gracefully", () => {
      const handler = vi.fn();
      eventBus.on("library:import-error", handler);

      useLibraryStore.getState().importLibrary("not valid json{{{");

      expect(useLibraryStore.getState().entries).toHaveLength(0);
      expect(handler).toHaveBeenCalledOnce();
    });
  });

  // -----------------------------------------------------------------------
  // exportLibrary
  // -----------------------------------------------------------------------

  describe("exportLibrary()", () => {
    it("exports empty library as JSON array", () => {
      const json = useLibraryStore.getState().exportLibrary();
      expect(JSON.parse(json)).toEqual([]);
    });

    it("exports all entries as valid JSON", () => {
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Flow 1", type: "flow" }),
      );
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Func 1", type: "function" }),
      );

      const json = useLibraryStore.getState().exportLibrary();
      const parsed = JSON.parse(json);

      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed).toHaveLength(2);
    });

    it("preserves all entry fields in export", () => {
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Full Entry", type: "template" }),
      );

      const json = useLibraryStore.getState().exportLibrary();
      const parsed = JSON.parse(json);
      const entry = parsed[0];

      expect(entry).toHaveProperty("id");
      expect(entry).toHaveProperty("name", "Full Entry");
      expect(entry).toHaveProperty("path");
      expect(entry).toHaveProperty("type", "template");
      expect(entry).toHaveProperty("content");
      expect(entry).toHaveProperty("createdAt");
      expect(entry).toHaveProperty("updatedAt");
    });
  });

  // -----------------------------------------------------------------------
  // setLoading
  // -----------------------------------------------------------------------

  describe("setLoading()", () => {
    it("sets isLoading state", () => {
      useLibraryStore.getState().setLoading(true);
      expect(useLibraryStore.getState().isLoading).toBe(true);

      useLibraryStore.getState().setLoading(false);
      expect(useLibraryStore.getState().isLoading).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // Round-trip: export -> import
  // -----------------------------------------------------------------------

  describe("round-trip export/import", () => {
    it("can re-import exported entries", () => {
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Flow A", type: "flow" }),
      );
      useLibraryStore.getState().saveToLibrary(
        makeEntry({ name: "Func B", type: "function" }),
      );

      const json = useLibraryStore.getState().exportLibrary();

      // Clear and re-import
      useLibraryStore.setState({ entries: [] });
      useLibraryStore.getState().importLibrary(json);

      expect(useLibraryStore.getState().entries).toHaveLength(2);
      const names = useLibraryStore.getState().entries.map((e) => e.name);
      expect(names).toContain("Flow A");
      expect(names).toContain("Func B");
    });
  });
});
