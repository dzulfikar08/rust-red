import { useId } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * Text input field with label.
 * Renders as a <textarea> when `multiline` is true (or when the key
 * contains "code" / "script").
 */
export function TextField({
  label,
  value,
  onChange,
  error,
  disabled,
  required,
  multiline,
}: FieldProps & { multiline?: boolean }) {
  const id = useId();

  const inputClasses =
    "w-full rounded border px-2 py-1.5 text-sm outline-none transition-colors " +
    "bg-[#444] border-[#555] text-gray-100 placeholder-gray-400 " +
    "focus:border-[#777] focus:ring-1 focus:ring-[#557da0] " +
    "disabled:opacity-50 disabled:cursor-not-allowed";

  return (
    <div className="flex flex-col gap-1">
      <label
        htmlFor={id}
        className="text-xs font-medium text-gray-300"
      >
        {label}
        {required && <span className="ml-0.5 text-red-400">*</span>}
      </label>

      {multiline ? (
        <textarea
          id={id}
          className={`${inputClasses} min-h-[80px] resize-y`}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-invalid={!!error}
        />
      ) : (
        <input
          id={id}
          type="text"
          className={inputClasses}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-invalid={!!error}
        />
      )}

      {error && (
        <span className="text-xs text-red-400">{error}</span>
      )}
    </div>
  );
}
