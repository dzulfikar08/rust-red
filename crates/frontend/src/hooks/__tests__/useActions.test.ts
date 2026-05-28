import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useActions } from "../useActions";
import { useActionStore } from "../../store/action-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useActionStore.setState({ actions: new Map() });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useActions", () => {
  beforeEach(() => {
    resetStore();
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // registerAction
  // -------------------------------------------------------------------------

  describe("registerAction()", () => {
    it("registers an action and returns an unregister function", () => {
      const { result } = renderHook(() => useActions());

      const handler = vi.fn();
      const unregister = result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
      });

      expect(typeof unregister).toBe("function");

      const action = result.current.getAction("core:undo");
      expect(action).toBeDefined();
      expect(action!.name).toBe("Undo");
    });

    it("allows registering multiple actions", () => {
      const { result } = renderHook(() => useActions());

      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });
      result.current.registerAction({
        id: "core:redo",
        name: "Redo",
        scope: "core",
        handler: vi.fn(),
      });

      expect(result.current.getAllActions()).toHaveLength(2);
    });

    it("overwrites an action with the same id", () => {
      const { result } = renderHook(() => useActions());

      const handler1 = vi.fn();
      const handler2 = vi.fn();

      result.current.registerAction({
        id: "core:undo",
        name: "Undo v1",
        scope: "core",
        handler: handler1,
      });
      result.current.registerAction({
        id: "core:undo",
        name: "Undo v2",
        scope: "core",
        handler: handler2,
      });

      expect(result.current.getAllActions()).toHaveLength(1);
      expect(result.current.getAction("core:undo")!.name).toBe("Undo v2");
    });
  });

  // -------------------------------------------------------------------------
  // unregisterAction
  // -------------------------------------------------------------------------

  describe("unregister via returned function", () => {
    it("removes the action when unregister is called", () => {
      const { result } = renderHook(() => useActions());

      const unregister = result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });

      expect(result.current.getAction("core:undo")).toBeDefined();

      act(() => {
        unregister();
      });

      expect(result.current.getAction("core:undo")).toBeUndefined();
    });
  });

  // -------------------------------------------------------------------------
  // invokeAction
  // -------------------------------------------------------------------------

  describe("invokeAction()", () => {
    it("invokes the handler of a registered action", () => {
      const { result } = renderHook(() => useActions());

      const handler = vi.fn();
      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
      });

      act(() => {
        result.current.invokeAction("core:undo");
      });

      expect(handler).toHaveBeenCalledTimes(1);
    });

    it("does nothing for an unknown action id", () => {
      const { result } = renderHook(() => useActions());

      // Should not throw
      act(() => {
        result.current.invokeAction("nonexistent:action");
      });
    });

    it("does not call handler when enabled() returns false", () => {
      const { result } = renderHook(() => useActions());

      const handler = vi.fn();
      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
        enabled: () => false,
      });

      act(() => {
        result.current.invokeAction("core:undo");
      });

      expect(handler).not.toHaveBeenCalled();
    });

    it("calls handler when enabled() returns true", () => {
      const { result } = renderHook(() => useActions());

      const handler = vi.fn();
      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
        enabled: () => true,
      });

      act(() => {
        result.current.invokeAction("core:undo");
      });

      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  // -------------------------------------------------------------------------
  // getAction
  // -------------------------------------------------------------------------

  describe("getAction()", () => {
    it("returns the action by id", () => {
      const { result } = renderHook(() => useActions());

      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });

      const action = result.current.getAction("core:undo");
      expect(action).toBeDefined();
      expect(action!.id).toBe("core:undo");
    });

    it("returns undefined for unknown id", () => {
      const { result } = renderHook(() => useActions());
      expect(result.current.getAction("nonexistent")).toBeUndefined();
    });
  });

  // -------------------------------------------------------------------------
  // getAllActions
  // -------------------------------------------------------------------------

  describe("getAllActions()", () => {
    it("returns all registered actions", () => {
      const { result } = renderHook(() => useActions());

      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });
      result.current.registerAction({
        id: "node:edit",
        name: "Edit Node",
        scope: "node",
        handler: vi.fn(),
      });

      const all = result.current.getAllActions();
      expect(all).toHaveLength(2);
    });

    it("returns empty array when no actions registered", () => {
      const { result } = renderHook(() => useActions());
      expect(result.current.getAllActions()).toEqual([]);
    });
  });

  // -------------------------------------------------------------------------
  // getActionsByScope
  // -------------------------------------------------------------------------

  describe("getActionsByScope()", () => {
    it("returns actions matching a given scope", () => {
      const { result } = renderHook(() => useActions());

      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });
      result.current.registerAction({
        id: "core:redo",
        name: "Redo",
        scope: "core",
        handler: vi.fn(),
      });
      result.current.registerAction({
        id: "node:edit",
        name: "Edit Node",
        scope: "node",
        handler: vi.fn(),
      });

      const coreActions = result.current.getActionsByScope("core");
      expect(coreActions).toHaveLength(2);
      expect(coreActions.every((a) => a.scope === "core")).toBe(true);
    });

    it("returns empty array for unknown scope", () => {
      const { result } = renderHook(() => useActions());
      expect(result.current.getActionsByScope("nonexistent")).toEqual([]);
    });
  });

  // -------------------------------------------------------------------------
  // Cleanup on unmount
  // -------------------------------------------------------------------------

  describe("cleanup on unmount", () => {
    it("unregisters actions when the hook unmounts", () => {
      const { result, unmount } = renderHook(() => useActions());

      result.current.registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: vi.fn(),
      });

      expect(result.current.getAction("core:undo")).toBeDefined();

      unmount();

      // After unmount, the action should be cleaned up
      expect(useActionStore.getState().getAction("core:undo")).toBeUndefined();
    });
  });
});
