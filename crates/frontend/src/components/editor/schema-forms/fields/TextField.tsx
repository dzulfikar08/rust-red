import { useId } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * Error icon badge shown inside fields with validation errors.
 */
function ErrorIcon() {
  return (
    <span
      className="absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-bold leading-none select-none pointer-events-none"
      aria-hidden="true"
    >
      !
    </span>
  );
}

/**
 * Build the base input class string, with error-aware border colour.
 */
function inputClasses(hasError: boolean): string {
  const border = hasError ? "border-red-500" : "border-[#555]";
  const focusBorder = hasError
    ? "focus:border-red-400 focus:ring-1 focus:ring-red-400"
    : "focus:border-[#777] focus:ring-1 focus:ring-[#557da0]";

  return (
    "w-full rounded border px-2 py-1.5 text-sm outline-none transition-colors " +
    "bg-[#444] text-gray-100 placeholder-gray-400 " +
    `${border} ${focusBorder} ` +
    "disabled:opacity-50 disabled:cursor-not-allowed"
  );
}

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
  const hasError = !!error;

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
        {multiline ? (
          <textarea
            id={id}
            className={`${inputClasses(hasError)} min-h-[80px] resize-y`}
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            aria-invalid={hasError}
          />
        ) : (
          <input
            id={id}
            type="text"
            className={inputClasses(hasError)}
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            aria-invalid={hasError}
          />
        )}
        {hasError && !multiline && <ErrorIcon />}
      </div>

      {hasError && (
        <span className="text-xs text-red-400">{error}</span>
      )}
    </div>
  );
}
