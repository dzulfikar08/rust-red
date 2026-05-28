/**
 * Tests for the TemplateEditor component.
 *
 * Monaco Editor does not render in jsdom, so we mock @monaco-editor/react.
 * TypedInput is mocked to simplify testing.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TemplateEditor } from "../TemplateEditor";
import { registerNodeEditor, clearNodeEditors, hasCustomEditor } from "../registry";

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock("@monaco-editor/react", () => ({
  default: vi.fn(({ value, onChange, language }) => (
    <div
      data-testid="mock-monaco-editor"
      data-language={language}
      data-value={value}
    >
      <textarea
        data-testid="mock-monaco-textarea"
        value={value}
        onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
          onChange?.(e.target.value)
        }
      />
    </div>
  )),
}));

vi.mock("@/store/theme-store", () => ({
  useThemeStore: vi.fn((selector: (s: any) => any) =>
    selector({ theme: "dark", setTheme: vi.fn(), toggleTheme: vi.fn() }),
  ),
}));

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
    nodeType: "template",
    values: {
      field: "payload",
      fieldType: "msg",
      format: "handlebars",
      syntax: "mustache",
      template: "This is the payload: {{payload}} !",
      output: "str",
      ...overrides,
    },
    onChange: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("TemplateEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    registerNodeEditor("template", TemplateEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("template")).toBe(true);
  });

  it("renders all form fields", () => {
    render(<TemplateEditor {...defaultProps()} />);

    expect(screen.getByTestId("template-editor")).toBeInTheDocument();
    expect(screen.getByTestId("template-format")).toBeInTheDocument();
    expect(screen.getByTestId("template-syntax")).toBeInTheDocument();
    expect(screen.getByTestId("template-output")).toBeInTheDocument();
  });

  it("renders the code editor with default template value", () => {
    render(<TemplateEditor {...defaultProps()} />);

    const editor = screen.getByTestId("mock-monaco-editor");
    expect(editor.dataset.value).toBe("This is the payload: {{payload}} !");
  });

  it("calls onChange when format is changed", () => {
    const onChange = vi.fn();
    render(<TemplateEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("template-format"), {
      target: { value: "json" },
    });

    expect(onChange).toHaveBeenCalledWith("format", "json");
  });

  it("calls onChange when syntax is changed", () => {
    const onChange = vi.fn();
    render(<TemplateEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("template-syntax"), {
      target: { value: "plain" },
    });

    expect(onChange).toHaveBeenCalledWith("syntax", "plain");
  });

  it("calls onChange when output is changed", () => {
    const onChange = vi.fn();
    render(<TemplateEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("template-output"), {
      target: { value: "json" },
    });

    expect(onChange).toHaveBeenCalledWith("output", "json");
  });

  it("calls onChange when template code is edited", () => {
    const onChange = vi.fn();
    render(<TemplateEditor {...defaultProps()} onChange={onChange} />);

    const textarea = screen.getByTestId("mock-monaco-textarea");
    fireEvent.change(textarea, { target: { value: "Hello {{name}}" } });

    expect(onChange).toHaveBeenCalledWith("template", "Hello {{name}}");
  });
});
