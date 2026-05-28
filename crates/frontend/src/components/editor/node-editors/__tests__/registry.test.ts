import { describe, it, expect, beforeEach } from "vitest";
import {
  registerNodeEditor,
  getNodeEditor,
  hasCustomEditor,
  unregisterNodeEditor,
  clearNodeEditors,
} from "../registry";

// A stub component for testing
function StubEditor() {
  return null;
}
function AnotherEditor() {
  return null;
}

describe("Node Editor Registry", () => {
  beforeEach(() => {
    clearNodeEditors();
  });

  it("registers and retrieves an editor component", () => {
    registerNodeEditor("test-type", StubEditor);
    expect(getNodeEditor("test-type")).toBe(StubEditor);
  });

  it("returns null for an unregistered type", () => {
    expect(getNodeEditor("nonexistent")).toBeNull();
  });

  it("reports whether a custom editor exists", () => {
    expect(hasCustomEditor("my-type")).toBe(false);
    registerNodeEditor("my-type", StubEditor);
    expect(hasCustomEditor("my-type")).toBe(true);
  });

  it("allows unregistering an editor", () => {
    registerNodeEditor("temp", StubEditor);
    expect(hasCustomEditor("temp")).toBe(true);
    unregisterNodeEditor("temp");
    expect(hasCustomEditor("temp")).toBe(false);
    expect(getNodeEditor("temp")).toBeNull();
  });

  it("overwrites an existing registration", () => {
    registerNodeEditor("overwrite", StubEditor);
    registerNodeEditor("overwrite", AnotherEditor);
    expect(getNodeEditor("overwrite")).toBe(AnotherEditor);
  });

  it("clears all registrations", () => {
    registerNodeEditor("a", StubEditor);
    registerNodeEditor("b", AnotherEditor);
    clearNodeEditors();
    expect(hasCustomEditor("a")).toBe(false);
    expect(hasCustomEditor("b")).toBe(false);
  });
});
