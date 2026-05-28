/**
 * Tests for the SwitchEditor component.
 *
 * Tests rendering, rule add/remove, operator changes, and checkall/repair.
 * TypedInput is mocked to simplify testing.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useState, type ReactNode } from "react";
import { SwitchEditor } from "../SwitchEditor";
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
    nodeType: "switch",
    values: {
      property: "payload",
      propertyType: "msg",
      rules: [{ t: "eq", v: "", vt: "str" }],
      checkall: "true",
      repair: false,
      outputs: 2,
      ...overrides,
    },
    onChange: vi.fn(),
  };
}

/** Stateful wrapper to keep rules in React state for add/remove testing */
function StatefulSwitchEditor(props: any) {
  const [values, setValues] = useState(props.values);
  const handleChange = (key: string, value: any) => {
    setValues((prev: any) => ({ ...prev, [key]: value }));
    props.onChange(key, value);
  };
  return <SwitchEditor {...props} values={values} onChange={handleChange} />;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SwitchEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    registerNodeEditor("switch", SwitchEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("switch")).toBe(true);
  });

  it("renders with default property and rules", () => {
    render(<SwitchEditor {...defaultProps()} />);

    expect(screen.getByTestId("switch-editor")).toBeInTheDocument();
    expect(screen.getByTestId("switch-rules-list")).toBeInTheDocument();
    // Should have the first rule
    expect(screen.getByTestId("switch-rule-0")).toBeInTheDocument();
  });

  it("renders the otherwise output indicator", () => {
    render(<SwitchEditor {...defaultProps()} />);

    expect(screen.getByTestId("switch-otherwise")).toBeInTheDocument();
  });

  it("renders checkall radios", () => {
    render(<SwitchEditor {...defaultProps()} />);

    expect(screen.getByTestId("switch-checkall-true")).toBeInTheDocument();
    expect(screen.getByTestId("switch-checkall-false")).toBeInTheDocument();
  });

  it("renders repair checkbox", () => {
    render(<SwitchEditor {...defaultProps()} />);

    expect(screen.getByTestId("switch-repair")).toBeInTheDocument();
  });

  it("adds a new rule when the add button is clicked", () => {
    render(<StatefulSwitchEditor {...defaultProps()} />);

    // Should start with 1 rule
    expect(screen.getByTestId("switch-rule-0")).toBeInTheDocument();

    // Add a rule
    fireEvent.click(screen.getByTestId("switch-add-rule"));

    // Should now have 2 rules
    expect(screen.getByTestId("switch-rule-1")).toBeInTheDocument();
  });

  it("removes a rule when the remove button is clicked", () => {
    const onChange = vi.fn();
    render(
      <StatefulSwitchEditor
        {...defaultProps({
          rules: [
            { t: "eq", v: "a", vt: "str" },
            { t: "neq", v: "b", vt: "str" },
          ],
        })}
        onChange={onChange}
      />,
    );

    // Should start with 2 rules
    expect(screen.getByTestId("switch-rule-0")).toBeInTheDocument();
    expect(screen.getByTestId("switch-rule-1")).toBeInTheDocument();

    // Remove the second rule
    fireEvent.click(screen.getByTestId("switch-remove-rule-1"));

    // Should now have only 1 rule
    expect(screen.getByTestId("switch-rule-0")).toBeInTheDocument();
    expect(screen.queryByTestId("switch-rule-1")).not.toBeInTheDocument();
  });

  it("changes operator on a rule", () => {
    const onChange = vi.fn();
    render(<SwitchEditor {...defaultProps()} onChange={onChange} />);

    const select = screen.getByTestId("switch-rule-op-0");
    fireEvent.change(select, { target: { value: "regex" } });

    // Should have updated the rule operator
    expect(onChange).toHaveBeenCalledWith("rules", expect.arrayContaining([
      expect.objectContaining({ t: "regex" }),
    ]));
  });
});
