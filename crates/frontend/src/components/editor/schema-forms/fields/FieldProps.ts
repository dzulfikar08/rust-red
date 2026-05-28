import type { NodeDefault } from "@/red/nodes/types";

/**
 * Common props for all schema-driven field components.
 */
export interface FieldProps {
  /** Property name (key from defaults) */
  name: string;
  /** Optional display label (defaults to humanised name) */
  label?: string;
  /** Current value */
  value: unknown;
  /** Callback when value changes */
  onChange: (value: unknown) => void;
  /** Validation error message */
  error?: string;
  /** Disable the field */
  disabled?: boolean;
  /** Whether the field is required */
  required?: boolean;
  /** The full schema entry for this field */
  schema: NodeDefault;
}
