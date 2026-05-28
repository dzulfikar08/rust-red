import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useFlowStore } from "../../../store/flow-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { ConfigSelector, ADD_NEW_SENTINEL } from "../ConfigSelector";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetFlowStore() {
  useFlowStore.setState({ nodes: [], edges: [] });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ConfigSelector", () => {
  beforeEach(() => {
    resetFlowStore();
    nodeRegistry.clear();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders a dropdown with a testid", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );
      expect(screen.getByTestId("config-selector")).toBeInTheDocument();
    });

    it("renders a label when provided", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
          label="Broker"
        />,
      );
      expect(screen.getByText("Broker")).toBeInTheDocument();
    });

    it("renders the placeholder option", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );
      expect(screen.getByText("-- select --")).toBeInTheDocument();
    });

    it("lists existing config nodes of the specified type", () => {
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Local Broker",
            },
          },
          {
            id: "cfg-2",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Remote Broker",
            },
          },
        ],
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );

      expect(screen.getByText("Local Broker")).toBeInTheDocument();
      expect(screen.getByText("Remote Broker")).toBeInTheDocument();
    });

    it("does not list config nodes of a different type", () => {
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "My Broker",
            },
          },
          {
            id: "cfg-2",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "tls-config",
              name: "My TLS",
            },
          },
        ],
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );

      expect(screen.getByText("My Broker")).toBeInTheDocument();
      expect(screen.queryByText("My TLS")).not.toBeInTheDocument();
    });

    it("renders the 'Add new' option using the type paletteLabel", () => {
      nodeRegistry.registerType("mqtt-broker", {
        id: "mqtt-broker",
        type: "mqtt-broker",
        name: "mqtt-broker",
        category: "config",
        color: "#e2d96e",
        defaults: {},
        inputs: 0,
        outputs: 0,
        paletteLabel: "mqtt-broker",
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );

      expect(
        screen.getByText(`Add new mqtt-broker...`),
      ).toBeInTheDocument();
    });

    it("falls back to nodeType when no registry entry exists", () => {
      render(
        <ConfigSelector
          nodeType="my-custom-config"
          value=""
          onChange={() => {}}
        />,
      );

      expect(
        screen.getByText("Add new my-custom-config..."),
      ).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Selection
  // -----------------------------------------------------------------------

  describe("selection", () => {
    it("calls onChange when a config is selected", async () => {
      const user = userEvent.setup();
      const handleChange = vi.fn();

      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Broker A",
            },
          },
        ],
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={handleChange}
        />,
      );

      const select = screen.getByTestId("config-selector");
      await user.selectOptions(select, "cfg-1");

      expect(handleChange).toHaveBeenCalledWith("cfg-1");
    });

    it("shows the selected value", () => {
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Broker A",
            },
          },
        ],
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value="cfg-1"
          onChange={() => {}}
        />,
      );

      const select = screen.getByTestId("config-selector") as HTMLSelectElement;
      expect(select.value).toBe("cfg-1");
    });
  });

  // -----------------------------------------------------------------------
  // Edit button
  // -----------------------------------------------------------------------

  describe("edit button", () => {
    it("renders an edit button", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value="cfg-1"
          onChange={() => {}}
        />,
      );
      expect(
        screen.getByTestId("config-selector-edit-btn"),
      ).toBeInTheDocument();
    });

    it("is disabled when no value is selected", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );
      expect(screen.getByTestId("config-selector-edit-btn")).toBeDisabled();
    });

    it("is enabled when a value is selected", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value="cfg-1"
          onChange={() => {}}
        />,
      );
      expect(screen.getByTestId("config-selector-edit-btn")).not.toBeDisabled();
    });

    it("opens the config editor overlay when clicked", async () => {
      const user = userEvent.setup();

      nodeRegistry.registerType("mqtt-broker", {
        id: "mqtt-broker",
        type: "mqtt-broker",
        name: "mqtt-broker",
        category: "config",
        color: "#e2d96e",
        defaults: { url: { value: "" } },
        inputs: 0,
        outputs: 0,
        paletteLabel: "mqtt-broker",
      });

      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Broker A",
              url: "localhost:1883",
            },
          },
        ],
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value="cfg-1"
          onChange={() => {}}
        />,
      );

      await user.click(screen.getByTestId("config-selector-edit-btn"));
      expect(screen.getByTestId("config-editor-overlay")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Add new
  // -----------------------------------------------------------------------

  describe("add new", () => {
    it("opens the config editor overlay when 'Add new' is selected", async () => {
      const user = userEvent.setup();

      nodeRegistry.registerType("mqtt-broker", {
        id: "mqtt-broker",
        type: "mqtt-broker",
        name: "mqtt-broker",
        category: "config",
        color: "#e2d96e",
        defaults: { url: { value: "" } },
        inputs: 0,
        outputs: 0,
        paletteLabel: "mqtt-broker",
      });

      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
        />,
      );

      const select = screen.getByTestId("config-selector");
      await user.selectOptions(select, ADD_NEW_SENTINEL);

      expect(screen.getByTestId("config-editor-overlay")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Disabled state
  // -----------------------------------------------------------------------

  describe("disabled state", () => {
    it("disables the dropdown when disabled prop is true", () => {
      render(
        <ConfigSelector
          nodeType="mqtt-broker"
          value=""
          onChange={() => {}}
          disabled
        />,
      );
      expect(screen.getByTestId("config-selector")).toBeDisabled();
    });
  });
});
