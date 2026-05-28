/**
 * Node Validation Engine
 *
 * Validates node form data against node definition schemas.
 * Provides structured error objects for displaying inline
 * validation messages in the node editor UI.
 */

import type { NodeDefinition, NodeDefault } from "@/red/nodes/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ValidationError {
  /** Property/field name that failed validation */
  field: string;
  /** Human-readable error message */
  message: string;
  /** Category of validation failure */
  type: "required" | "invalid" | "custom";
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Check if a value is considered "empty" (missing, null, undefined, blank string).
 */
function isEmpty(value: unknown): boolean {
  return value === undefined || value === null || value === "";
}

/**
 * Check if a value is a valid JSON string.
 */
function isValidJson(value: string): boolean {
  try {
    JSON.parse(value);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Type-specific validators
// ---------------------------------------------------------------------------

/**
 * Validate a value against the schema type hint.
 * Returns a ValidationError or null.
 */
function validateByType(
  field: string,
  value: unknown,
  schema: NodeDefault,
): ValidationError | null {
  const schemaType = schema.type;
  const strValue = typeof value === "string" ? value : String(value ?? "");

  // TypedInput fields with a type badge (e.g. "num", "json", "str")
  if (typeof schemaType === "string") {
    switch (schemaType) {
      case "num":
        if (!isEmpty(value) && Number.isNaN(Number(strValue))) {
          return {
            field,
            message: `${field} must be a valid number`,
            type: "invalid",
          };
        }
        break;

      case "json":
        if (!isEmpty(value) && !isValidJson(strValue)) {
          return {
            field,
            message: `${field} must be valid JSON`,
            type: "invalid",
          };
        }
        break;

      // "str" and other types: no type-specific validation beyond required
      default:
        break;
    }
  }

  // Number schema (detected from default value type)
  if (typeof schema.value === "number" && !isEmpty(value)) {
    if (Number.isNaN(Number(value))) {
      return {
        field,
        message: `${field} must be a valid number`,
        type: "invalid",
      };
    }
  }

  return null;
}

// ---------------------------------------------------------------------------
// Main validator
// ---------------------------------------------------------------------------

/**
 * Validate node form data against a node definition schema.
 *
 * Rules applied in order:
 * 1. **Required fields**: If `schema.required === true`, the value must not be
 *    empty, null, or undefined.
 * 2. **Type-specific validation**:
 *    - `num` type: must be a valid number (when not empty)
 *    - `json` type: must be valid JSON (when not empty)
 *    - Number defaults: must be a valid number (when not empty)
 * 3. **Custom validators**: If `schema.validate` is a function, it is called
 *    with the value. If it returns `false`, the field is invalid.
 *
 * @param nodeType - The node type identifier (used for context)
 * @param formData - Current form data (property name → value)
 * @param nodeDefinition - The full node definition with defaults schema
 * @returns Array of validation errors (empty array means the form is valid)
 */
export function validateNodeForm(
  nodeType: string,
  formData: Record<string, unknown>,
  nodeDefinition: NodeDefinition,
): ValidationError[] {
  const errors: ValidationError[] = [];
  const defaults = nodeDefinition.defaults;

  // Always validate the "name" field if present in defaults or formData
  const keysToValidate = new Set<string>([
    ...Object.keys(defaults),
    ...Object.keys(formData),
  ]);

  for (const field of keysToValidate) {
    const schema = defaults[field];
    const value = formData[field];

    // Skip fields not in the schema (e.g. internal fields like "id", "type", "z")
    if (!schema) continue;

    // 1. Required check
    if (schema.required && isEmpty(value)) {
      errors.push({
        field,
        message: `${field} is required`,
        type: "required",
      });
      continue; // Skip further validation for empty required fields
    }

    // 2. Type-specific validation (only when value is not empty)
    if (!isEmpty(value)) {
      const typeError = validateByType(field, value, schema);
      if (typeError) {
        errors.push(typeError);
        continue; // Don't double-report with custom validator
      }
    }

    // 3. Custom validator function
    if (typeof schema.validate === "function") {
      try {
        const result = schema.validate(value);
        if (!result) {
          errors.push({
            field,
            message: `${field} is invalid`,
            type: "custom",
          });
        }
      } catch {
        errors.push({
          field,
          message: `${field} failed validation`,
          type: "custom",
        });
      }
    }
  }

  return errors;
}

/**
 * Convert an array of ValidationError objects into a simple Record<string, string>
 * mapping field names to their first error message. Useful for passing to form
 * components that expect a simple key→message map.
 */
export function errorsToMap(
  errors: ValidationError[],
): Record<string, string> {
  const map: Record<string, string> = {};
  for (const error of errors) {
    // Only keep the first error per field
    if (!(error.field in map)) {
      map[error.field] = error.message;
    }
  }
  return map;
}

/**
 * Validate a single field's value against its schema.
 * Returns the first error for the field, or null if valid.
 *
 * Useful for inline / on-blur validation.
 */
export function validateField(
  field: string,
  value: unknown,
  schema: NodeDefault,
): ValidationError | null {
  // Required check
  if (schema.required && isEmpty(value)) {
    return {
      field,
      message: `${field} is required`,
      type: "required",
    };
  }

  // Type-specific validation
  if (!isEmpty(value)) {
    const typeError = validateByType(field, value, schema);
    if (typeError) return typeError;
  }

  // Custom validator
  if (typeof schema.validate === "function") {
    try {
      if (!schema.validate(value)) {
        return {
          field,
          message: `${field} is invalid`,
          type: "custom",
        };
      }
    } catch {
      return {
        field,
        message: `${field} failed validation`,
        type: "custom",
      };
    }
  }

  return null;
}
