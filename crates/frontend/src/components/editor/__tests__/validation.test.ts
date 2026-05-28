import { describe, it, expect } from "vitest";
import {
  validateNodeForm,
  validateField,
  errorsToMap,
  type ValidationError,
} from "../validation";
import type { NodeDefinition } from "@/red/nodes/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeDefinition(
  defaults: Record<string, { value: unknown; required?: boolean; validate?: string | ((value: unknown) => boolean); type?: string }>,
): NodeDefinition {
  return {
    id: "test-node",
    type: "test",
    name: "Test Node",
    category: "common",
    color: "#a6bbcf",
    defaults,
    inputs: 0,
    outputs: 1,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("validateNodeForm", () => {
  // -----------------------------------------------------------------------
  // Required field validation
  // -----------------------------------------------------------------------

  describe("required fields", () => {
    it("returns an error when a required field is empty string", () => {
      const def = makeDefinition({
        topic: { value: "", required: true },
      });
      const errors = validateNodeForm("test", { topic: "" }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0]).toEqual({
        field: "topic",
        message: "topic is required",
        type: "required",
      });
    });

    it("returns an error when a required field is undefined", () => {
      const def = makeDefinition({
        topic: { value: "", required: true },
      });
      const errors = validateNodeForm("test", {}, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("required");
    });

    it("returns an error when a required field is null", () => {
      const def = makeDefinition({
        topic: { value: "", required: true },
      });
      const errors = validateNodeForm("test", { topic: null }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("required");
    });

    it("returns no errors when a required field has a value", () => {
      const def = makeDefinition({
        topic: { value: "", required: true },
      });
      const errors = validateNodeForm("test", { topic: "sensor/data" }, def);
      expect(errors).toHaveLength(0);
    });

    it("does not flag non-required empty fields", () => {
      const def = makeDefinition({
        topic: { value: "", required: false },
      });
      const errors = validateNodeForm("test", { topic: "" }, def);
      expect(errors).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Type-specific validation
  // -----------------------------------------------------------------------

  describe("type-specific validation", () => {
    it("validates num type field with non-numeric value", () => {
      const def = makeDefinition({
        count: { value: "", type: "num" },
      });
      const errors = validateNodeForm("test", { count: "abc" }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("invalid");
      expect(errors[0].message).toContain("valid number");
    });

    it("passes num type field with valid number", () => {
      const def = makeDefinition({
        count: { value: "", type: "num" },
      });
      const errors = validateNodeForm("test", { count: "42" }, def);
      expect(errors).toHaveLength(0);
    });

    it("passes num type field when empty (not required)", () => {
      const def = makeDefinition({
        count: { value: "", type: "num" },
      });
      const errors = validateNodeForm("test", { count: "" }, def);
      expect(errors).toHaveLength(0);
    });

    it("validates json type field with invalid JSON", () => {
      const def = makeDefinition({
        config: { value: "", type: "json" },
      });
      const errors = validateNodeForm("test", { config: "{invalid}" }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("invalid");
      expect(errors[0].message).toContain("valid JSON");
    });

    it("passes json type field with valid JSON", () => {
      const def = makeDefinition({
        config: { value: "", type: "json" },
      });
      const errors = validateNodeForm("test", { config: '{"key": "value"}' }, def);
      expect(errors).toHaveLength(0);
    });

    it("passes json type field when empty (not required)", () => {
      const def = makeDefinition({
        config: { value: "", type: "json" },
      });
      const errors = validateNodeForm("test", { config: "" }, def);
      expect(errors).toHaveLength(0);
    });

    it("validates number default value with non-numeric input", () => {
      const def = makeDefinition({
        repeat: { value: 0 },
      });
      const errors = validateNodeForm("test", { repeat: "not-a-number" }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("invalid");
    });

    it("passes number default value with numeric input", () => {
      const def = makeDefinition({
        repeat: { value: 0 },
      });
      const errors = validateNodeForm("test", { repeat: 10 }, def);
      expect(errors).toHaveLength(0);
    });

    it("passes str type field (no type-specific validation)", () => {
      const def = makeDefinition({
        payload: { value: "", type: "str" },
      });
      const errors = validateNodeForm("test", { payload: "anything" }, def);
      expect(errors).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Custom validator
  // -----------------------------------------------------------------------

  describe("custom validators", () => {
    it("calls custom validate function and reports error when it returns false", () => {
      const def = makeDefinition({
        port: {
          value: 0,
          validate: (v: unknown) => {
            const n = Number(v);
            return n >= 0 && n <= 65535;
          },
        },
      });
      const errors = validateNodeForm("test", { port: 99999 }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("custom");
      expect(errors[0].message).toContain("invalid");
    });

    it("passes when custom validate function returns true", () => {
      const def = makeDefinition({
        port: {
          value: 0,
          validate: (v: unknown) => {
            const n = Number(v);
            return n >= 0 && n <= 65535;
          },
        },
      });
      const errors = validateNodeForm("test", { port: 8080 }, def);
      expect(errors).toHaveLength(0);
    });

    it("handles validate function that throws an error", () => {
      const def = makeDefinition({
        bad: {
          value: "",
          validate: () => {
            throw new Error("broken");
          },
        },
      });
      const errors = validateNodeForm("test", { bad: "test" }, def);
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("custom");
      expect(errors[0].message).toContain("failed validation");
    });

    it("ignores string validate (built-in validator name)", () => {
      const def = makeDefinition({
        broker: {
          value: "",
          validate: "MQTTBroker",
        },
      });
      const errors = validateNodeForm("test", { broker: "any" }, def);
      expect(errors).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Multiple fields
  // -----------------------------------------------------------------------

  describe("multiple fields", () => {
    it("reports errors for multiple invalid fields", () => {
      const def = makeDefinition({
        name: { value: "", required: true },
        topic: { value: "", required: true },
        count: { value: "", type: "num" },
      });
      const errors = validateNodeForm("test", { name: "", topic: "", count: "abc" }, def);
      expect(errors).toHaveLength(3);
    });

    it("reports only required error when a required field is empty (no type validation)", () => {
      const def = makeDefinition({
        count: { value: "", type: "num", required: true },
      });
      const errors = validateNodeForm("test", { count: "" }, def);
      // Should only report required, not also "not a valid number"
      expect(errors).toHaveLength(1);
      expect(errors[0].type).toBe("required");
    });
  });

  // -----------------------------------------------------------------------
  // No schema for field
  // -----------------------------------------------------------------------

  describe("fields without schema", () => {
    it("skips fields that are in formData but not in defaults schema", () => {
      const def = makeDefinition({
        name: { value: "" },
      });
      const errors = validateNodeForm("test", { name: "ok", extraField: "" }, def);
      expect(errors).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // Valid form
  // -----------------------------------------------------------------------

  describe("valid form", () => {
    it("returns empty array for a completely valid form", () => {
      const def = makeDefinition({
        name: { value: "", required: true },
        topic: { value: "" },
        count: { value: 0 },
        payload: { value: "", type: "str" },
      });
      const formData = { name: "My Node", topic: "sensors", count: 5, payload: "hello" };
      const errors = validateNodeForm("test", formData, def);
      expect(errors).toHaveLength(0);
    });
  });
});

// ---------------------------------------------------------------------------
// validateField (single field)
// ---------------------------------------------------------------------------

describe("validateField", () => {
  it("returns error for required empty field", () => {
    const result = validateField("topic", "", { value: "", required: true });
    expect(result).not.toBeNull();
    expect(result?.type).toBe("required");
  });

  it("returns null for valid field", () => {
    const result = validateField("topic", "hello", { value: "" });
    expect(result).toBeNull();
  });

  it("returns error for invalid number type", () => {
    const result = validateField("count", "abc", { value: "", type: "num" });
    expect(result).not.toBeNull();
    expect(result?.type).toBe("invalid");
  });

  it("returns error when custom validator fails", () => {
    const result = validateField("port", 99999, {
      value: 0,
      validate: (v: unknown) => Number(v) <= 65535,
    });
    expect(result).not.toBeNull();
    expect(result?.type).toBe("custom");
  });
});

// ---------------------------------------------------------------------------
// errorsToMap
// ---------------------------------------------------------------------------

describe("errorsToMap", () => {
  it("converts error array to record", () => {
    const errors: ValidationError[] = [
      { field: "name", message: "name is required", type: "required" },
      { field: "topic", message: "topic is required", type: "required" },
    ];
    const map = errorsToMap(errors);
    expect(map).toEqual({
      name: "name is required",
      topic: "topic is required",
    });
  });

  it("keeps only the first error per field", () => {
    const errors: ValidationError[] = [
      { field: "topic", message: "first error", type: "required" },
      { field: "topic", message: "second error", type: "invalid" },
    ];
    const map = errorsToMap(errors);
    expect(map).toEqual({ topic: "first error" });
  });

  it("returns empty object for empty array", () => {
    expect(errorsToMap([])).toEqual({});
  });
});
