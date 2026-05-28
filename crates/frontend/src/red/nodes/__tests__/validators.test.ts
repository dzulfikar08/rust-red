import { describe, it, expect } from "vitest";
import { validateNodeProperty, validateNode, isRequired } from "../validators";
import type { NodeDefinition, FlowNode, NodeDefault } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeDefinition(
  defaults: Record<string, NodeDefault> = {},
): NodeDefinition {
  return {
    id: "test",
    type: "test",
    name: "Test",
    category: "test",
    color: "#fff",
    defaults,
    inputs: 1,
    outputs: 1,
  };
}

function makeNode(props: Record<string, unknown> = {}): FlowNode {
  return {
    id: "n1",
    type: "test",
    x: 0,
    y: 0,
    z: "tab-1",
    wires: [],
    ...props,
  };
}

// ---------------------------------------------------------------------------
// isRequired
// ---------------------------------------------------------------------------

describe("isRequired", () => {
  it("returns true when required is true", () => {
    expect(isRequired({ value: "", required: true })).toBe(true);
  });

  it("returns false when required is false", () => {
    expect(isRequired({ value: "", required: false })).toBe(false);
  });

  it("returns false when required is undefined", () => {
    expect(isRequired({ value: "" })).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// validateNodeProperty
// ---------------------------------------------------------------------------

describe("validateNodeProperty", () => {
  it("returns true for unknown properties (no spec)", () => {
    const def = makeDefinition();
    const node = makeNode();
    expect(validateNodeProperty(node, def, "nonexistent")).toBe(true);
  });

  it("returns true when required and value is present", () => {
    const def = makeDefinition({ name: { value: "", required: true } });
    const node = makeNode({ name: "Hello" });
    expect(validateNodeProperty(node, def, "name")).toBe(true);
  });

  it("returns false when required and value is empty string", () => {
    const def = makeDefinition({ name: { value: "", required: true } });
    const node = makeNode({ name: "" });
    expect(validateNodeProperty(node, def, "name")).toBe(false);
  });

  it("returns false when required and value is undefined", () => {
    const def = makeDefinition({ name: { value: "", required: true } });
    const node = makeNode(); // no name
    expect(validateNodeProperty(node, def, "name")).toBe(false);
  });

  it("returns false when required and value is null", () => {
    const def = makeDefinition({ name: { value: "", required: true } });
    const node = makeNode({ name: null });
    expect(validateNodeProperty(node, def, "name")).toBe(false);
  });

  it("returns true when not required and value is empty", () => {
    const def = makeDefinition({ name: { value: "" } });
    const node = makeNode({ name: "" });
    expect(validateNodeProperty(node, def, "name")).toBe(true);
  });

  it("uses validate function when provided", () => {
    const def = makeDefinition({
      port: { value: 0, validate: (v) => typeof v === "number" && v > 0 && v < 65536 },
    });
    const valid = makeNode({ port: 8080 });
    const invalid = makeNode({ port: -1 });

    expect(validateNodeProperty(valid, def, "port")).toBe(true);
    expect(validateNodeProperty(invalid, def, "port")).toBe(false);
  });

  it("passes when validate function returns true even if required is set", () => {
    const def = makeDefinition({
      url: {
        value: "",
        required: true,
        validate: (v) => typeof v === "string" && v.startsWith("/"),
      },
    });
    const valid = makeNode({ url: "/api/test" });
    const invalid = makeNode({ url: "no-slash" });

    expect(validateNodeProperty(valid, def, "url")).toBe(true);
    expect(validateNodeProperty(invalid, def, "url")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// validateNode
// ---------------------------------------------------------------------------

describe("validateNode", () => {
  it("returns valid:true for an empty defaults map", () => {
    const def = makeDefinition();
    const node = makeNode();
    const result = validateNode(node, def);
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual({});
  });

  it("returns valid:true when all properties pass", () => {
    const def = makeDefinition({
      name: { value: "", required: true },
      count: { value: 0 },
    });
    const node = makeNode({ name: "Test", count: 5 });
    const result = validateNode(node, def);
    expect(result.valid).toBe(true);
    expect(Object.keys(result.errors)).toHaveLength(0);
  });

  it("reports errors for missing required fields", () => {
    const def = makeDefinition({
      name: { value: "", required: true },
      url: { value: "", required: true },
    });
    const node = makeNode({ name: "" }); // url missing, name empty
    const result = validateNode(node, def);

    expect(result.valid).toBe(false);
    expect(result.errors.name).toBe("name is required");
    expect(result.errors.url).toBe("url is required");
  });

  it("reports errors from validate functions", () => {
    const def = makeDefinition({
      port: {
        value: 0,
        validate: (v) => typeof v === "number" && v > 0,
      },
    });
    const node = makeNode({ port: 0 });
    const result = validateNode(node, def);

    expect(result.valid).toBe(false);
    expect(result.errors.port).toBe("port is invalid");
  });

  it("does not report error when validate passes", () => {
    const def = makeDefinition({
      port: {
        value: 0,
        validate: (v) => typeof v === "number" && v > 0,
      },
    });
    const node = makeNode({ port: 8080 });
    const result = validateNode(node, def);
    expect(result.valid).toBe(true);
  });

  it("stops at required check failure (does not call validate)", () => {
    const validateFn = (v: unknown): boolean => {
      // This should not be called because required check fails first
      throw new Error("validate should not be called");
    };

    const def = makeDefinition({
      url: { value: "", required: true, validate: validateFn },
    });
    const node = makeNode({ url: "" });
    const result = validateNode(node, def);

    expect(result.valid).toBe(false);
    expect(result.errors.url).toBe("url is required");
  });
});
