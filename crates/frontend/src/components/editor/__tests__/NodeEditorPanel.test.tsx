import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEditorPanelStore } from "../../../store/editor-panel-store";
import { useFlowStore } from "../../../store/flow-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { NodeEditorPanel } from "../NodeEditorPanel";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetEditorPanelStore() {
  useEditorPanelStore.setState({
    isOpen: false,
    isModal: false,
    nodeId: null,
    nodeType: null,
    originalData: {},
    formData: {},
    isDirty: false,
  });
}

function openEditor(overrides?: Partial<{ nodeId: string; nodeType: string; formData: Record<string, unknown> }>) {
  useEditorPanelStore.getState().openEditor(
    overrides?.nodeId ?? "test-node-1",
    overrides?.nodeType ?? "inject",
    overrides?.formData ?? { name: "Test Node", topic: "sensor/data" },
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("NodeEditorPanel", () => {
  beforeEach(() => {
    resetEditorPanelStore();
    nodeRegistry.clear();
  });

  // -----------------------------------------------------------------------
  // Visibility
  // -----------------------------------------------------------------------

  describe("visibility", () => {
    it("renders nothing when editor is closed", () => {
      render(<NodeEditorPanel />);
      expect(screen.queryByTestId("node-editor-panel")).not.toBeInTheDocument();
    });

    it("renders the panel when editor is open", () => {
      openEditor();
      render(<NodeEditorPanel />);
      expect(screen.getByTestId("node-editor-panel")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Header
  // -----------------------------------------------------------------------

  describe("header", () => {
    it("renders a close button", () => {
      openEditor();
      render(<NodeEditorPanel />);
      expect(screen.getByTestId("node-editor-btn-close")).toBeInTheDocument();
    });

    it("renders the title using the node type", () => {
      nodeRegistry.registerType("inject", {
        id: "inject",
        type: "inject",
        name: "inject",
        category: "common",
        color: "#a6bbcf",
        defaults: {},
        inputs: 0,
        outputs: 1,
        paletteLabel: "inject",
      });
      openEditor({ nodeType: "inject" });
      render(<NodeEditorPanel />);

      expect(screen.getByTestId("node-editor-header")).toHaveTextContent(
        "Edit inject node",
      );
    });

    it("renders the toggle-modal button", () => {
      openEditor();
      render(<NodeEditorPanel />);
      expect(screen.getByTestId("node-editor-btn-toggle-modal")).toBeInTheDocument();
    });

    it("falls back to node type string when no palette label", () => {
      openEditor({ nodeType: "unknown-type" });
      render(<NodeEditorPanel />);
      expect(screen.getByTestId("node-editor-header")).toHaveTextContent(
        "Edit unknown-type node",
      );
    });
  });

  // -----------------------------------------------------------------------
  // Body fields
  // -----------------------------------------------------------------------

  describe("body fields", () => {
    it("renders the name field", () => {
      openEditor({ formData: { name: "My Node" } });
      render(<NodeEditorPanel />);
      const nameInput = screen.getByTestId("node-editor-field-name");
      expect(nameInput).toBeInTheDocument();
      expect(nameInput).toHaveValue("My Node");
    });

    it("renders the description textarea", () => {
      openEditor({ formData: { info: "Some description" } });
      render(<NodeEditorPanel />);
      const desc = screen.getByTestId("node-editor-field-info");
      expect(desc).toBeInTheDocument();
      expect(desc).toHaveValue("Some description");
    });

    it("renders dynamic fields from node defaults", () => {
      nodeRegistry.registerType("inject", {
        id: "inject",
        type: "inject",
        name: "inject",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "" },
          payload: { value: "" },
        },
        inputs: 0,
        outputs: 1,
      });
      openEditor({
        nodeType: "inject",
        formData: { name: "Test", topic: "sensor", payload: "data" },
      });
      render(<NodeEditorPanel />);

      expect(screen.getByTestId("node-editor-field-topic")).toBeInTheDocument();
      expect(screen.getByTestId("node-editor-field-payload")).toBeInTheDocument();
    });

    it("updates form field when typing", async () => {
      const user = userEvent.setup();
      openEditor({ formData: { name: "Old" } });
      render(<NodeEditorPanel />);

      const nameInput = screen.getByTestId("node-editor-field-name");
      await user.clear(nameInput);
      await user.type(nameInput, "New Name");

      expect(useEditorPanelStore.getState().formData.name).toBe("New Name");
    });
  });

  // -----------------------------------------------------------------------
  // Footer buttons
  // -----------------------------------------------------------------------

  describe("footer buttons", () => {
    it("renders Cancel and Done buttons", () => {
      openEditor();
      render(<NodeEditorPanel />);
      expect(screen.getByTestId("node-editor-btn-cancel")).toBeInTheDocument();
      expect(screen.getByTestId("node-editor-btn-done")).toBeInTheDocument();
    });

    it("closes the editor on Cancel", async () => {
      const user = userEvent.setup();
      openEditor();
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-cancel"));
      expect(useEditorPanelStore.getState().isOpen).toBe(false);
    });

    it("saves form data to flow store on Done", async () => {
      const user = userEvent.setup();

      // Add a node to the flow store so updateNodeData can find it
      useFlowStore.setState({
        nodes: [
          {
            id: "test-node-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: { name: "Test", type: "inject" },
          },
        ],
      });

      openEditor({ formData: { name: "Updated Node", topic: "new-topic" } });
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-done"));

      // Editor should be closed
      expect(useEditorPanelStore.getState().isOpen).toBe(false);

      // Node data should be updated in flow store
      const node = useFlowStore.getState().nodes.find(
        (n) => n.id === "test-node-1",
      );
      expect(node?.data).toMatchObject({
        name: "Updated Node",
        topic: "new-topic",
      });
    });
  });

  // -----------------------------------------------------------------------
  // Modal mode
  // -----------------------------------------------------------------------

  describe("modal mode", () => {
    it("renders a backdrop overlay when isModal is true", () => {
      openEditor();
      useEditorPanelStore.getState().toggleModal();
      render(<NodeEditorPanel />);

      expect(screen.getByTestId("node-editor-modal-backdrop")).toBeInTheDocument();
    });

    it("does not render backdrop in sidebar mode", () => {
      openEditor();
      render(<NodeEditorPanel />);

      expect(
        screen.queryByTestId("node-editor-modal-backdrop"),
      ).not.toBeInTheDocument();
    });

    it("toggles between sidebar and modal", async () => {
      const user = userEvent.setup();
      openEditor();
      render(<NodeEditorPanel />);

      // Initially sidebar mode
      expect(
        screen.queryByTestId("node-editor-modal-backdrop"),
      ).not.toBeInTheDocument();

      // Toggle to modal
      await user.click(screen.getByTestId("node-editor-btn-toggle-modal"));
      expect(screen.getByTestId("node-editor-modal-backdrop")).toBeInTheDocument();

      // Toggle back to sidebar
      await user.click(screen.getByTestId("node-editor-btn-toggle-modal"));
      expect(
        screen.queryByTestId("node-editor-modal-backdrop"),
      ).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Unsaved changes warning
  // -----------------------------------------------------------------------

  describe("unsaved changes warning", () => {
    it("warns before closing when form is dirty", async () => {
      const confirmMock = vi.fn(() => false);
      vi.spyOn(window, "confirm").mockImplementation(confirmMock);
      const user = userEvent.setup();

      openEditor({ formData: { name: "Original" } });
      render(<NodeEditorPanel />);

      // Make a change
      const nameInput = screen.getByTestId("node-editor-field-name");
      await user.clear(nameInput);
      await user.type(nameInput, "Changed");

      // Try to close via X button
      await user.click(screen.getByTestId("node-editor-btn-close"));

      expect(confirmMock).toHaveBeenCalledWith(
        "You have unsaved changes. Discard and close?",
      );

      // Should stay open if user cancels the confirm
      expect(useEditorPanelStore.getState().isOpen).toBe(true);

      vi.restoreAllMocks();
    });

    it("closes without warning when form is clean", async () => {
      const confirmSpy = vi.spyOn(window, "confirm").mockImplementation(() => true);
      const user = userEvent.setup();

      openEditor({ formData: { name: "Original" } });
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-close"));

      expect(confirmSpy).not.toHaveBeenCalled();
      expect(useEditorPanelStore.getState().isOpen).toBe(false);

      vi.restoreAllMocks();
    });
  });

  // -----------------------------------------------------------------------
  // Close button (X)
  // -----------------------------------------------------------------------

  describe("close button", () => {
    it("closes the editor when not dirty", async () => {
      const user = userEvent.setup();
      openEditor();
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-close"));
      expect(useEditorPanelStore.getState().isOpen).toBe(false);
    });
  });
});
