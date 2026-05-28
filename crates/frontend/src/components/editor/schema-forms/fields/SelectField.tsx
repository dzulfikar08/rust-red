import { useId } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * Select (dropdown) field with label.
 *
 * Options can be supplied via `schema.options` or `schema.validate` as a
 * pipe-separated string (Node-RED convention). Falls back to a text input
 * when no options are available.
 */
export interface SelectFieldProps extends FieldProps {
  /** Explicit list of option values */
  options?: string[];
}

export function SelectField({
  label,
  value,
  onChange,
  error,
  disabled,
  required,
  options = [],
}: SelectFieldProps) {
  const id = useId();
  const hasError = !!error;
  const border = hasError ? "border-red-500" : "border-[#555]";
  const focusBorder = hasError
    ? "focus:border-red-400 focus:ring-1 focus:ring-red-400"
    : "focus:border-[#777] focus:ring-1 focus:ring-[#557da0]";

  return (
    <div className="flex flex-col gap-1">
      <label
        htmlFor={id}
        className="text-xs font-medium text-gray-300"
      >
        {label}
        {required && <span className="ml-0.5 text-red-400">*</span>}
      </label>

      <div className="relative">
        <select
          id={id}
          className={
            "w-full rounded border px-2 py-1.5 text-sm outline-none transition-colors " +
            "bg-[#444] text-gray-100 " +
            `${border} ${focusBorder} ` +
            "disabled:opacity-50 disabled:cursor-not-allowed"
          }
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-invalid={hasError}
        >
          {options.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
        {hasError && (
          <span
            className="absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-bold leading-none select-none pointer-events-none"
            aria-hidden="true"
          >
            !
          </span>
        )}
      </div>

      {hasError && (
        <span className="text-xs text-red-400">{error}</span>
      )}
    </div>
  );
}
