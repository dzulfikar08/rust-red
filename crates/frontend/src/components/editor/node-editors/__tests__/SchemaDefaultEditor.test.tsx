import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SchemaDefaultEditor } from "../SchemaDefaultEditor";
import { nodeRegistry } from "@/red/nodes/registry";

describe("SchemaDefaultEditor", () => {
  beforeEach(() => {
    nodeRegistry.clear();
  });

  it("renders fields from the node type's schema", () => {
    nodeRegistry.registerType("json", {
      id: "json",
      type: "json",
      name: "json",
      category: "parser",
      color: "#c0edc0",
      defaults: {
        name: { value: "" },
        action: { value: "" },
        property: { value: "payload" },
      },
      inputs: 1,
      outputs: 1,
    });

    render(
      <SchemaDefaultEditor
        nodeId="n1"
        nodeType="json"
        values={{ name: "My JSON", action: "", property: "payload" }}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("Name")).toHaveValue("My JSON");
    expect(screen.getByLabelText("Property")).toHaveValue("payload");
  });

  it("shows a fallback message for an unknown node type", () => {
    render(
      <SchemaDefaultEditor
        nodeId="n1"
        nodeType="unknown-type"
        values={{}}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getByText(/no schema found/i)).toBeInTheDocument();
  });

  it("hides internal properties like links", () => {
    nodeRegistry.registerType("link in", {
      id: "link in",
      type: "link in",
      name: "link in",
      category: "common",
      color: "#a6bbcf",
      defaults: {
        name: { value: "" },
        links: { value: [] },
      },
      inputs: 0,
      outputs: 1,
    });

    render(
      <SchemaDefaultEditor
        nodeId="n1"
        nodeType="link in"
        values={{ name: "My Link", links: [] }}
        onChange={vi.fn()}
      />,
    );

    // "name" should be visible
    expect(screen.getByLabelText("Name")).toBeInTheDocument();
    // "links" should NOT be rendered
    expect(screen.queryByLabelText("Links")).not.toBeInTheDocument();
  });

  it("passes errors through to SchemaForm", () => {
    nodeRegistry.registerType("test", {
      id: "test",
      type: "test",
      name: "test",
      category: "common",
      color: "#fff",
      defaults: {
        topic: { value: "", required: true },
      },
      inputs: 1,
      outputs: 1,
    });

    render(
      <SchemaDefaultEditor
        nodeId="n1"
        nodeType="test"
        values={{ topic: "" }}
        onChange={vi.fn()}
        errors={{ topic: "Topic is required" }}
      />,
    );

    expect(screen.getByText("Topic is required")).toBeInTheDocument();
  });
});
