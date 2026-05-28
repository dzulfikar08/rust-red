import { useId, useState } from "react";
import type { FieldProps } from "./FieldProps";

/**
 * Password input field with label and optional visibility toggle.
 */
export function PasswordField({
  label,
  value,
  onChange,
  error,
  disabled,
  required,
}: FieldProps) {
  const id = useId();
  const [visible, setVisible] = useState(false);
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
        <input
          id={id}
          type={visible ? "text" : "password"}
          className={
            "w-full rounded border px-2 py-1.5 pr-8 text-sm outline-none transition-colors " +
            "bg-[#444] text-gray-100 placeholder-gray-400 " +
            `${border} ${focusBorder} ` +
            "disabled:opacity-50 disabled:cursor-not-allowed"
          }
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-invalid={hasError}
        />
        <button
          type="button"
          className={
            "absolute right-1.5 top-1/2 -translate-y-1/2 text-xs text-gray-400 " +
            "hover:text-gray-200 focus:outline-none"
          }
          onClick={() => setVisible((v) => !v)}
          tabIndex={-1}
          aria-label={visible ? "Hide password" : "Show password"}
        >
          {visible ? "\u25CF" : "\u25CB"}
        </button>
        {hasError && (
          <span
            className="absolute right-8 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-bold leading-none select-none pointer-events-none"
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
