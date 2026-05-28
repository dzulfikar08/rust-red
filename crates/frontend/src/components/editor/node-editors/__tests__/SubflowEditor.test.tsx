/**
 * Tests for the SubflowEditor component.
 *
 * Verifies rendering of all form fields and interactive editing
 * of subflow properties: name, category, color, icon, description,
 * inputs, outputs, and environment variables.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SubflowEditor } from "../SubflowEditor";
import { registerNodeEditor, clearNodeEditors, hasCustomEditor } from "../registry";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function defaultProps(overrides: Record<string, any> = {}) {
  return {
    nodeId: "sf1",
    nodeType: "subflow",
    values: {
      name: "",
      category: "function",
      color: "#da9aaa",
      icon: "debugger",
      description: "",
      in: 1,
      out: 1,
      env: [],
      ...overrides,
    },
    onChange: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SubflowEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    registerNodeEditor("subflow", SubflowEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("subflow")).toBe(true);
  });

  it("renders the name field", () => {
    render(<SubflowEditor {...defaultProps({ name: "Test Subflow" })} />);
    expect(screen.getByTestId("subflow-name")).toHaveValue("Test Subflow");
  });

  it("renders the category select", () => {
    render(<SubflowEditor {...defaultProps({ category: "network" })} />);
    expect(screen.getByTestId("subflow-category")).toHaveValue("network");
  });

  it("renders color swatches", () => {
    render(<SubflowEditor {...defaultProps()} />);
    expect(screen.getByTestId("subflow-colors")).toBeInTheDocument();
  });

  it("renders icon buttons", () => {
    render(<SubflowEditor {...defaultProps()} />);
    expect(screen.getByTestId("subflow-icons")).toBeInTheDocument();
  });

  it("renders description textarea", () => {
    render(<SubflowEditor {...defaultProps({ description: "A test subflow" })} />);
    expect(screen.getByTestId("subflow-description")).toHaveValue("A test subflow");
  });

  it("renders input and output fields", () => {
    render(<SubflowEditor {...defaultProps({ in: 2, out: 3 })} />);
    expect(screen.getByTestId("subflow-in")).toHaveValue(2);
    expect(screen.getByTestId("subflow-out")).toHaveValue(3);
  });

  it("calls onChange when name is edited", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-name"), {
      target: { value: "New Name" },
    });
    expect(onChange).toHaveBeenCalledWith("name", "New Name");
  });

  it("calls onChange when category is changed", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-category"), {
      target: { value: "network" },
    });
    expect(onChange).toHaveBeenCalledWith("category", "network");
  });

  it("calls onChange when description is changed", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-description"), {
      target: { value: "Updated description" },
    });
    expect(onChange).toHaveBeenCalledWith("description", "Updated description");
  });

  it("calls onChange when inputs is changed", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-in"), {
      target: { value: "3" },
    });
    expect(onChange).toHaveBeenCalledWith("in", 3);
  });

  it("calls onChange when outputs is changed", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-out"), {
      target: { value: "4" },
    });
    expect(onChange).toHaveBeenCalledWith("out", 4);
  });

  it("calls onChange when a color swatch is clicked", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.click(screen.getByTestId("subflow-color-a6bbcf"));
    expect(onChange).toHaveBeenCalledWith("color", "#a6bbcf");
  });

  it("calls onChange when an icon is clicked", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.click(screen.getByTestId("subflow-icon-feed"));
    expect(onChange).toHaveBeenCalledWith("icon", "feed");
  });

  it("clamps inputs to valid range (0-20)", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    // Value above max
    fireEvent.change(screen.getByTestId("subflow-in"), {
      target: { value: "50" },
    });
    expect(onChange).toHaveBeenCalledWith("in", 20);
  });

  it("clamps outputs to 0 when NaN", () => {
    const onChange = vi.fn();
    render(<SubflowEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("subflow-out"), {
      target: { value: "abc" },
    });
    expect(onChange).toHaveBeenCalledWith("out", 0);
  });

  // -----------------------------------------------------------------------
  // Environment variables
  // -----------------------------------------------------------------------

  describe("environment variables", () => {
    it("shows env toggle button with count", () => {
      render(
        <SubflowEditor
          {...defaultProps({
            env: [
              { name: "A", value: "1", type: "str" },
              { name: "B", value: "2", type: "num" },
            ],
          })}
        />,
      );
      expect(screen.getByTestId("subflow-env-toggle")).toHaveTextContent(
        "Environment Variables (2)",
      );
    });

    it("toggles env panel visibility", () => {
      render(
        <SubflowEditor
          {...defaultProps({
            env: [{ name: "X", value: "1", type: "str" }],
          })}
        />,
      );

      // Panel is hidden initially
      expect(screen.queryByTestId("subflow-env-panel")).not.toBeInTheDocument();

      // Click toggle to show
      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      expect(screen.getByTestId("subflow-env-panel")).toBeInTheDocument();

      // Click toggle to hide
      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      expect(screen.queryByTestId("subflow-env-panel")).not.toBeInTheDocument();
    });

    it("renders existing env vars when panel is open", () => {
      render(
        <SubflowEditor
          {...defaultProps({
            env: [
              { name: "API_KEY", value: "secret", type: "str" },
              { name: "COUNT", value: "10", type: "num" },
            ],
          })}
        />,
      );

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));

      expect(screen.getByTestId("subflow-env-name-0")).toHaveValue("API_KEY");
      expect(screen.getByTestId("subflow-env-value-0")).toHaveValue("secret");
      expect(screen.getByTestId("subflow-env-type-0")).toHaveValue("str");
      expect(screen.getByTestId("subflow-env-name-1")).toHaveValue("COUNT");
    });

    it("calls onChange when adding an env var", () => {
      const onChange = vi.fn();
      render(<SubflowEditor {...defaultProps({ env: [] })} onChange={onChange} />);

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      fireEvent.click(screen.getByTestId("subflow-env-add"));

      expect(onChange).toHaveBeenCalledWith("env", [
        { name: "", value: "", type: "str" },
      ]);
    });

    it("calls onChange when removing an env var", () => {
      const onChange = vi.fn();
      const envVars = [
        { name: "A", value: "1", type: "str" },
        { name: "B", value: "2", type: "num" },
      ];
      render(
        <SubflowEditor {...defaultProps({ env: envVars })} onChange={onChange} />,
      );

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      fireEvent.click(screen.getByTestId("subflow-env-remove-0"));

      expect(onChange).toHaveBeenCalledWith("env", [
        { name: "B", value: "2", type: "num" },
      ]);
    });

    it("calls onChange when editing an env var name", () => {
      const onChange = vi.fn();
      const envVars = [{ name: "OLD", value: "1", type: "str" }];
      render(
        <SubflowEditor {...defaultProps({ env: envVars })} onChange={onChange} />,
      );

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      fireEvent.change(screen.getByTestId("subflow-env-name-0"), {
        target: { value: "NEW" },
      });

      expect(onChange).toHaveBeenCalledWith("env", [
        { name: "NEW", value: "1", type: "str" },
      ]);
    });

    it("calls onChange when editing an env var value", () => {
      const onChange = vi.fn();
      const envVars = [{ name: "X", value: "old", type: "str" }];
      render(
        <SubflowEditor {...defaultProps({ env: envVars })} onChange={onChange} />,
      );

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      fireEvent.change(screen.getByTestId("subflow-env-value-0"), {
        target: { value: "new" },
      });

      expect(onChange).toHaveBeenCalledWith("env", [
        { name: "X", value: "new", type: "str" },
      ]);
    });

    it("calls onChange when changing env var type", () => {
      const onChange = vi.fn();
      const envVars = [{ name: "X", value: "1", type: "str" }];
      render(
        <SubflowEditor {...defaultProps({ env: envVars })} onChange={onChange} />,
      );

      fireEvent.click(screen.getByTestId("subflow-env-toggle"));
      fireEvent.change(screen.getByTestId("subflow-env-type-0"), {
        target: { value: "num" },
      });

      expect(onChange).toHaveBeenCalledWith("env", [
        { name: "X", value: "1", type: "num" },
      ]);
    });
  });
});
