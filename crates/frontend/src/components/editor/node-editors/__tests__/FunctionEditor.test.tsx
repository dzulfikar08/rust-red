/**
 * Tests for the FunctionEditor component.
 *
 * Monaco Editor does not render in jsdom, so we mock @monaco-editor/react
 * and verify the wrapper passes the correct props and handles tab switching.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FunctionEditor } from "../FunctionEditor";
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function defaultProps(overrides: Record<string, any> = {}) {
  return {
    nodeId: "n1",
    nodeType: "function",
    values: { func: "\nreturn msg;", outputs: 1, timeout: 0, initialize: "", finalize: "", ...overrides },
    onChange: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("FunctionEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    // Re-register after clearing (module-level registration already ran once,
    // but clearNodeEditors wiped it)
    registerNodeEditor("function", FunctionEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("function")).toBe(true);
  });

  it("renders the tab bar with all four tabs", () => {
    render(<FunctionEditor {...defaultProps()} />);

    expect(screen.getByTestId("function-editor-tabs")).toBeInTheDocument();
    expect(screen.getByTestId("function-tab-setup")).toHaveTextContent("Setup");
    expect(screen.getByTestId("function-tab-init")).toHaveTextContent("On Start");
    expect(screen.getByTestId("function-tab-body")).toHaveTextContent("On Message");
    expect(screen.getByTestId("function-tab-finalize")).toHaveTextContent("On Stop");
  });

  it("shows the On Message tab by default with code editor", () => {
    render(<FunctionEditor {...defaultProps()} />);

    expect(screen.getByTestId("function-body-panel")).toBeInTheDocument();
    const editor = screen.getByTestId("mock-monaco-editor");
    expect(editor.dataset.value).toBe("\nreturn msg;");
    expect(editor.dataset.language).toBe("javascript");
  });

  it("switches to Setup tab and shows outputs/timeout fields", () => {
    render(<FunctionEditor {...defaultProps()} />);

    fireEvent.click(screen.getByTestId("function-tab-setup"));

    expect(screen.getByTestId("function-setup-panel")).toBeInTheDocument();
    expect(screen.getByTestId("function-outputs")).toHaveValue(1);
    expect(screen.getByTestId("function-timeout")).toHaveValue(0);
  });

  it("switches to On Start tab and shows init editor", () => {
    render(<FunctionEditor {...defaultProps({ initialize: "// init code" })} />);

    fireEvent.click(screen.getByTestId("function-tab-init"));

    expect(screen.getByTestId("function-init-panel")).toBeInTheDocument();
    const editor = screen.getByTestId("mock-monaco-editor");
    expect(editor.dataset.value).toBe("// init code");
  });

  it("switches to On Stop tab and shows finalize editor", () => {
    render(<FunctionEditor {...defaultProps({ finalize: "// cleanup" })} />);

    fireEvent.click(screen.getByTestId("function-tab-finalize"));

    expect(screen.getByTestId("function-finalize-panel")).toBeInTheDocument();
    const editor = screen.getByTestId("mock-monaco-editor");
    expect(editor.dataset.value).toBe("// cleanup");
  });

  it("calls onChange when code is edited in On Message tab", () => {
    const onChange = vi.fn();
    render(<FunctionEditor {...defaultProps()} onChange={onChange} />);

    const textarea = screen.getByTestId("mock-monaco-textarea");
    fireEvent.change(textarea, { target: { value: "return null;" } });

    expect(onChange).toHaveBeenCalledWith("func", "return null;");
  });

  it("calls onChange when outputs is changed in Setup tab", () => {
    const onChange = vi.fn();
    render(<FunctionEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.click(screen.getByTestId("function-tab-setup"));

    const outputsInput = screen.getByTestId("function-outputs");
    fireEvent.change(outputsInput, { target: { value: "3" } });

    expect(onChange).toHaveBeenCalledWith("outputs", 3);
  });
});
