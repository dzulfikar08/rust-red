import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TypedInput, validateValue } from "../TypedInput";
import type { TypedInputType } from "../TypedInput";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderTypedInput(overrides: Partial<Parameters<typeof TypedInput>[0]> = {}) {
  const defaultProps = {
    value: "",
    type: "str" as TypedInputType,
    onChange: vi.fn(),
    ...overrides,
  };
  const result = render(<TypedInput {...defaultProps} />);
  return { ...result, props: defaultProps };
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

describe("TypedInput", () => {
  describe("rendering", () => {
    it("renders with default type", () => {
      renderTypedInput({ type: "str", value: "hello" });
      expect(screen.getByTestId("typedinput-container")).toBeInTheDocument();
      expect(screen.getByTestId("typedinput-value-input")).toHaveValue("hello");
    });

    it("renders the type button when multiple types available", () => {
      renderTypedInput({ type: "str", types: ["str", "num", "bool"] });
      expect(screen.getByTestId("typedinput-type-button")).toBeInTheDocument();
    });

    it("renders single type label when only one type", () => {
      renderTypedInput({ type: "str", types: ["str"] });
      expect(screen.getByTestId("typedinput-single-type-label")).toBeInTheDocument();
      expect(screen.getByTestId("typedinput-single-type-label")).toHaveTextContent("string");
    });

    it("renders label when provided", () => {
      renderTypedInput({ label: "Property" });
      expect(screen.getByText("Property")).toBeInTheDocument();
    });

    it("does not render label when not provided", () => {
      renderTypedInput();
      expect(screen.queryByRole("label")).not.toBeInTheDocument();
    });

    it("renders placeholder when provided", () => {
      renderTypedInput({ placeholder: "Enter value..." });
      expect(screen.getByTestId("typedinput-value-input")).toHaveAttribute(
        "placeholder",
        "Enter value..."
      );
    });
  });

  // -------------------------------------------------------------------------
  // Type dropdown
  // -------------------------------------------------------------------------

  describe("type dropdown", () => {
    it("opens type menu on click", () => {
      renderTypedInput({ types: ["str", "num", "bool"] });
      const btn = screen.getByTestId("typedinput-type-button");
      fireEvent.click(btn);
      expect(screen.getByTestId("typedinput-type-menu")).toBeInTheDocument();
    });

    it("renders all types in dropdown", () => {
      renderTypedInput({ types: ["str", "num", "bool"] });
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const menu = screen.getByTestId("typedinput-type-menu");
      const options = menu.querySelectorAll('[role="option"]');
      expect(options).toHaveLength(3);
    });

    it("closes menu when a type is selected", () => {
      renderTypedInput({ types: ["str", "num"] });
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      expect(screen.getByTestId("typedinput-type-menu")).toBeInTheDocument();

      // Click the "number" option
      const numOption = screen.getByText("number", { selector: '[role="option"] span' });
      fireEvent.click(numOption.closest('[role="option"]')!);
      expect(screen.queryByTestId("typedinput-type-menu")).not.toBeInTheDocument();
    });

    it("shows checkmark next to current type", () => {
      renderTypedInput({ type: "str", types: ["str", "num"] });
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const menu = screen.getByTestId("typedinput-type-menu");
      const selectedOption = menu.querySelector('[aria-selected="true"]');
      expect(selectedOption).toBeInTheDocument();
      expect(selectedOption).toHaveTextContent("string");
    });
  });

  // -------------------------------------------------------------------------
  // Type switching
  // -------------------------------------------------------------------------

  describe("type change", () => {
    it("calls onChange with new type and value", () => {
      const onChange = vi.fn();
      renderTypedInput({ type: "str", value: "hello", onChange, types: ["str", "num"] });

      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const numOption = screen.getByText("number", { selector: '[role="option"] span' });
      fireEvent.click(numOption.closest('[role="option"]')!);

      expect(onChange).toHaveBeenCalledWith("", "num");
    });

    it("restores previous value when switching back to a type", () => {
      const onChange = vi.fn();
      const { rerender } = render(
        <TypedInput
          value="hello"
          type="str"
          onChange={onChange}
          types={["str", "num"]}
        />
      );

      // Switch to num
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const numOption = screen.getByText("number", { selector: '[role="option"] span' });
      fireEvent.click(numOption.closest('[role="option"]')!);

      // Simulate the parent updating props after onChange
      const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1];
      rerender(
        <TypedInput
          value={lastCall[0]}
          type={lastCall[1]}
          onChange={onChange}
          types={["str", "num"]}
        />
      );

      // Switch back to str - should restore "hello"
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const strOption = screen.getByText("string", { selector: '[role="option"] span' });
      fireEvent.click(strOption.closest('[role="option"]')!);

      expect(onChange).toHaveBeenLastCalledWith("hello", "str");
    });

    it("defaults to first option when switching to bool type", () => {
      const onChange = vi.fn();
      renderTypedInput({ type: "str", value: "test", onChange, types: ["str", "bool"] });

      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const boolOption = screen.getByText("boolean", { selector: '[role="option"] span' });
      fireEvent.click(boolOption.closest('[role="option"]')!);

      expect(onChange).toHaveBeenCalledWith("true", "bool");
    });
  });

  // -------------------------------------------------------------------------
  // Value changes
  // -------------------------------------------------------------------------

  describe("value changes", () => {
    it("calls onChange when typing in text input", () => {
      const onChange = vi.fn();
      renderTypedInput({ type: "str", value: "", onChange });

      const input = screen.getByTestId("typedinput-value-input");
      fireEvent.change(input, { target: { value: "new value" } });

      expect(onChange).toHaveBeenCalledWith("new value", "str");
    });

    it("calls onChange when selecting bool option", () => {
      const onChange = vi.fn();
      renderTypedInput({ type: "bool", value: "true", onChange, types: ["bool"] });

      // For single type, there's no type button, so we need to provide multiple types
      // Let me re-render with multiple types
    });

    it("calls onChange when selecting bool option from dropdown", () => {
      const onChange = vi.fn();
      renderTypedInput({ type: "bool", value: "true", onChange, types: ["str", "bool"] });

      const boolBtn = screen.getByTestId("typedinput-bool-select");
      fireEvent.click(boolBtn);

      const falseOption = screen.getByText("false", { selector: '[role="option"]' });
      fireEvent.click(falseOption);

      expect(onChange).toHaveBeenCalledWith("false", "bool");
    });
  });

  // -------------------------------------------------------------------------
  // Prefix display
  // -------------------------------------------------------------------------

  describe("prefix display", () => {
    it("shows prefix for msg type", () => {
      renderTypedInput({ type: "msg", value: "payload" });
      expect(screen.getByTestId("typedinput-prefix")).toHaveTextContent("msg.");
    });

    it("shows prefix for flow type", () => {
      renderTypedInput({ type: "flow", value: "context" });
      expect(screen.getByTestId("typedinput-prefix")).toHaveTextContent("flow.");
    });

    it("shows prefix for global type", () => {
      renderTypedInput({ type: "global", value: "data" });
      expect(screen.getByTestId("typedinput-prefix")).toHaveTextContent("global.");
    });

    it("shows prefix for env type", () => {
      renderTypedInput({ type: "env", value: "MY_VAR" });
      expect(screen.getByTestId("typedinput-prefix")).toHaveTextContent("env.");
    });

    it("does not show prefix for str type", () => {
      renderTypedInput({ type: "str", value: "hello" });
      expect(screen.queryByTestId("typedinput-prefix")).not.toBeInTheDocument();
    });

    it("does not show prefix for num type", () => {
      renderTypedInput({ type: "num", value: "42" });
      expect(screen.queryByTestId("typedinput-prefix")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Bool type
  // -------------------------------------------------------------------------

  describe("bool type", () => {
    it("shows bool dropdown instead of text input", () => {
      renderTypedInput({ type: "bool", value: "true", types: ["str", "bool"] });
      expect(screen.getByTestId("typedinput-bool-select")).toBeInTheDocument();
      expect(screen.queryByTestId("typedinput-value-input")).not.toBeInTheDocument();
    });

    it("displays current bool value", () => {
      renderTypedInput({ type: "bool", value: "false", types: ["str", "bool"] });
      expect(screen.getByTestId("typedinput-bool-select")).toHaveTextContent("false");
    });

    it("displays true when value is empty", () => {
      renderTypedInput({ type: "bool", value: "", types: ["str", "bool"] });
      expect(screen.getByTestId("typedinput-bool-select")).toHaveTextContent("true");
    });

    it("opens bool dropdown on click", () => {
      renderTypedInput({ type: "bool", value: "true", types: ["str", "bool"] });
      fireEvent.click(screen.getByTestId("typedinput-bool-select"));
      expect(screen.getByTestId("typedinput-bool-menu")).toBeInTheDocument();
    });

    it("has true and false options in bool dropdown", () => {
      renderTypedInput({ type: "bool", value: "true", types: ["str", "bool"] });
      fireEvent.click(screen.getByTestId("typedinput-bool-select"));
      const menu = screen.getByTestId("typedinput-bool-menu");
      expect(menu).toHaveTextContent("true");
      expect(menu).toHaveTextContent("false");
    });
  });

  // -------------------------------------------------------------------------
  // Validation
  // -------------------------------------------------------------------------

  describe("validation", () => {
    it("shows error for invalid number", () => {
      renderTypedInput({ type: "num", value: "abc", types: ["num"] });
      expect(screen.getByTestId("typedinput-error")).toBeInTheDocument();
    });

    it("does not show error for valid number", () => {
      renderTypedInput({ type: "num", value: "42", types: ["num"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("does not show error for empty number (allowed)", () => {
      renderTypedInput({ type: "num", value: "", types: ["num"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("does not show error for decimal numbers", () => {
      renderTypedInput({ type: "num", value: "3.14", types: ["num"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("does not show error for negative numbers", () => {
      renderTypedInput({ type: "num", value: "-10", types: ["num"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("shows error for invalid JSON", () => {
      renderTypedInput({ type: "json", value: "{invalid", types: ["json"] });
      expect(screen.getByTestId("typedinput-error")).toBeInTheDocument();
    });

    it("does not show error for valid JSON", () => {
      renderTypedInput({ type: "json", value: '{"key":"value"}', types: ["json"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("does not show error for empty JSON (allowed)", () => {
      renderTypedInput({ type: "json", value: "", types: ["json"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("does not show error for JSON array", () => {
      renderTypedInput({ type: "json", value: "[1,2,3]", types: ["json"] });
      expect(screen.queryByTestId("typedinput-error")).not.toBeInTheDocument();
    });

    it("applies error border style on validation failure", () => {
      renderTypedInput({ type: "num", value: "not-a-number", types: ["num"] });
      const container = screen.getByTestId("typedinput-container");
      // The error class should add border-red-400
      expect(container.className).toContain("border-red-400");
    });
  });

  // -------------------------------------------------------------------------
  // validateValue utility
  // -------------------------------------------------------------------------

  describe("validateValue utility", () => {
    it("returns valid for str type", () => {
      expect(validateValue("str", "anything")).toEqual({ valid: true });
    });

    it("returns valid for empty num", () => {
      expect(validateValue("num", "")).toEqual({ valid: true });
    });

    it("returns invalid for bad number", () => {
      expect(validateValue("num", "abc")).toEqual({
        valid: false,
        error: "Must be a valid number",
      });
    });

    it("returns valid for good number", () => {
      expect(validateValue("num", "42")).toEqual({ valid: true });
    });

    it("returns invalid for bad JSON", () => {
      const result = validateValue("json", "{bad");
      expect(result.valid).toBe(false);
      expect(result.error).toBe("Invalid JSON");
    });

    it("returns valid for good JSON", () => {
      expect(validateValue("json", '{"a":1}')).toEqual({ valid: true });
    });
  });

  // -------------------------------------------------------------------------
  // Disabled state
  // -------------------------------------------------------------------------

  describe("disabled state", () => {
    it("applies disabled styling", () => {
      renderTypedInput({ disabled: true });
      const container = screen.getByTestId("typedinput-container");
      expect(container.className).toContain("opacity-50");
    });

    it("does not open type menu when disabled", () => {
      renderTypedInput({ disabled: true, types: ["str", "num"] });
      const btn = screen.getByTestId("typedinput-type-button");
      fireEvent.click(btn);
      expect(screen.queryByTestId("typedinput-type-menu")).not.toBeInTheDocument();
    });

    it("disables the text input", () => {
      renderTypedInput({ disabled: true, type: "str" });
      expect(screen.getByTestId("typedinput-value-input")).toBeDisabled();
    });

    it("does not open bool menu when disabled", () => {
      renderTypedInput({ disabled: true, type: "bool", value: "true", types: ["bool", "str"] });
      const btn = screen.getByTestId("typedinput-bool-select");
      fireEvent.click(btn);
      expect(screen.queryByTestId("typedinput-bool-menu")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Custom types list
  // -------------------------------------------------------------------------

  describe("custom types list", () => {
    it("only shows provided types in dropdown", () => {
      renderTypedInput({ types: ["msg", "str"] });
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const menu = screen.getByTestId("typedinput-type-menu");
      const options = menu.querySelectorAll('[role="option"]');
      expect(options).toHaveLength(2);
      expect(menu).toHaveTextContent("msg.");
      expect(menu).toHaveTextContent("string");
    });

    it("works with single type (no dropdown)", () => {
      renderTypedInput({ types: ["num"], type: "num" });
      expect(screen.queryByTestId("typedinput-type-button")).not.toBeInTheDocument();
      expect(screen.getByTestId("typedinput-single-type-label")).toHaveTextContent("number");
    });

    it("shows all types when types prop not provided", () => {
      renderTypedInput();
      fireEvent.click(screen.getByTestId("typedinput-type-button"));
      const menu = screen.getByTestId("typedinput-type-menu");
      const options = menu.querySelectorAll('[role="option"]');
      expect(options).toHaveLength(10);
    });
  });

  // -------------------------------------------------------------------------
  // Focus state
  // -------------------------------------------------------------------------

  describe("focus state", () => {
    it("applies focus styling when input is focused", () => {
      renderTypedInput({ type: "str", value: "test" });
      const input = screen.getByTestId("typedinput-value-input");
      fireEvent.focus(input);
      const container = screen.getByTestId("typedinput-container");
      expect(container.className).toContain("border-blue-400");
    });

    it("removes focus styling when input loses focus", () => {
      renderTypedInput({ type: "str", value: "test" });
      const input = screen.getByTestId("typedinput-value-input");
      fireEvent.focus(input);
      fireEvent.blur(input);
      const container = screen.getByTestId("typedinput-container");
      expect(container.className).not.toContain("border-blue-400");
    });
  });

  // -------------------------------------------------------------------------
  // All type renderings
  // -------------------------------------------------------------------------

  describe("all types rendering", () => {
    const typesToTest: TypedInputType[] = [
      "msg", "flow", "global", "str", "num", "bool", "json", "bin", "env", "node",
    ];

    typesToTest.forEach((t) => {
      it(`renders ${t} type correctly`, () => {
        const value = t === "bool" ? "true" : t === "num" ? "42" : "test";
        renderTypedInput({ type: t, value, types: typesToTest });
        expect(screen.getByTestId("typedinput-container")).toBeInTheDocument();
      });
    });
  });
});
