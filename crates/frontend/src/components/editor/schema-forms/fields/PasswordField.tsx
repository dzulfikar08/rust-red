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
            "bg-[#444] border-[#555] text-gray-100 placeholder-gray-400 " +
            "focus:border-[#777] focus:ring-1 focus:ring-[#557da0] " +
            "disabled:opacity-50 disabled:cursor-not-allowed"
          }
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-invalid={!!error}
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
      </div>

      {error && (
        <span className="text-xs text-red-400">{error}</span>
      )}
    </div>
  );
}
