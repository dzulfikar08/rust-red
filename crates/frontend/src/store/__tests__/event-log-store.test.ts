import { describe, it, expect, beforeEach } from "vitest";
import { useEventLogStore } from "../event-log-store";

function resetStore() {
  useEventLogStore.setState({
    entries: [],
    filter: "",
    maxEntries: 200,
  });
}

describe("useEventLogStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty entries", () => {
      expect(useEventLogStore.getState().entries).toEqual([]);
    });

    it("starts with empty filter", () => {
      expect(useEventLogStore.getState().filter).toBe("");
    });

    it("has default maxEntries of 200", () => {
      expect(useEventLogStore.getState().maxEntries).toBe(200);
    });
  });

  // -----------------------------------------------------------------------
  // addEntry
  // -----------------------------------------------------------------------

  describe("addEntry()", () => {
    it("adds an entry and generates id and timestamp", () => {
      useEventLogStore.getState().addEntry({
        type: "nodes:added",
        message: "Node added",
        level: "info",
      });

      const { entries } = useEventLogStore.getState();
      expect(entries).toHaveLength(1);
      expect(entries[0].id).toBeTruthy();
      expect(entries[0].timestamp).toBeTypeOf("number");
      expect(entries[0].type).toBe("nodes:added");
      expect(entries[0].message).toBe("Node added");
      expect(entries[0].level).toBe("info");
    });

    it("preserves optional data field", () => {
      useEventLogStore.getState().addEntry({
        type: "deploy:error",
        message: "Deploy failed",
        level: "error",
        data: { error: "connection refused" },
      });

      const entry = useEventLogStore.getState().entries[0];
      expect(entry.data).toEqual({ error: "connection refused" });
    });

    it("appends entries in chronological order", () => {
      useEventLogStore.getState().addEntry({
        type: "nodes:added",
        message: "First",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "nodes:removed",
        message: "Second",
        level: "info",
      });

      const { entries } = useEventLogStore.getState();
      expect(entries).toHaveLength(2);
      expect(entries[0].message).toBe("First");
      expect(entries[1].message).toBe("Second");
    });

    it("enforces maxEntries limit", () => {
      useEventLogStore.setState({ maxEntries: 3 });

      for (let i = 0; i < 5; i++) {
        useEventLogStore.getState().addEntry({
          type: "test",
          message: `Entry ${i}`,
          level: "info",
        });
      }

      const { entries } = useEventLogStore.getState();
      expect(entries).toHaveLength(3);
      // Should keep the last 3 entries
      expect(entries[0].message).toBe("Entry 2");
      expect(entries[1].message).toBe("Entry 3");
      expect(entries[2].message).toBe("Entry 4");
    });

    it("works without data field", () => {
      useEventLogStore.getState().addEntry({
        type: "workspace:changed",
        message: "Workspace changed",
        level: "info",
      });

      const entry = useEventLogStore.getState().entries[0];
      expect(entry.data).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // clearEntries
  // -----------------------------------------------------------------------

  describe("clearEntries()", () => {
    it("removes all entries", () => {
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "A",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "test",
        message: "B",
        level: "info",
      });

      useEventLogStore.getState().clearEntries();
      expect(useEventLogStore.getState().entries).toHaveLength(0);
    });

    it("works on an already-empty list", () => {
      useEventLogStore.getState().clearEntries();
      expect(useEventLogStore.getState().entries).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // setFilter / getFiltered
  // -----------------------------------------------------------------------

  describe("setFilter() and getFiltered()", () => {
    beforeEach(() => {
      useEventLogStore.getState().addEntry({
        type: "nodes:added",
        message: "Added debug node",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "deploy:success",
        message: "Deploy succeeded",
        level: "info",
      });
      useEventLogStore.getState().addEntry({
        type: "deploy:error",
        message: "Deploy failed: connection refused",
        level: "error",
      });
    });

    it("returns all entries when filter is empty", () => {
      const filtered = useEventLogStore.getState().getFiltered();
      expect(filtered).toHaveLength(3);
    });

    it("filters by type (case-insensitive)", () => {
      useEventLogStore.getState().setFilter("DEPLOY");
      const filtered = useEventLogStore.getState().getFiltered();
      expect(filtered).toHaveLength(2);
      expect(filtered.every((e) => e.type.includes("deploy"))).toBe(true);
    });

    it("filters by message content", () => {
      useEventLogStore.getState().setFilter("failed");
      const filtered = useEventLogStore.getState().getFiltered();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].type).toBe("deploy:error");
    });

    it("returns empty array when nothing matches", () => {
      useEventLogStore.getState().setFilter("nonexistent");
      const filtered = useEventLogStore.getState().getFiltered();
      expect(filtered).toHaveLength(0);
    });

    it("updates filter state", () => {
      useEventLogStore.getState().setFilter("test-filter");
      expect(useEventLogStore.getState().filter).toBe("test-filter");
    });
  });
});
