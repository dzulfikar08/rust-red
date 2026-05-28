import { describe, it, expect, beforeEach, vi } from "vitest";
import { useContextStore } from "../context-store";

// ---------------------------------------------------------------------------
// Reset helper
// ---------------------------------------------------------------------------

function resetStore() {
  useContextStore.setState({
    flowContext: null,
    globalContext: null,
    isLoading: false,
    error: null,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useContextStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with null flow context", () => {
      expect(useContextStore.getState().flowContext).toBeNull();
    });

    it("starts with null global context", () => {
      expect(useContextStore.getState().globalContext).toBeNull();
    });

    it("is not loading initially", () => {
      expect(useContextStore.getState().isLoading).toBe(false);
    });

    it("has no error initially", () => {
      expect(useContextStore.getState().error).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // setContext
  // -----------------------------------------------------------------------

  describe("setContext()", () => {
    it("sets flow and global context", () => {
      const flow = { counter: 42 };
      const global = { version: "1.0" };
      useContextStore.getState().setContext(flow, global);
      expect(useContextStore.getState().flowContext).toEqual({ counter: 42 });
      expect(useContextStore.getState().globalContext).toEqual({
        version: "1.0",
      });
    });

    it("overwrites previous context", () => {
      useContextStore.getState().setContext({ a: 1 }, { b: 2 });
      useContextStore.getState().setContext({ c: 3 }, { d: 4 });
      expect(useContextStore.getState().flowContext).toEqual({ c: 3 });
      expect(useContextStore.getState().globalContext).toEqual({ d: 4 });
    });

    it("clears error when setting context", () => {
      useContextStore.setState({ error: "something went wrong" });
      useContextStore.getState().setContext({}, {});
      expect(useContextStore.getState().error).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // refresh -- API success
  // -----------------------------------------------------------------------

  describe("refresh()", () => {
    it("sets loading state during fetch", async () => {
      // Make fetch hang forever so we can observe the loading state
      vi.spyOn(globalThis, "fetch").mockImplementation(
        () => new Promise(() => {}),
      );

      const promise = useContextStore.getState().refresh();
      expect(useContextStore.getState().isLoading).toBe(true);

      // Let it settle (it won't, but we need to return the promise)
      // Use a timeout to avoid hanging the test suite
      setTimeout(() => promise, 50);
    });

    it("loads context from API on success", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ flow: { x: 1 }, global: { y: 2 } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      await useContextStore.getState().refresh();

      expect(useContextStore.getState().isLoading).toBe(false);
      expect(useContextStore.getState().flowContext).toEqual({ x: 1 });
      expect(useContextStore.getState().globalContext).toEqual({ y: 2 });

      vi.restoreAllMocks();
    });

    it("falls back to mock data when API fails", async () => {
      vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("Network error"));

      await useContextStore.getState().refresh();

      expect(useContextStore.getState().isLoading).toBe(false);
      // Mock data should be loaded
      expect(useContextStore.getState().flowContext).not.toBeNull();
      expect(useContextStore.getState().globalContext).not.toBeNull();
      expect(useContextStore.getState().flowContext).toHaveProperty("counter");
      expect(useContextStore.getState().globalContext).toHaveProperty("startTime");

      vi.restoreAllMocks();
    });

    it("handles API returning non-object context gracefully", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ flow: "not-an-object", global: 42 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      await useContextStore.getState().refresh();

      expect(useContextStore.getState().flowContext).toBeNull();
      expect(useContextStore.getState().globalContext).toBeNull();

      vi.restoreAllMocks();
    });
  });
});
