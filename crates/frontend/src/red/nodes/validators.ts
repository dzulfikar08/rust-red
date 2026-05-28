/**
 * Node & Property Validators
 *
 * Validation utilities for checking node property values against
 * their `NodeDefinition.defaults` specifications.
 */

import type { NodeDefault, NodeDefinition, FlowNode } from "./types";

/**
 * Returns `true` when the property's `required` flag is explicitly set to `true`.
 */
export function isRequired(defaults: NodeDefault): boolean {
  return defaults.required === true;
}

/**
 * Validate a single property of a node against its definition.
 *
 * Returns `true` when the value is valid:
 *  - If `required` is true, the value must not be `undefined`, `null`, or `""`.
 *  - If `validate` is a function, it must return truthy.
 *  - If `validate` is a string (e.g. a built-in validator name) it is ignored
 *    for now – we treat it as passing.
 */
export function validateNodeProperty(
  node: FlowNode,
  definition: NodeDefinition,
  property: string,
): boolean {
  const defaultSpec = definition.defaults[property];
  if (!defaultSpec) return true; // unknown property – nothing to validate

  const value = node[property];

  // Required check
  if (defaultSpec.required) {
    if (value === undefined || value === null || value === "") {
      return false;
    }
  }

  // Validate function
  if (typeof defaultSpec.validate === "function") {
    return defaultSpec.validate(value);
  }

  return true;
}

/**
 * Validate all properties of a node against its type definition.
 *
 * Returns an object with:
 *  - `valid` – `true` when every property passes
 *  - `errors` – map of property name → error message (only for failures)
 */
export function validateNode(
  node: FlowNode,
  definition: NodeDefinition,
): { valid: boolean; errors: Record<string, string> } {
  const errors: Record<string, string> = {};

  for (const [property, defaultSpec] of Object.entries(definition.defaults)) {
    const value = node[property];

    // Required check
    if (defaultSpec.required) {
      if (value === undefined || value === null || value === "") {
        errors[property] = `${property} is required`;
        continue;
      }
    }

    // Validate function
    if (typeof defaultSpec.validate === "function") {
      if (!defaultSpec.validate(value)) {
        errors[property] = `${property} is invalid`;
      }
    }
  }

  return { valid: Object.keys(errors).length === 0, errors };
}
