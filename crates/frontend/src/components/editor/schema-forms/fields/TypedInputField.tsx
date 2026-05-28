import { useId } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * TypedInput field - wraps the TypedInput component.
 *
 * The TypedInput component is being built in parallel. Until it is available
 * this renders a plain text input with a small type badge. The component is
 * designed so the real TypedInput can be swapped in later without changing the
 * SchemaField consumer.
 */
export function TypedInputField({
  label,
  value,
  onChange,
  error,
  disabled,
  required,
  schema,
}: FieldProps) {
  const id = useId();
  const inputType = schema.type ?? "str";
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

      <div className="flex items-stretch gap-0">
        {/* TODO: Replace with actual TypedInput component once available */}
        <span
          className={
            "inline-flex items-center rounded-l border border-r-0 " +
            `${border} ` +
            "bg-[#3a3a3a] px-2 text-[10px] font-medium uppercase text-gray-400 " +
            "select-none"
          }
        >
          {inputType}
        </span>
        <div className="relative flex-1">
          <input
            id={id}
            type="text"
            className={
              "w-full rounded-r border bg-[#444] px-2 py-1.5 pr-6 " +
              "text-sm text-gray-100 outline-none transition-colors " +
              `${border} ${focusBorder} ` +
              "disabled:opacity-50 disabled:cursor-not-allowed"
            }
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            aria-invalid={hasError}
          />
          {hasError && (
            <span
              className="absolute right-1.5 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-bold leading-none select-none pointer-events-none"
              aria-hidden="true"
            >
              !
            </span>
          )}
        </div>
      </div>

      {hasError && (
        <span className="text-xs text-red-400">{error}</span>
      )}
    </div>
  );
}
