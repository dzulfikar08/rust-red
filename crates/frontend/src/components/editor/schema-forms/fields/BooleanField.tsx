import { useId } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * Checkbox field with label.
 */
export function BooleanField({
  label,
  value,
  onChange,
  error,
  disabled,
}: FieldProps) {
  const id = useId();

  return (
    <div className="flex items-center gap-2">
      <input
        id={id}
        type="checkbox"
        className={
          "h-4 w-4 rounded border border-[#555] bg-[#444] text-[#557da0] " +
          "focus:ring-1 focus:ring-[#557da0] " +
          "disabled:opacity-50 disabled:cursor-not-allowed"
        }
        checked={Boolean(value)}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        aria-invalid={!!error}
      />

      <label
        htmlFor={id}
        className="text-xs font-medium text-gray-300 select-none cursor-pointer"
      >
        {label}
      </label>

      {error && (
        <span className="text-xs text-red-400 ml-auto">{error}</span>
      )}
    </div>
  );
}
