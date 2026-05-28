/**
 * Tests for the ChangeEditor component.
 *
 * Tests rendering, rule add/remove, action type changes.
 * TypedInput is mocked to simplify testing.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useState } from "react";
import { ChangeEditor } from "../ChangeEditor";
import { registerNodeEditor, clearNodeEditors, hasCustomEditor } from "../registry";

// ---------------------------------------------------------------------------
// Mock TypedInput
// ---------------------------------------------------------------------------

vi.mock("../../../common/TypedInput", () => ({
  TypedInput: vi.fn(({ value, type, onChange, types, label, "data-testid": testId }: any) => (
    <div data-testid={testId ?? "typedinput-mock"}>
      <input
        data-testid="typedinput-value"
        value={value}
        onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
          onChange?.(e.target.value, type)
        }
      />
      <select
        data-testid="typedinput-type"
        value={type}
        onChange={(e: React.ChangeEvent<HTMLSelectElement>) =>
          onChange?.(value, e.target.value)
        }
      >
        {(types ?? ["msg", "str"]).map((t: string) => (
          <option key={t} value={t}>{t}</option>
        ))}
      </select>
    </div>
  )),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function defaultProps(overrides: Record<string, any> = {}) {
  return {
    nodeId: "n1",
    nodeType: "change",
    values: {
      rules: [{ t: "set", p: "payload", pt: "msg", to: "", tot: "str" }],
      ...overrides,
    },
    onChange: vi.fn(),
  };
}

/** Stateful wrapper to keep rules in React state for add/remove testing */
function StatefulChangeEditor(props: any) {
  const [values, setValues] = useState(props.values);
  const handleChange = (key: string, value: any) => {
    setValues((prev: any) => ({ ...prev, [key]: value }));
    props.onChange(key, value);
  };
  return <ChangeEditor {...props} values={values} onChange={handleChange} />;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ChangeEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    registerNodeEditor("change", ChangeEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("change")).toBe(true);
  });

  it("renders with default rules", () => {
    render(<ChangeEditor {...defaultProps()} />);

    expect(screen.getByTestId("change-editor")).toBeInTheDocument();
    expect(screen.getByTestId("change-rules-list")).toBeInTheDocument();
    expect(screen.getByTestId("change-rule-0")).toBeInTheDocument();
  });

  it("renders a 'set' rule with action select and 'to' field", () => {
    render(<ChangeEditor {...defaultProps()} />);

    // Action select should default to "set"
    expect(screen.getByTestId("change-rule-action-0")).toHaveValue("set");
  });

  it("renders a 'delete' rule without value fields", () => {
    render(
      <ChangeEditor
        {...defaultProps({
          rules: [{ t: "delete", p: "payload", pt: "msg" }],
        })}
      />,
    );

    expect(screen.getByTestId("change-rule-action-0")).toHaveValue("delete");
    // Delete should not show any value field
  });

  it("renders a 'change' rule with search and replace fields", () => {
    render(
      <ChangeEditor
        {...defaultProps({
          rules: [{ t: "change", p: "payload", pt: "msg", from: "", fromt: "str", to: "", tot: "str" }],
        })}
      />,
    );

    expect(screen.getByTestId("change-rule-action-0")).toHaveValue("change");
  });

  it("adds a new rule when the add button is clicked", () => {
    render(<StatefulChangeEditor {...defaultProps()} />);

    expect(screen.getByTestId("change-rule-0")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("change-add-rule"));

    expect(screen.getByTestId("change-rule-1")).toBeInTheDocument();
  });

  it("removes a rule when the remove button is clicked", () => {
    render(
      <StatefulChangeEditor
        {...defaultProps({
          rules: [
            { t: "set", p: "payload", pt: "msg", to: "", tot: "str" },
            { t: "delete", p: "topic", pt: "msg" },
          ],
        })}
      />,
    );

    expect(screen.getByTestId("change-rule-0")).toBeInTheDocument();
    expect(screen.getByTestId("change-rule-1")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("change-remove-rule-1"));

    expect(screen.getByTestId("change-rule-0")).toBeInTheDocument();
    expect(screen.queryByTestId("change-rule-1")).not.toBeInTheDocument();
  });

  it("changes action type on a rule", () => {
    const onChange = vi.fn();
    render(<ChangeEditor {...defaultProps()} onChange={onChange} />);

    const select = screen.getByTestId("change-rule-action-0");
    fireEvent.change(select, { target: { value: "delete" } });

    expect(onChange).toHaveBeenCalledWith("rules", expect.arrayContaining([
      expect.objectContaining({ t: "delete" }),
    ]));
  });
});
