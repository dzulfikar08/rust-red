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
  const hasError = !!error;
  const border = hasError ? "border-red-500" : "border-[#555]";

  return (
    <div className="flex items-center gap-2">
      <input
        id={id}
        type="checkbox"
        className={
          `h-4 w-4 rounded border ${border} bg-[#444] text-[#557da0] ` +
          "focus:ring-1 focus:ring-[#557da0] " +
          "disabled:opacity-50 disabled:cursor-not-allowed"
        }
        checked={Boolean(value)}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        aria-invalid={hasError}
      />

      <label
        htmlFor={id}
        className="text-xs font-medium text-gray-300 select-none cursor-pointer"
      >
        {label}
      </label>

      {hasError && (
        <>
          <span
            className="w-4 h-4 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-bold leading-none select-none"
            aria-hidden="true"
          >
            !
          </span>
          <span className="text-xs text-red-400">{error}</span>
        </>
      )}
    </div>
  );
}
