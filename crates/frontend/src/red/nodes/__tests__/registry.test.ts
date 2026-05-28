import { describe, it, expect, beforeEach } from "vitest";
import { NodeRegistry, nodeRegistry } from "../registry";
import type { NodeDefinition, FlowNode } from "../types";
import { registerBuiltinNodes, getBuiltinNodeList } from "../builtin-nodes";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeDef(overrides: Partial<NodeDefinition> = {}): NodeDefinition {
  return {
    id: "test",
    type: "test",
    name: "Test Node",
    category: "test-category",
    color: "#ff0000",
    defaults: {},
    inputs: 1,
    outputs: 1,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("NodeRegistry", () => {
  let registry: NodeRegistry;

  beforeEach(() => {
    registry = new NodeRegistry();
  });

  // ---- register / get ---------------------------------------------------

  describe("registerType / getType", () => {
    it("registers and retrieves a type", () => {
      const def = makeDef({ type: "inject" });
      registry.registerType("inject", def);
      expect(registry.getType("inject")).toBe(def);
    });

    it("returns undefined for unregistered types", () => {
      expect(registry.getType("nonexistent")).toBeUndefined();
    });

    it("overwrites an existing type", () => {
      const v1 = makeDef({ type: "inject", color: "#111" });
      const v2 = makeDef({ type: "inject", color: "#222" });
      registry.registerType("inject", v1);
      registry.registerType("inject", v2);
      expect(registry.getType("inject")!.color).toBe("#222");
    });
  });

  // ---- remove -----------------------------------------------------------

  describe("removeType", () => {
    it("removes a registered type", () => {
      registry.registerType("test", makeDef({ type: "test" }));
      registry.removeType("test");
      expect(registry.getType("test")).toBeUndefined();
    });

    it("is a no-op for unregistered types", () => {
      expect(() => registry.removeType("nope")).not.toThrow();
    });
  });

  // ---- clear ------------------------------------------------------------

  describe("clear", () => {
    it("removes all types", () => {
      registry.registerType("a", makeDef({ type: "a" }));
      registry.registerType("b", makeDef({ type: "b" }));
      registry.clear();
      expect(registry.getType("a")).toBeUndefined();
      expect(registry.getType("b")).toBeUndefined();
    });
  });

  // ---- getAllTypes -------------------------------------------------------

  describe("getAllTypes", () => {
    it("returns a copy of the internal map", () => {
      registry.registerType("a", makeDef({ type: "a" }));
      const map = registry.getAllTypes();
      expect(map.size).toBe(1);
      // Mutating the copy should not affect the registry
      map.delete("a");
      expect(registry.getType("a")).toBeDefined();
    });
  });

  // ---- categories -------------------------------------------------------

  describe("getTypesByCategory / getCategories", () => {
    beforeEach(() => {
      registry.registerType("inject", makeDef({ type: "inject", category: "common" }));
      registry.registerType("debug", makeDef({ type: "debug", category: "common" }));
      registry.registerType("function", makeDef({ type: "function", category: "function" }));
      registry.registerType("httpin", makeDef({ type: "httpin", category: "network" }));
    });

    it("returns all types in a given category", () => {
      const common = registry.getTypesByCategory("common");
      expect(common).toHaveLength(2);
      expect(common.map((d) => d.type)).toEqual(
        expect.arrayContaining(["inject", "debug"]),
      );
    });

    it("returns empty array for unknown category", () => {
      expect(registry.getTypesByCategory("nonexistent")).toEqual([]);
    });

    it("returns sorted unique category names", () => {
      const cats = registry.getCategories();
      expect(cats).toEqual(["common", "function", "network"]);
    });
  });

  // ---- colour / icon / label --------------------------------------------

  describe("getNodeColor", () => {
    it("returns the registered colour", () => {
      registry.registerType("inject", makeDef({ type: "inject", color: "#a6bbcf" }));
      expect(registry.getNodeColor("inject")).toBe("#a6bbcf");
    });

    it("returns white for unknown types", () => {
      expect(registry.getNodeColor("nope")).toBe("#ffffff");
    });
  });

  describe("getNodeIcon", () => {
    it("returns the icon for a registered type", () => {
      registry.registerType("inject", makeDef({ type: "inject", icon: "fa-arrow" }));
      expect(registry.getNodeIcon("inject")).toBe("fa-arrow");
    });

    it("returns undefined when no icon", () => {
      registry.registerType("plain", makeDef({ type: "plain" }));
      expect(registry.getNodeIcon("plain")).toBeUndefined();
    });
  });

  describe("getNodeLabel", () => {
    it("uses label function when provided", () => {
      registry.registerType(
        "inject",
        makeDef({
          type: "inject",
          label: (node) => `inject: ${node.topic as string}`,
        }),
      );
      const node: FlowNode = {
        id: "1", type: "inject", x: 0, y: 0, z: "tab1", wires: [], topic: "hello",
      };
      expect(registry.getNodeLabel("inject", node)).toBe("inject: hello");
    });

    it("uses static label string", () => {
      registry.registerType("test", makeDef({ type: "test", label: "My Node" }));
      const node: FlowNode = { id: "1", type: "test", x: 0, y: 0, z: "tab1", wires: [] };
      expect(registry.getNodeLabel("test", node)).toBe("My Node");
    });

    it("falls back to node.name", () => {
      registry.registerType("test", makeDef({ type: "test" }));
      const node: FlowNode = { id: "1", type: "test", name: "Named", x: 0, y: 0, z: "tab1", wires: [] };
      expect(registry.getNodeLabel("test", node)).toBe("Named");
    });

    it("falls back to paletteLabel", () => {
      registry.registerType("test", makeDef({ type: "test", paletteLabel: "Palette" }));
      const node: FlowNode = { id: "1", type: "test", x: 0, y: 0, z: "tab1", wires: [] };
      expect(registry.getNodeLabel("test", node)).toBe("Palette");
    });

    it("ultimately falls back to type string", () => {
      registry.registerType("test", makeDef({ type: "test" }));
      const node: FlowNode = { id: "1", type: "test", x: 0, y: 0, z: "tab1", wires: [] };
      expect(registry.getNodeLabel("test", node)).toBe("test");
    });

    it("returns type for unregistered type", () => {
      const node: FlowNode = { id: "1", type: "nope", x: 0, y: 0, z: "tab1", wires: [] };
      expect(registry.getNodeLabel("nope", node)).toBe("nope");
    });
  });
});

// ---------------------------------------------------------------------------
// Built-in nodes
// ---------------------------------------------------------------------------

describe("Built-in nodes", () => {
  let registry: NodeRegistry;

  beforeEach(() => {
    registry = new NodeRegistry();
  });

  it("registers all built-in types without error", () => {
    const list = getBuiltinNodeList();
    expect(list.length).toBeGreaterThan(20);

    for (const def of list) {
      expect(() => registry.registerType(def.type, def)).not.toThrow();
    }
  });

  it("covers all expected categories", () => {
    for (const def of getBuiltinNodeList()) {
      registry.registerType(def.type, def);
    }
    const cats = registry.getCategories();
    expect(cats).toContain("common");
    expect(cats).toContain("function");
    expect(cats).toContain("network");
    expect(cats).toContain("storage");
    expect(cats).toContain("parser");
    expect(cats).toContain("sequence");
  });

  it("common category includes expected types", () => {
    for (const def of getBuiltinNodeList()) {
      registry.registerType(def.type, def);
    }
    const commonTypes = registry.getTypesByCategory("common").map((d) => d.type);
    expect(commonTypes).toContain("inject");
    expect(commonTypes).toContain("debug");
    expect(commonTypes).toContain("comment");
    expect(commonTypes).toContain("link in");
    expect(commonTypes).toContain("link out");
    expect(commonTypes).toContain("link call");
    expect(commonTypes).toContain("execute");
  });

  it("function category includes expected types", () => {
    for (const def of getBuiltinNodeList()) {
      registry.registerType(def.type, def);
    }
    const fnTypes = registry.getTypesByCategory("function").map((d) => d.type);
    for (const t of ["function", "switch", "change", "range", "template", "delay", "trigger", "exec", "rbe"]) {
      expect(fnTypes).toContain(t);
    }
  });
});

// ---------------------------------------------------------------------------
// Singleton (registerBuiltinNodes)
// ---------------------------------------------------------------------------

describe("registerBuiltinNodes (singleton)", () => {
  beforeEach(() => {
    nodeRegistry.clear();
  });

  it("populates the global singleton registry", () => {
    registerBuiltinNodes();
    expect(nodeRegistry.getType("inject")).toBeDefined();
    expect(nodeRegistry.getType("debug")).toBeDefined();
    expect(nodeRegistry.getType("function")).toBeDefined();
    expect(nodeRegistry.getType("json")).toBeDefined();
    expect(nodeRegistry.getType("split")).toBeDefined();
  });
});
