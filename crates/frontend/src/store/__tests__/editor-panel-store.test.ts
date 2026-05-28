import { describe, it, expect, beforeEach } from "vitest";
import { useEditorPanelStore } from "../editor-panel-store";

function resetStore() {
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

describe("useEditorPanelStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts closed", () => {
      expect(useEditorPanelStore.getState().isOpen).toBe(false);
    });

    it("starts in sidebar mode (not modal)", () => {
      expect(useEditorPanelStore.getState().isModal).toBe(false);
    });

    it("starts with no node selected", () => {
      expect(useEditorPanelStore.getState().nodeId).toBeNull();
      expect(useEditorPanelStore.getState().nodeType).toBeNull();
    });

    it("starts with empty form data", () => {
      expect(useEditorPanelStore.getState().formData).toEqual({});
    });

    it("starts clean (not dirty)", () => {
      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // openEditor
  // -----------------------------------------------------------------------

  describe("openEditor()", () => {
    it("opens the editor with node data", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "My Node", topic: "test" });

      const state = useEditorPanelStore.getState();
      expect(state.isOpen).toBe(true);
      expect(state.nodeId).toBe("node-1");
      expect(state.nodeType).toBe("inject");
    });

    it("copies node data to both originalData and formData", () => {
      const data = { name: "My Node", topic: "test" };
      useEditorPanelStore.getState().openEditor("node-1", "inject", data);

      const state = useEditorPanelStore.getState();
      expect(state.originalData).toEqual(data);
      expect(state.formData).toEqual(data);
    });

    it("starts in sidebar mode when opened", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      expect(useEditorPanelStore.getState().isModal).toBe(false);
    });

    it("starts clean when opened", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });

    it("resets modal state when opening a new node", () => {
      useEditorPanelStore.getState().toggleModal();
      expect(useEditorPanelStore.getState().isModal).toBe(true);

      useEditorPanelStore
        .getState()
        .openEditor("node-2", "debug", { name: "Debug" });
      expect(useEditorPanelStore.getState().isModal).toBe(false);
      expect(useEditorPanelStore.getState().nodeId).toBe("node-2");
    });
  });

  // -----------------------------------------------------------------------
  // closeEditor
  // -----------------------------------------------------------------------

  describe("closeEditor()", () => {
    it("closes the editor", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().closeEditor();

      expect(useEditorPanelStore.getState().isOpen).toBe(false);
    });

    it("clears nodeId and nodeType", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().closeEditor();

      expect(useEditorPanelStore.getState().nodeId).toBeNull();
      expect(useEditorPanelStore.getState().nodeType).toBeNull();
    });

    it("clears form data", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().closeEditor();

      expect(useEditorPanelStore.getState().formData).toEqual({});
      expect(useEditorPanelStore.getState().originalData).toEqual({});
    });

    it("resets dirty state", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().updateField("name", "Changed");
      useEditorPanelStore.getState().closeEditor();

      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // toggleModal
  // -----------------------------------------------------------------------

  describe("toggleModal()", () => {
    it("switches from sidebar to modal", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().toggleModal();

      expect(useEditorPanelStore.getState().isModal).toBe(true);
    });

    it("switches back from modal to sidebar", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().toggleModal();
      useEditorPanelStore.getState().toggleModal();

      expect(useEditorPanelStore.getState().isModal).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // updateField
  // -----------------------------------------------------------------------

  describe("updateField()", () => {
    it("updates a single field", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().updateField("name", "New");

      expect(useEditorPanelStore.getState().formData.name).toBe("New");
    });

    it("marks form as dirty when value changes", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().updateField("name", "New");

      expect(useEditorPanelStore.getState().isDirty).toBe(true);
    });

    it("does not mark dirty when value is same as original", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Same" });
      useEditorPanelStore.getState().updateField("name", "Same");

      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });

    it("adds new fields to formData", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Test" });
      useEditorPanelStore.getState().updateField("topic", "new-topic");

      expect(useEditorPanelStore.getState().formData.topic).toBe("new-topic");
    });
  });

  // -----------------------------------------------------------------------
  // setFormData
  // -----------------------------------------------------------------------

  describe("setFormData()", () => {
    it("replaces the entire form data", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old", topic: "old" });
      useEditorPanelStore.getState().setFormData({ name: "New" });

      expect(useEditorPanelStore.getState().formData).toEqual({ name: "New" });
      expect(useEditorPanelStore.getState().formData).not.toHaveProperty("topic");
    });

    it("marks dirty when new data differs from original", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().setFormData({ name: "New" });

      expect(useEditorPanelStore.getState().isDirty).toBe(true);
    });

    it("does not mark dirty when data matches original", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Same" });
      useEditorPanelStore.getState().setFormData({ name: "Same" });

      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // markClean
  // -----------------------------------------------------------------------

  describe("markClean()", () => {
    it("resets the dirty flag", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().updateField("name", "New");
      expect(useEditorPanelStore.getState().isDirty).toBe(true);

      useEditorPanelStore.getState().markClean();
      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });

    it("updates originalData to match current formData", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().updateField("name", "New");
      useEditorPanelStore.getState().markClean();

      expect(useEditorPanelStore.getState().originalData).toEqual({
        name: "New",
      });
    });

    it("subsequent edit to same value stays clean", () => {
      useEditorPanelStore
        .getState()
        .openEditor("node-1", "inject", { name: "Old" });
      useEditorPanelStore.getState().updateField("name", "New");
      useEditorPanelStore.getState().markClean();
      useEditorPanelStore.getState().updateField("name", "New");

      expect(useEditorPanelStore.getState().isDirty).toBe(false);
    });
  });
});
