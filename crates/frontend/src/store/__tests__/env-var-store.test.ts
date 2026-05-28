import { describe, it, expect, beforeEach } from "vitest";
import { useEnvVarStore } from "../env-var-store";

// ---------------------------------------------------------------------------
// Reset helper
// ---------------------------------------------------------------------------

function resetStore() {
  useEnvVarStore.setState({ vars: [] });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useEnvVarStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with an empty vars array", () => {
      expect(useEnvVarStore.getState().vars).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // setVars
  // -----------------------------------------------------------------------

  describe("setVars()", () => {
    it("replaces the entire variable list", () => {
      useEnvVarStore.getState().setVars([
        { name: "API_KEY", value: "123", type: "str" },
        { name: "PORT", value: "8080", type: "num" },
      ]);
      expect(useEnvVarStore.getState().vars).toHaveLength(2);
      expect(useEnvVarStore.getState().vars[0]).toEqual({
        name: "API_KEY",
        value: "123",
        type: "str",
      });
    });

    it("overwrites previous variables", () => {
      useEnvVarStore.getState().setVars([{ name: "A", value: "1", type: "str" }]);
      useEnvVarStore.getState().setVars([{ name: "B", value: "2", type: "num" }]);
      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "B", value: "2", type: "num" },
      ]);
    });
  });

  // -----------------------------------------------------------------------
  // addVar
  // -----------------------------------------------------------------------

  describe("addVar()", () => {
    it("adds a variable with default type str", () => {
      useEnvVarStore.getState().addVar("HOST", "localhost");
      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "HOST", value: "localhost", type: "str" },
      ]);
    });

    it("adds a variable with explicit type", () => {
      useEnvVarStore.getState().addVar("PORT", "8080", "num");
      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "PORT", value: "8080", type: "num" },
      ]);
    });

    it("adds a bool variable", () => {
      useEnvVarStore.getState().addVar("DEBUG", "true", "bool");
      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "DEBUG", value: "true", type: "bool" },
      ]);
    });

    it("does not add a duplicate name", () => {
      useEnvVarStore.getState().addVar("X", "1");
      useEnvVarStore.getState().addVar("X", "2");
      expect(useEnvVarStore.getState().vars).toHaveLength(1);
      expect(useEnvVarStore.getState().vars[0].value).toBe("1");
    });
  });

  // -----------------------------------------------------------------------
  // removeVar
  // -----------------------------------------------------------------------

  describe("removeVar()", () => {
    it("removes a variable by name", () => {
      useEnvVarStore.getState().setVars([
        { name: "A", value: "1", type: "str" },
        { name: "B", value: "2", type: "num" },
      ]);
      useEnvVarStore.getState().removeVar("A");
      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "B", value: "2", type: "num" },
      ]);
    });

    it("is a no-op when name does not exist", () => {
      useEnvVarStore.getState().setVars([
        { name: "A", value: "1", type: "str" },
      ]);
      useEnvVarStore.getState().removeVar("NONEXISTENT");
      expect(useEnvVarStore.getState().vars).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // updateVar
  // -----------------------------------------------------------------------

  describe("updateVar()", () => {
    it("updates the value of an existing variable", () => {
      useEnvVarStore.getState().setVars([
        { name: "API_KEY", value: "old", type: "str" },
      ]);
      useEnvVarStore.getState().updateVar("API_KEY", { value: "new" });
      expect(useEnvVarStore.getState().vars[0].value).toBe("new");
    });

    it("updates the type of an existing variable", () => {
      useEnvVarStore.getState().setVars([
        { name: "PORT", value: "8080", type: "str" },
      ]);
      useEnvVarStore.getState().updateVar("PORT", { type: "num" });
      expect(useEnvVarStore.getState().vars[0].type).toBe("num");
    });

    it("updates both value and type simultaneously", () => {
      useEnvVarStore.getState().setVars([
        { name: "FLAG", value: "false", type: "str" },
      ]);
      useEnvVarStore.getState().updateVar("FLAG", { value: "true", type: "bool" });
      expect(useEnvVarStore.getState().vars[0]).toEqual({
        name: "FLAG",
        value: "true",
        type: "bool",
      });
    });

    it("does not modify other variables", () => {
      useEnvVarStore.getState().setVars([
        { name: "A", value: "1", type: "str" },
        { name: "B", value: "2", type: "num" },
      ]);
      useEnvVarStore.getState().updateVar("A", { value: "updated" });
      expect(useEnvVarStore.getState().vars[1]).toEqual({
        name: "B",
        value: "2",
        type: "num",
      });
    });
  });

  // -----------------------------------------------------------------------
  // getVar
  // -----------------------------------------------------------------------

  describe("getVar()", () => {
    it("returns the variable when found", () => {
      useEnvVarStore.getState().setVars([
        { name: "HOST", value: "localhost", type: "str" },
      ]);
      expect(useEnvVarStore.getState().getVar("HOST")).toEqual({
        name: "HOST",
        value: "localhost",
        type: "str",
      });
    });

    it("returns undefined when not found", () => {
      expect(useEnvVarStore.getState().getVar("NOPE")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // resolveValue
  // -----------------------------------------------------------------------

  describe("resolveValue()", () => {
    it("resolves a single ${VAR} reference", () => {
      useEnvVarStore.getState().setVars([
        { name: "HOST", value: "localhost", type: "str" },
      ]);
      expect(
        useEnvVarStore.getState().resolveValue("Server at ${HOST}"),
      ).toBe("Server at localhost");
    });

    it("resolves multiple different references", () => {
      useEnvVarStore.getState().setVars([
        { name: "HOST", value: "localhost", type: "str" },
        { name: "PORT", value: "8080", type: "num" },
      ]);
      expect(
        useEnvVarStore.getState().resolveValue("http://${HOST}:${PORT}"),
      ).toBe("http://localhost:8080");
    });

    it("resolves duplicate references", () => {
      useEnvVarStore.getState().setVars([
        { name: "X", value: "42", type: "num" },
      ]);
      expect(
        useEnvVarStore.getState().resolveValue("${X}-${X}"),
      ).toBe("42-42");
    });

    it("replaces unknown references with empty string", () => {
      expect(
        useEnvVarStore.getState().resolveValue("val=${UNKNOWN}"),
      ).toBe("val=");
    });

    it("returns the input unchanged when no references present", () => {
      expect(
        useEnvVarStore.getState().resolveValue("plain text"),
      ).toBe("plain text");
    });

    it("returns empty string for empty input", () => {
      expect(useEnvVarStore.getState().resolveValue("")).toBe("");
    });
  });
});
