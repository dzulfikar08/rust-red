import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState, type ReactNode } from "react";
import { SchemaForm } from "../SchemaForm";
import type { NodeDefault } from "@/red/nodes/types";

/**
 * Wrapper that keeps values in React state so the form behaves as controlled.
 */
function StatefulForm({
  initial,
  schema,
  onChange,
}: {
  initial: Record<string, unknown>;
  schema: Record<string, NodeDefault>;
  onChange?: (key: string, value: unknown) => void;
}) {
  const [values, setValues] = useState(initial);
  const handleChange = (key: string, value: unknown) => {
    setValues((prev) => ({ ...prev, [key]: value }));
    onChange?.(key, value);
  };
  return <SchemaForm schema={schema} values={values} onChange={handleChange} />;
}

/**
 * Helper: build a simple schema + matching values.
 */
function makeSchema(entries: Record<string, NodeDefault>) {
  return entries;
}

describe("SchemaForm", () => {
  it("renders a text field from schema", () => {
    const schema = makeSchema({
      name: { value: "" },
      topic: { value: "", required: false },
    });
    const values = { name: "my-node", topic: "test/topic" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    // Both fields should render with their labels
    expect(screen.getByLabelText("Name")).toBeInTheDocument();
    expect(screen.getByLabelText("Topic")).toBeInTheDocument();

    // Values should be populated
    expect(screen.getByLabelText("Name")).toHaveValue("my-node");
    expect(screen.getByLabelText("Topic")).toHaveValue("test/topic");
  });

  it("renders a number field from schema", () => {
    const schema = makeSchema({
      repeat: { value: 0 },
      outputs: { value: 1 },
    });
    const values = { repeat: 5, outputs: 2 };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    expect(screen.getByLabelText("Repeat")).toHaveValue(5);
    expect(screen.getByLabelText("Outputs")).toHaveValue(2);
  });

  it("renders a boolean (checkbox) field from schema", () => {
    const schema = makeSchema({
      once: { value: false },
    });
    const values = { once: true };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    const checkbox = screen.getByLabelText("Once");
    expect(checkbox).toBeInTheDocument();
    expect(checkbox).toBeChecked();
  });

  it("renders a typed input field when schema has type property", () => {
    const schema = makeSchema({
      payload: { value: "", type: "str" },
    });
    const values = { payload: "hello" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    // TypedInputField shows a type badge
    expect(screen.getByText("str")).toBeInTheDocument();
    expect(screen.getByLabelText("Payload")).toHaveValue("hello");
  });

  it("renders a password field for keys ending in 'password'", () => {
    const schema = makeSchema({
      userPassword: { value: "" },
    });
    const values = { userPassword: "secret" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    const input = screen.getByLabelText("User Password");
    expect(input).toHaveAttribute("type", "password");
    expect(input).toHaveValue("secret");
  });

  it("places the 'name' field first regardless of schema order", () => {
    const schema = makeSchema({
      zebra: { value: "" },
      name: { value: "" },
      alpha: { value: "" },
    });
    const values = { zebra: "z", name: "n", alpha: "a" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    const labels = screen
      .getAllByRole("textbox")
      .map((el) => el.id);
    // The first input should be associated with the "Name" label
    const nameLabel = screen.getByLabelText("Name");
    const allInputs = screen.getAllByRole("textbox");
    expect(allInputs[0]).toBe(nameLabel);
  });

  it("calls onChange when a text field value changes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const schema = makeSchema({
      topic: { value: "" },
    });

    render(
      <StatefulForm
        initial={{ topic: "" }}
        schema={schema}
        onChange={onChange}
      />,
    );

    await user.type(screen.getByLabelText("Topic"), "hello");

    // onChange should have been called for each character, accumulating
    expect(onChange).toHaveBeenCalled();
    const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1];
    expect(lastCall[0]).toBe("topic");
    expect(lastCall[1]).toBe("hello");
  });

  it("calls onChange when a number field value changes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const schema = makeSchema({
      repeat: { value: 0 },
    });

    render(
      <StatefulForm
        initial={{ repeat: "" }}
        schema={schema}
        onChange={onChange}
      />,
    );

    const input = screen.getByLabelText("Repeat");
    await user.type(input, "42");

    // Last call should be the final accumulated number value
    const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1];
    expect(lastCall[0]).toBe("repeat");
    expect(lastCall[1]).toBe(42);
  });

  it("calls onChange when a boolean field value changes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const schema = makeSchema({
      once: { value: false },
    });
    const values = { once: false };

    render(
      <SchemaForm schema={schema} values={values} onChange={onChange} />,
    );

    await user.click(screen.getByLabelText("Once"));

    expect(onChange).toHaveBeenCalledWith("once", true);
  });

  it("displays validation errors", () => {
    const schema = makeSchema({
      topic: { value: "", required: true },
    });
    const values = { topic: "" };
    const errors = { topic: "Topic is required" };

    render(
      <SchemaForm
        schema={schema}
        values={values}
        onChange={vi.fn()}
        errors={errors}
      />,
    );

    expect(screen.getByText("Topic is required")).toBeInTheDocument();
  });

  it("shows error icon for fields with errors", () => {
    const schema = makeSchema({
      topic: { value: "", required: true },
    });
    const values = { topic: "" };
    const errors = { topic: "Topic is required" };

    render(
      <SchemaForm
        schema={schema}
        values={values}
        onChange={vi.fn()}
        errors={errors}
      />,
    );

    // The input should have aria-invalid when there's an error
    const inputs = screen.getAllByRole("textbox");
    const topicInput = inputs.find((el) => el.id.includes("topic") || el.getAttribute("value") === "");
    expect(topicInput).toBeTruthy();
    expect(topicInput).toHaveAttribute("aria-invalid", "true");
  });

  it("does not show error styling for fields without errors", () => {
    const schema = makeSchema({
      topic: { value: "" },
      name: { value: "" },
    });
    const values = { topic: "valid", name: "valid" };
    const errors = { topic: "Topic has error" };

    render(
      <SchemaForm
        schema={schema}
        values={values}
        onChange={vi.fn()}
        errors={errors}
      />,
    );

    // Topic input should have aria-invalid, name should not
    const allInputs = screen.getAllByRole("textbox");
    // Name comes first (sorted), then Topic
    expect(allInputs[0]).not.toHaveAttribute("aria-invalid", "true");
    expect(allInputs[1]).toHaveAttribute("aria-invalid", "true");
  });

  it("shows multiple field errors simultaneously", () => {
    const schema = makeSchema({
      name: { value: "", required: true },
      topic: { value: "", required: true },
    });
    const values = { name: "", topic: "" };
    const errors = { name: "Name is required", topic: "Topic is required" };

    render(
      <SchemaForm
        schema={schema}
        values={values}
        onChange={vi.fn()}
        errors={errors}
      />,
    );

    expect(screen.getByText("Name is required")).toBeInTheDocument();
    expect(screen.getByText("Topic is required")).toBeInTheDocument();
  });

  it("disables all fields when disabled prop is true", () => {
    const schema = makeSchema({
      name: { value: "" },
      repeat: { value: 0 },
      once: { value: false },
    });
    const values = { name: "", repeat: 0, once: false };

    render(
      <SchemaForm
        schema={schema}
        values={values}
        onChange={vi.fn()}
        disabled
      />,
    );

    expect(screen.getByLabelText("Name")).toBeDisabled();
    expect(screen.getByLabelText("Repeat")).toBeDisabled();
    expect(screen.getByLabelText("Once")).toBeDisabled();
  });

  it("renders a multiline textarea for code/script keys", () => {
    const schema = makeSchema({
      myCode: { value: "" },
    });
    const values = { myCode: "return msg;" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    const textarea = screen.getByLabelText("My Code");
    expect(textarea.tagName).toBe("TEXTAREA");
    expect(textarea).toHaveValue("return msg;");
  });

  it("shows required indicator for required fields", () => {
    const schema = makeSchema({
      topic: { value: "", required: true },
    });
    const values = { topic: "" };

    render(
      <SchemaForm schema={schema} values={values} onChange={vi.fn()} />,
    );

    expect(screen.getByText("*")).toBeInTheDocument();
  });
});
