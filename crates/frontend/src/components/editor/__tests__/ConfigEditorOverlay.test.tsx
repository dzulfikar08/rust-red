import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useFlowStore } from "../../../store/flow-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { ConfigEditorOverlay } from "../ConfigEditorOverlay";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetFlowStore() {
  useFlowStore.setState({ nodes: [], edges: [] });
}

function registerMqttBroker() {
  nodeRegistry.registerType("mqtt-broker", {
    id: "mqtt-broker",
    type: "mqtt-broker",
    name: "mqtt-broker",
    category: "config",
    color: "#e2d96e",
    defaults: {
      url: { value: "" },
      port: { value: 1883 },
    },
    inputs: 0,
    outputs: 0,
    paletteLabel: "mqtt-broker",
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ConfigEditorOverlay", () => {
  beforeEach(() => {
    resetFlowStore();
    nodeRegistry.clear();
  });

  // -----------------------------------------------------------------------
  // Rendering (create mode)
  // -----------------------------------------------------------------------

  describe("create mode", () => {
    it("renders the overlay", () => {
      registerMqttBroker();
      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );
      expect(screen.getByTestId("config-editor-overlay")).toBeInTheDocument();
    });

    it("renders the dialog", () => {
      registerMqttBroker();
      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );
      expect(screen.getByTestId("config-editor-dialog")).toBeInTheDocument();
    });

    it("shows 'Add' in the header for new configs", () => {
      registerMqttBroker();
      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );
      expect(screen.getByTestId("config-editor-header")).toHaveTextContent(
        "Add mqtt-broker config",
      );
    });

    it("renders the name field", () => {
      registerMqttBroker();
      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );
      expect(
        screen.getByTestId("config-editor-field-name"),
      ).toBeInTheDocument();
    });

    it("renders Cancel and Add buttons", () => {
      registerMqttBroker();
      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );
      expect(
        screen.getByTestId("config-editor-btn-cancel"),
      ).toBeInTheDocument();
      expect(
        screen.getByTestId("config-editor-btn-save"),
      ).toBeInTheDocument();
      expect(screen.getByTestId("config-editor-btn-save")).toHaveTextContent(
        "Add",
      );
    });

    it("calls onCancel when Cancel is clicked", async () => {
      const user = userEvent.setup();
      const handleCancel = vi.fn();
      registerMqttBroker();

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={handleCancel}
        />,
      );

      await user.click(screen.getByTestId("config-editor-btn-cancel"));
      expect(handleCancel).toHaveBeenCalledOnce();
    });

    it("calls onCancel when close (X) button is clicked", async () => {
      const user = userEvent.setup();
      const handleCancel = vi.fn();
      registerMqttBroker();

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={() => {}}
          onCancel={handleCancel}
        />,
      );

      await user.click(screen.getByTestId("config-editor-btn-close"));
      expect(handleCancel).toHaveBeenCalledOnce();
    });

    it("creates a new config node on save", async () => {
      const user = userEvent.setup();
      const handleSave = vi.fn();
      registerMqttBroker();

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId={null}
          onSave={handleSave}
          onCancel={() => {}}
        />,
      );

      // Fill in the name
      const nameInput = screen.getByTestId("config-editor-field-name");
      await user.clear(nameInput);
      await user.type(nameInput, "Test Broker");

      // Save
      await user.click(screen.getByTestId("config-editor-btn-save"));

      // Should have called onSave with a new ID
      expect(handleSave).toHaveBeenCalledOnce();
      const savedId = handleSave.mock.calls[0][0] as string;
      expect(savedId).toMatch(/^config-mqtt-broker-/);

      // The node should be added to the flow store
      const node = useFlowStore.getState().nodes.find((n) => n.id === savedId);
      expect(node).toBeDefined();
      expect(node?.data).toMatchObject({
        _isConfig: true,
        type: "mqtt-broker",
        name: "Test Broker",
      });
    });
  });

  // -----------------------------------------------------------------------
  // Rendering (edit mode)
  // -----------------------------------------------------------------------

  describe("edit mode", () => {
    it("shows 'Edit' in the header for existing configs", () => {
      registerMqttBroker();
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Existing Broker",
              url: "broker.example.com",
            },
          },
        ],
      });

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId="cfg-1"
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );

      expect(screen.getByTestId("config-editor-header")).toHaveTextContent(
        "Edit mqtt-broker config",
      );
    });

    it("pre-populates fields from existing config data", () => {
      registerMqttBroker();
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Existing Broker",
              url: "broker.example.com",
            },
          },
        ],
      });

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId="cfg-1"
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );

      expect(screen.getByTestId("config-editor-field-name")).toHaveValue(
        "Existing Broker",
      );
    });

    it("shows 'Update' button in edit mode", () => {
      registerMqttBroker();
      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Existing Broker",
            },
          },
        ],
      });

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId="cfg-1"
          onSave={() => {}}
          onCancel={() => {}}
        />,
      );

      expect(screen.getByTestId("config-editor-btn-save")).toHaveTextContent(
        "Update",
      );
    });

    it("updates the existing node on save", async () => {
      const user = userEvent.setup();
      const handleSave = vi.fn();
      registerMqttBroker();

      useFlowStore.setState({
        nodes: [
          {
            id: "cfg-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: {
              _isConfig: true,
              type: "mqtt-broker",
              name: "Old Name",
              url: "old.example.com",
            },
          },
        ],
      });

      render(
        <ConfigEditorOverlay
          configType="mqtt-broker"
          configId="cfg-1"
          onSave={handleSave}
          onCancel={() => {}}
        />,
      );

      // Change the name
      const nameInput = screen.getByTestId("config-editor-field-name");
      await user.clear(nameInput);
      await user.type(nameInput, "New Name");

      // Save
      await user.click(screen.getByTestId("config-editor-btn-save"));

      expect(handleSave).toHaveBeenCalledWith("cfg-1");

      // Node data should be updated
      const node = useFlowStore
        .getState()
        .nodes.find((n) => n.id === "cfg-1");
      expect(node?.data).toMatchObject({
        _isConfig: true,
        type: "mqtt-broker",
        name: "New Name",
      });
    });
  });
});
