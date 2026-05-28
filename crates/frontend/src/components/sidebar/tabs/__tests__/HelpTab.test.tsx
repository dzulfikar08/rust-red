import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { HelpTab } from "../HelpTab";
import { useEditorStore } from "../../../../store/editor-store";
import { nodeRegistry } from "../../../../red/nodes/registry";
import { helpContentMap } from "../help-content";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useEditorStore.setState({ selectedNodeId: null });
}

/** Register a minimal node definition so HelpTab can resolve title/colour. */
function registerTestNode(
  type: string,
  overrides: Record<string, unknown> = {},
) {
  nodeRegistry.registerType(type, {
    id: type,
    type,
    name: type,
    category: "common",
    color: "#a6bbcf",
    defaults: {},
    inputs: 1,
    outputs: 1,
    paletteLabel: type.charAt(0).toUpperCase() + type.slice(1),
    ...overrides,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("HelpTab", () => {
  beforeEach(() => {
    resetStore();
    nodeRegistry.clear();
  });

  // -----------------------------------------------------------------------
  // General help (no node selected)
  // -----------------------------------------------------------------------

  describe("no node selected", () => {
    it("renders without crashing", () => {
      render(<HelpTab />);
      expect(screen.getByTestId("sidebar-tab-content-help")).toBeInTheDocument();
    });

    it("shows general help title", () => {
      render(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent(
        "Node-RED Help",
      );
    });

    it("shows the general help description", () => {
      render(<HelpTab />);
      const desc = screen.getByTestId("help-description");
      expect(desc.textContent).toContain("flow-based programming");
    });

    it("shows a 'no node selected' section", () => {
      render(<HelpTab />);
      expect(screen.getByTestId("help-no-selection")).toBeInTheDocument();
      expect(screen.getByTestId("help-no-selection").textContent).toContain(
        "Select a node",
      );
    });

    it("shows keyboard shortcuts in details", () => {
      render(<HelpTab />);
      const details = screen.getByTestId("help-details");
      expect(details.textContent).toContain("Keyboard Shortcuts");
      expect(details.textContent).toContain("Ctrl-Space");
    });

    it("shows references", () => {
      render(<HelpTab />);
      const refs = screen.getByTestId("help-references");
      expect(refs.textContent).toContain("Node-RED Documentation");
      expect(refs.textContent).toContain("Flow Library");
    });
  });

  // -----------------------------------------------------------------------
  // Node type selected (with registry entry)
  // -----------------------------------------------------------------------

  describe("node selected with registry entry", () => {
    it("shows node type title from paletteLabel", () => {
      registerTestNode("inject", { paletteLabel: "inject" });
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent("inject");
    });

    it("shows category colour dot", () => {
      registerTestNode("inject", { color: "#a6bbcf" });
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const dot = screen.getByTestId("help-category-dot");
      expect(dot).toBeInTheDocument();
      // The dot should have the color from the registry
      expect(dot.style.backgroundColor).toBeTruthy();
    });

    it("does not show 'no node selected' section", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      expect(screen.queryByTestId("help-no-selection")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Inject node help content
  // -----------------------------------------------------------------------

  describe("inject node help", () => {
    it("shows inject description", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const desc = screen.getByTestId("help-description");
      expect(desc.textContent).toContain("Injects a message into a flow");
    });

    it("shows properties section", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const props = screen.getByTestId("help-properties");
      expect(props.textContent).toContain("payload");
      expect(props.textContent).toContain("topic");
    });

    it("shows outputs section", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const outputs = screen.getByTestId("help-outputs");
      expect(outputs.textContent).toContain("payload");
    });

    it("shows details section with HTML content", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const details = screen.getByTestId("help-details");
      expect(details.textContent).toContain("Inject node can initiate");
    });

    it("shows references with links", () => {
      registerTestNode("inject");
      useEditorStore.getState().selectNode("inject");

      render(<HelpTab />);
      const refs = screen.getByTestId("help-references");
      expect(refs.textContent).toContain("JSONata");
      expect(refs.textContent).toContain("JavaScript");
      // Links should be rendered
      const link = refs.querySelector("a");
      expect(link).not.toBeNull();
      expect(link?.getAttribute("href")).toBe("https://jsonata.org/");
    });
  });

  // -----------------------------------------------------------------------
  // Node selected but not in registry (falls back to type as title)
  // -----------------------------------------------------------------------

  describe("node selected without registry entry", () => {
    it("uses the type string as title when not in registry", () => {
      useEditorStore.getState().selectNode("custom-node");

      render(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent(
        "custom-node",
      );
    });

    it("falls back to general help when type is not in help map", () => {
      useEditorStore.getState().selectNode("unknown-node");

      render(<HelpTab />);
      // Should show general help content since type is not in helpContentMap
      const desc = screen.getByTestId("help-description");
      expect(desc.textContent).toContain("flow-based programming");
    });
  });

  // -----------------------------------------------------------------------
  // Debug node (no outputs)
  // -----------------------------------------------------------------------

  describe("debug node help", () => {
    it("shows debug description", () => {
      registerTestNode("debug");
      useEditorStore.getState().selectNode("debug");

      render(<HelpTab />);
      const desc = screen.getByTestId("help-description");
      expect(desc.textContent).toContain("display messages in the debug sidebar");
    });

    it("does not show outputs section (empty outputs)", () => {
      registerTestNode("debug");
      useEditorStore.getState().selectNode("debug");

      render(<HelpTab />);
      expect(screen.queryByTestId("help-outputs")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Help content map coverage
  // -----------------------------------------------------------------------

  describe("help content map", () => {
    it("contains help for all expected built-in types", () => {
      const expectedTypes = [
        "inject",
        "debug",
        "function",
        "change",
        "switch",
        "template",
        "http request",
        "delay",
        "trigger",
        "comment",
      ];
      for (const type of expectedTypes) {
        expect(helpContentMap[type]).toBeDefined();
        expect(helpContentMap[type].description.length).toBeGreaterThan(0);
      }
    });

    it("each entry has a non-empty description", () => {
      for (const [type, entry] of Object.entries(helpContentMap)) {
        expect(entry.description.length, `description for ${type}`).toBeGreaterThan(0);
      }
    });
  });

  // -----------------------------------------------------------------------
  // HTML sanitization
  // -----------------------------------------------------------------------

  describe("HTML sanitization", () => {
    it("renders allowed HTML tags in details", () => {
      registerTestNode("function");
      useEditorStore.getState().selectNode("function");

      render(<HelpTab />);
      const htmlContent = screen.getByTestId("help-html-content");
      // Should contain list items rendered from HTML
      expect(htmlContent.textContent).toContain("msg");
      expect(htmlContent.textContent).toContain("node");
    });
  });

  // -----------------------------------------------------------------------
  // Selection changes
  // -----------------------------------------------------------------------

  describe("selection changes", () => {
    it("updates help when selection changes", () => {
      registerTestNode("inject");
      registerTestNode("debug");

      const { rerender } = render(<HelpTab />);

      // Initially no selection - general help
      expect(screen.getByTestId("help-title")).toHaveTextContent(
        "Node-RED Help",
      );

      // Select inject (registerTestNode creates paletteLabel "Inject")
      useEditorStore.getState().selectNode("inject");
      rerender(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent("Inject");

      // Select debug (registerTestNode creates paletteLabel "Debug")
      useEditorStore.getState().selectNode("debug");
      rerender(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent("Debug");

      // Deselect
      useEditorStore.getState().selectNode(null);
      rerender(<HelpTab />);
      expect(screen.getByTestId("help-title")).toHaveTextContent(
        "Node-RED Help",
      );
    });
  });
});
