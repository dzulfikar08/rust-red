/**
 * Node Type Registry
 *
 * Singleton registry that replaces Node-RED's RED.nodes type registry.
 * Stores `NodeDefinition` objects keyed by type string and provides
 * lookup helpers by category, colour, icon, and label.
 */

import type { NodeDefinition, FlowNode } from "./types";

export class NodeRegistry {
  private types = new Map<string, NodeDefinition>();

  // ---- Mutation ----------------------------------------------------------

  /**
   * Register a node type definition. Overwrites any existing definition
   * with the same type key.
   */
  registerType(type: string, definition: NodeDefinition): void {
    this.types.set(type, definition);
  }

  /**
   * Remove a previously registered type.
   */
  removeType(type: string): void {
    this.types.delete(type);
  }

  /**
   * Remove all registered types (useful for tests).
   */
  clear(): void {
    this.types.clear();
  }

  // ---- Query -------------------------------------------------------------

  /**
   * Get the definition for a specific type, or `undefined`.
   */
  getType(type: string): NodeDefinition | undefined {
    return this.types.get(type);
  }

  /**
   * Return the full internal map (read-only snapshot via a copy).
   */
  getAllTypes(): Map<string, NodeDefinition> {
    return new Map(this.types);
  }

  /**
   * Get all definitions in a given category.
   */
  getTypesByCategory(category: string): NodeDefinition[] {
    const results: NodeDefinition[] = [];
    for (const def of this.types.values()) {
      if (def.category === category) {
        results.push(def);
      }
    }
    return results;
  }

  /**
   * Get a sorted list of unique category names that have at least one type.
   */
  getCategories(): string[] {
    const cats = new Set<string>();
    for (const def of this.types.values()) {
      cats.add(def.category);
    }
    return Array.from(cats).sort();
  }

  /**
   * Resolve the colour for a node type. Falls back to `#ffffff`.
   */
  getNodeColor(type: string): string {
    return this.types.get(type)?.color ?? "#ffffff";
  }

  /**
   * Resolve the icon class for a node type. Returns `undefined` if none.
   */
  getNodeIcon(type: string): string | undefined {
    return this.types.get(type)?.icon;
  }

  /**
   * Resolve the display label for a node instance.
   *
   * Priority:
   *  1. `definition.label` if it's a function → call it with the node
   *  2. `definition.label` if it's a string → use it
   *  3. `node.name` if present and non-empty
   *  4. `definition.paletteLabel`
   *  5. `type`
   */
  getNodeLabel(type: string, node: FlowNode): string {
    const def = this.types.get(type);
    if (!def) return type;

    if (typeof def.label === "function") {
      return def.label(node as Record<string, unknown>);
    }
    if (typeof def.label === "string" && def.label.length > 0) {
      return def.label;
    }
    if (node.name && node.name.length > 0) {
      return node.name;
    }
    if (def.paletteLabel && def.paletteLabel.length > 0) {
      return def.paletteLabel;
    }
    return type;
  }
}

/** Singleton instance shared across the application. */
export const nodeRegistry = new NodeRegistry();
