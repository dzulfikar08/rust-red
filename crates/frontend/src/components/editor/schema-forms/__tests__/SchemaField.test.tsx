import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SchemaField } from "../SchemaField";
import type { NodeDefault } from "@/red/nodes/types";

describe("SchemaField - field type mapping", () => {
  it("renders a text input for string default value", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="topic"
        schema={schema}
        value="test"
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText("Topic");
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute("type", "text");
  });

  it("renders a textarea for code-related keys", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="func"
        schema={schema}
        value="return msg;"
        onChange={vi.fn()}
      />,
    );

    const textarea = screen.getByLabelText("Func");
    expect(textarea.tagName).toBe("TEXTAREA");
  });

  it("renders a textarea for keys containing 'script'", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="myScript"
        schema={schema}
        value=""
        onChange={vi.fn()}
      />,
    );

    const textarea = screen.getByLabelText("My Script");
    expect(textarea.tagName).toBe("TEXTAREA");
  });

  it("renders a number input for numeric default value", () => {
    const schema: NodeDefault = { value: 0 };

    render(
      <SchemaField
        name="repeat"
        schema={schema}
        value={5}
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText("Repeat");
    expect(input).toHaveAttribute("type", "number");
  });

  it("renders a checkbox for boolean default value", () => {
    const schema: NodeDefault = { value: false };

    render(
      <SchemaField
        name="once"
        schema={schema}
        value={true}
        onChange={vi.fn()}
      />,
    );

    const checkbox = screen.getByLabelText("Once");
    expect(checkbox).toHaveAttribute("type", "checkbox");
    expect(checkbox).toBeChecked();
  });

  it("renders a typed input when schema has type property", () => {
    const schema: NodeDefault = { value: "", type: "str" };

    render(
      <SchemaField
        name="payload"
        schema={schema}
        value="hello"
        onChange={vi.fn()}
      />,
    );

    // TypedInputField shows the type as a badge
    expect(screen.getByText("str")).toBeInTheDocument();
  });

  it("renders a password input for keys ending in 'password'", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="dbPassword"
        schema={schema}
        value="secret"
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText("Db Password");
    expect(input).toHaveAttribute("type", "password");
  });

  it("renders a password input for keys ending in 'pass'", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="secretPass"
        schema={schema}
        value="abc"
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText("Secret Pass");
    expect(input).toHaveAttribute("type", "password");
  });

  it("displays validation error message", () => {
    const schema: NodeDefault = { value: "", required: true };

    render(
      <SchemaField
        name="topic"
        schema={schema}
        value=""
        onChange={vi.fn()}
        error="Topic is required"
      />,
    );

    expect(screen.getByText("Topic is required")).toBeInTheDocument();
  });

  it("disables the field when disabled is true", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="topic"
        schema={schema}
        value=""
        onChange={vi.fn()}
        disabled
      />,
    );

    expect(screen.getByLabelText("Topic")).toBeDisabled();
  });

  it("humanises camelCase names into labels", () => {
    const schema: NodeDefault = { value: "" };

    render(
      <SchemaField
        name="onceDelay"
        schema={schema}
        value={0.1}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getByText("Once Delay")).toBeInTheDocument();
  });

  it("renders a text input as fallback for unknown types", () => {
    const schema: NodeDefault = { value: null };

    render(
      <SchemaField
        name="unknown"
        schema={schema}
        value="test"
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText("Unknown");
    expect(input).toHaveAttribute("type", "text");
  });
});
