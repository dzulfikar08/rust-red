/**
 * Tests for the HttpRequestEditor component.
 *
 * Tests rendering of all fields, collapsible sections, header list, and form changes.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useState } from "react";
import { HttpRequestEditor } from "../HttpRequestEditor";
import { registerNodeEditor, clearNodeEditors, hasCustomEditor } from "../registry";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function defaultProps(overrides: Record<string, any> = {}) {
  return {
    nodeId: "n1",
    nodeType: "http request",
    values: {
      method: "GET",
      url: "https://example.com",
      ret: "txt",
      paytoqs: "ignore",
      persist: false,
      tls: "",
      proxy: "",
      authType: "",
      user: "",
      password: "",
      senderr: false,
      headers: [],
      ...overrides,
    },
    onChange: vi.fn(),
  };
}

/** Stateful wrapper for testing header add/remove */
function StatefulHttpRequestEditor(props: any) {
  const [values, setValues] = useState(props.values);
  const handleChange = (key: string, value: any) => {
    setValues((prev: any) => ({ ...prev, [key]: value }));
    props.onChange(key, value);
  };
  return <HttpRequestEditor {...props} values={values} onChange={handleChange} />;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("HttpRequestEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearNodeEditors();
    registerNodeEditor("http request", HttpRequestEditor);
  });

  it("registers itself in the registry", () => {
    expect(hasCustomEditor("http request")).toBe(true);
  });

  it("renders method, URL, return type, and header section", () => {
    render(<HttpRequestEditor {...defaultProps()} />);

    expect(screen.getByTestId("httprequest-editor")).toBeInTheDocument();
    expect(screen.getByTestId("httprequest-method")).toHaveValue("GET");
    expect(screen.getByTestId("httprequest-url")).toHaveValue("https://example.com");
    expect(screen.getByTestId("httprequest-ret")).toHaveValue("txt");
  });

  it("shows payload selector only for GET method", () => {
    const { rerender } = render(<HttpRequestEditor {...defaultProps()} />);

    // GET method should show paytoqs
    expect(screen.getByTestId("httprequest-paytoqs")).toBeInTheDocument();

    // Switch to POST
    rerender(<HttpRequestEditor {...defaultProps({ method: "POST" })} />);
    expect(screen.queryByTestId("httprequest-paytoqs")).not.toBeInTheDocument();
  });

  it("calls onChange when method changes", () => {
    const onChange = vi.fn();
    render(<HttpRequestEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("httprequest-method"), {
      target: { value: "POST" },
    });

    expect(onChange).toHaveBeenCalledWith("method", "POST");
  });

  it("calls onChange when URL changes", () => {
    const onChange = vi.fn();
    render(<HttpRequestEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("httprequest-url"), {
      target: { value: "https://api.example.com/data" },
    });

    expect(onChange).toHaveBeenCalledWith("url", "https://api.example.com/data");
  });

  it("calls onChange when return type changes", () => {
    const onChange = vi.fn();
    render(<HttpRequestEditor {...defaultProps()} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("httprequest-ret"), {
      target: { value: "obj" },
    });

    expect(onChange).toHaveBeenCalledWith("ret", "obj");
  });

  it("toggles TLS section on and off", () => {
    render(<HttpRequestEditor {...defaultProps()} />);

    // Initially TLS config should not be visible
    expect(screen.queryByTestId("httprequest-tls")).not.toBeInTheDocument();

    // Check the TLS checkbox
    fireEvent.click(screen.getByTestId("httprequest-usetls"));

    // TLS config should now be visible
    expect(screen.getByTestId("httprequest-tls")).toBeInTheDocument();
  });

  it("toggles auth section on and off", () => {
    render(<HttpRequestEditor {...defaultProps()} />);

    // Initially auth should not be visible
    expect(screen.queryByTestId("httprequest-authtype")).not.toBeInTheDocument();

    // Check the auth checkbox
    fireEvent.click(screen.getByTestId("httprequest-useauth"));

    // Auth section should now be visible
    expect(screen.getByTestId("httprequest-authtype")).toBeInTheDocument();
  });

  it("shows username/password for basic auth", () => {
    render(<HttpRequestEditor {...defaultProps({ authType: "basic" })} />);

    // Auth section should be shown since authType is set (showAuth initializes from useAuth)
    expect(screen.getByTestId("httprequest-authtype")).toBeInTheDocument();

    // Basic auth should show user/password fields
    expect(screen.getByTestId("httprequest-user")).toBeInTheDocument();
    expect(screen.getByTestId("httprequest-password")).toBeInTheDocument();
  });

  it("shows token field for bearer auth", () => {
    render(
      <StatefulHttpRequestEditor {...defaultProps({ authType: "bearer" })} />,
    );

    // Auth section should be shown since authType is set
    expect(screen.getByTestId("httprequest-authtype")).toBeInTheDocument();

    // Bearer auth should show token field
    expect(screen.getByTestId("httprequest-token")).toBeInTheDocument();
  });

  it("adds a header row", () => {
    render(<StatefulHttpRequestEditor {...defaultProps()} />);

    expect(screen.getByTestId("httprequest-headers-list")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("httprequest-add-header"));

    expect(screen.getByTestId("httprequest-header-0")).toBeInTheDocument();
  });

  it("removes a header row", () => {
    render(
      <StatefulHttpRequestEditor
        {...defaultProps({
          headers: [
            { keyType: "other", keyValue: "Content-Type", valueType: "other", valueValue: "application/json" },
          ],
        })}
      />,
    );

    expect(screen.getByTestId("httprequest-header-0")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("httprequest-remove-header-0"));

    expect(screen.queryByTestId("httprequest-header-0")).not.toBeInTheDocument();
  });
});
