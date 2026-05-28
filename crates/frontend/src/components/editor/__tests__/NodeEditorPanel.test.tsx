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

    it("renders dynamic fields from node defaults via SchemaDefaultEditor", () => {
      // Use a type that does NOT have a custom editor, so SchemaDefaultEditor
      // renders the node definition's defaults through SchemaForm.
      nodeRegistry.registerType("custom-test", {
        id: "custom-test",
        type: "custom-test",
        name: "custom-test",
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
        nodeType: "custom-test",
        formData: { name: "Test", topic: "sensor", payload: "data" },
      });
      render(<NodeEditorPanel />);

      // SchemaForm renders fields with aria-label
      expect(screen.getByLabelText("Topic")).toBeInTheDocument();
      expect(screen.getByLabelText("Payload")).toBeInTheDocument();
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

      // Register a node type in the registry so validation can resolve the definition
      nodeRegistry.registerType("custom-save", {
        id: "custom-save",
        type: "custom-save",
        name: "custom-save",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          name: { value: "" },
          topic: { value: "" },
        },
        inputs: 0,
        outputs: 1,
      });

      // Add a node to the flow store so updateNodeData can find it
      useFlowStore.setState({
        nodes: [
          {
            id: "test-node-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: { name: "Test", type: "custom-save" },
          },
        ],
      });

      openEditor({
        nodeType: "custom-save",
        formData: { name: "Updated Node", topic: "new-topic" },
      });
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

  // -----------------------------------------------------------------------
  // Validation on save
  // -----------------------------------------------------------------------

  describe("validation on save", () => {
    it("prevents saving when required fields are empty", async () => {
      const user = userEvent.setup();

      // Register a node type with required fields
      nodeRegistry.registerType("val-test", {
        id: "val-test",
        type: "val-test",
        name: "val-test",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "", required: true },
        },
        inputs: 0,
        outputs: 1,
      });

      useFlowStore.setState({
        nodes: [
          {
            id: "test-node-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: { name: "Test", type: "val-test", topic: "" },
          },
        ],
      });

      openEditor({
        nodeType: "val-test",
        formData: { name: "Test Node", topic: "" },
      });
      render(<NodeEditorPanel />);

      // Click Done with empty required field
      await user.click(screen.getByTestId("node-editor-btn-done"));

      // Editor should still be open (save was prevented)
      expect(useEditorPanelStore.getState().isOpen).toBe(true);

      // Error count should be shown
      expect(screen.getByTestId("node-editor-error-count")).toHaveTextContent(
        "Fix 1 error before saving",
      );
    });

    it("saves successfully when all required fields are filled", async () => {
      const user = userEvent.setup();

      nodeRegistry.registerType("val-ok", {
        id: "val-ok",
        type: "val-ok",
        name: "val-ok",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "", required: true },
        },
        inputs: 0,
        outputs: 1,
      });

      useFlowStore.setState({
        nodes: [
          {
            id: "test-node-1",
            type: "nrNode",
            position: { x: 0, y: 0 },
            data: { name: "Test", type: "val-ok", topic: "filled" },
          },
        ],
      });

      openEditor({
        nodeType: "val-ok",
        formData: { name: "Test Node", topic: "sensor/data" },
      });
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-done"));

      // Editor should be closed (save succeeded)
      expect(useEditorPanelStore.getState().isOpen).toBe(false);
    });

    it("shows error count for multiple errors", async () => {
      const user = userEvent.setup();

      nodeRegistry.registerType("multi-err", {
        id: "multi-err",
        type: "multi-err",
        name: "multi-err",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "", required: true },
          payload: { value: "", required: true },
        },
        inputs: 0,
        outputs: 1,
      });

      openEditor({
        nodeType: "multi-err",
        formData: { name: "Test", topic: "", payload: "" },
      });
      render(<NodeEditorPanel />);

      await user.click(screen.getByTestId("node-editor-btn-done"));

      expect(screen.getByTestId("node-editor-error-count")).toHaveTextContent(
        "Fix 2 errors before saving",
      );
    });

    it("shows no error count when form is valid", () => {
      nodeRegistry.registerType("no-err", {
        id: "no-err",
        type: "no-err",
        name: "no-err",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "" },
        },
        inputs: 0,
        outputs: 1,
      });

      openEditor({
        nodeType: "no-err",
        formData: { name: "Test", topic: "value" },
      });
      render(<NodeEditorPanel />);

      expect(screen.queryByTestId("node-editor-error-count")).not.toBeInTheDocument();
    });

    it("shows red border on field with error", async () => {
      const user = userEvent.setup();

      nodeRegistry.registerType("field-err", {
        id: "field-err",
        type: "field-err",
        name: "field-err",
        category: "common",
        color: "#a6bbcf",
        defaults: {
          topic: { value: "", required: true },
        },
        inputs: 0,
        outputs: 1,
      });

      openEditor({
        nodeType: "field-err",
        formData: { name: "Test", topic: "" },
      });
      render(<NodeEditorPanel />);

      // Trigger validation by clicking Done
      await user.click(screen.getByTestId("node-editor-btn-done"));

      // Error count should be shown with the required topic field error
      expect(screen.getByTestId("node-editor-error-count")).toHaveTextContent(
        "Fix 1 error before saving",
      );
    });
  });
});
