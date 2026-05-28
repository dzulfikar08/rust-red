import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useKeyboard } from "../useKeyboard";
import { useKeyboardStore } from "../../store/keyboard-store";
import { useActionStore } from "../../store/action-store";
import { DEFAULT_KEYBINDINGS } from "../useKeyboard";
import type { KeyBinding } from "../../store/keyboard-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStores() {
  useKeyboardStore.setState({ bindings: new Map(), scope: "default" });
  useActionStore.setState({ actions: new Map() });
}

/** Fire a keyboard event on the jsdom window. */
function fireKey(
  key: string,
  opts: Partial<Pick<KeyboardEventInit, "ctrlKey" | "shiftKey" | "altKey" | "metaKey">> = {},
) {
  const event = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...opts,
  });
  // jsdom doesn't support preventDefault well, so we spy on it
  vi.spyOn(event, "preventDefault");
  window.dispatchEvent(event);
  return event;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useKeyboard", () => {
  beforeEach(() => {
    resetStores();
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // bind / unbind
  // -------------------------------------------------------------------------

  describe("bind()", () => {
    it("adds a binding and returns an unbind function", () => {
      const { result } = renderHook(() => useKeyboard());

      const binding: KeyBinding = {
        key: "z",
        modifiers: { ctrl: true },
        action: "core:undo",
      };

      const unbind = result.current.bind(binding);
      expect(typeof unbind).toBe("function");

      // Clean up
      unbind();
    });

    it("invokes the action when the key is pressed", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      // Register a dummy action
      const handler = vi.fn();
      useActionStore.getState().registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
      });

      // Bind Ctrl-Z to core:undo
      act(() => {
        kb.current.bind({
          key: "z",
          modifiers: { ctrl: true },
          action: "core:undo",
          preventDefault: true,
        });
      });

      fireKey("z", { ctrlKey: true });

      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  describe("unbind()", () => {
    it("removes a binding so the key no longer triggers the action", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      const handler = vi.fn();
      useActionStore.getState().registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
      });

      const binding: KeyBinding = {
        key: "z",
        modifiers: { ctrl: true },
        action: "core:undo",
      };

      act(() => {
        kb.current.bind(binding);
      });

      fireKey("z", { ctrlKey: true });
      expect(handler).toHaveBeenCalledTimes(1);

      act(() => {
        kb.current.unbind(binding);
      });

      fireKey("z", { ctrlKey: true });
      expect(handler).toHaveBeenCalledTimes(1); // Not called again
    });
  });

  // -------------------------------------------------------------------------
  // Scope
  // -------------------------------------------------------------------------

  describe("setScope() / getScope()", () => {
    it("defaults to 'default' scope", () => {
      const { result } = renderHook(() => useKeyboard());
      expect(result.current.getScope()).toBe("default");
    });

    it("switches scope", () => {
      const { result } = renderHook(() => useKeyboard());

      act(() => {
        result.current.setScope("node-edit");
      });

      expect(result.current.getScope()).toBe("node-edit");
    });

    it("scope-specific binding triggers only in matching scope", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      const globalHandler = vi.fn();
      const scopedHandler = vi.fn();

      useActionStore.getState().registerAction({
        id: "global:escape",
        name: "Escape",
        scope: "core",
        handler: globalHandler,
      });
      useActionStore.getState().registerAction({
        id: "scoped:escape",
        name: "Escape (scoped)",
        scope: "node",
        handler: scopedHandler,
      });

      // Global binding (no scope)
      act(() => {
        kb.current.bind({
          key: "Escape",
          action: "global:escape",
        });
      });

      // Scoped binding (scope = "node")
      act(() => {
        kb.current.bind({
          key: "Escape",
          action: "scoped:escape",
          scope: "node",
        });
      });

      // In "node" scope, the scoped binding should take priority
      act(() => {
        kb.current.setScope("node");
      });

      fireKey("Escape");
      expect(scopedHandler).toHaveBeenCalledTimes(1);
      expect(globalHandler).not.toHaveBeenCalled();

      // Reset
      scopedHandler.mockClear();
      globalHandler.mockClear();

      // In "default" scope, only the global binding should fire
      act(() => {
        kb.current.setScope("default");
      });

      fireKey("Escape");
      expect(globalHandler).toHaveBeenCalledTimes(1);
      expect(scopedHandler).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // Modifier detection
  // -------------------------------------------------------------------------

  describe("modifier detection", () => {
    it("distinguishes Ctrl-Z from Ctrl-Shift-Z", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      const undoHandler = vi.fn();
      const redoHandler = vi.fn();

      useActionStore.getState().registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler: undoHandler,
      });
      useActionStore.getState().registerAction({
        id: "core:redo",
        name: "Redo",
        scope: "core",
        handler: redoHandler,
      });

      act(() => {
        kb.current.bind({
          key: "z",
          modifiers: { ctrl: true },
          action: "core:undo",
        });
        kb.current.bind({
          key: "z",
          modifiers: { ctrl: true, shift: true },
          action: "core:redo",
        });
      });

      fireKey("z", { ctrlKey: true });
      expect(undoHandler).toHaveBeenCalledTimes(1);
      expect(redoHandler).not.toHaveBeenCalled();

      fireKey("z", { ctrlKey: true, shiftKey: true });
      expect(redoHandler).toHaveBeenCalledTimes(1);
    });

    it("does not match when wrong modifiers are held", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      const handler = vi.fn();
      useActionStore.getState().registerAction({
        id: "core:copy",
        name: "Copy",
        scope: "core",
        handler,
      });

      act(() => {
        kb.current.bind({
          key: "c",
          modifiers: { ctrl: true },
          action: "core:copy",
        });
      });

      // Just 'c' without Ctrl should not trigger
      fireKey("c");
      expect(handler).not.toHaveBeenCalled();

      // With Ctrl should trigger
      fireKey("c", { ctrlKey: true });
      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  // -------------------------------------------------------------------------
  // preventDefault
  // -------------------------------------------------------------------------

  describe("preventDefault", () => {
    it("calls preventDefault when binding has preventDefault: true", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      useActionStore.getState().registerAction({
        id: "core:save",
        name: "Save",
        scope: "core",
        handler: vi.fn(),
      });

      act(() => {
        kb.current.bind({
          key: "s",
          modifiers: { ctrl: true },
          action: "core:save",
          preventDefault: true,
        });
      });

      const event = fireKey("s", { ctrlKey: true });
      expect(event.preventDefault).toHaveBeenCalled();
    });

    it("does not call preventDefault by default", () => {
      const { result: kb } = renderHook(() => useKeyboard());

      useActionStore.getState().registerAction({
        id: "core:help",
        name: "Help",
        scope: "core",
        handler: vi.fn(),
      });

      act(() => {
        kb.current.bind({
          key: "?",
          action: "core:help",
        });
      });

      const event = fireKey("?");
      expect(event.preventDefault).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // Default keybindings
  // -------------------------------------------------------------------------

  describe("DEFAULT_KEYBINDINGS", () => {
    it("contains the expected bindings", () => {
      expect(DEFAULT_KEYBINDINGS.length).toBeGreaterThan(0);

      const actions = DEFAULT_KEYBINDINGS.map((b) => b.action);
      expect(actions).toContain("core:undo");
      expect(actions).toContain("core:redo");
      expect(actions).toContain("core:delete-selected");
      expect(actions).toContain("core:select-all");
      expect(actions).toContain("core:copy");
      expect(actions).toContain("core:paste");
      expect(actions).toContain("core:cut");
      expect(actions).toContain("core:deselect");
      expect(actions).toContain("core:deploy");
      expect(actions).toContain("core:show-help");
    });

    it("Ctrl-Z maps to core:undo", () => {
      const undo = DEFAULT_KEYBINDINGS.find(
        (b) => b.key === "z" && b.modifiers?.ctrl && !b.modifiers?.shift,
      );
      expect(undo).toBeDefined();
      expect(undo!.action).toBe("core:undo");
    });

    it("Ctrl-Y and Ctrl-Shift-Z both map to core:redo", () => {
      const ctrlY = DEFAULT_KEYBINDINGS.find(
        (b) => b.key === "y" && b.modifiers?.ctrl,
      );
      const ctrlShiftZ = DEFAULT_KEYBINDINGS.find(
        (b) => b.key === "z" && b.modifiers?.ctrl && b.modifiers?.shift,
      );
      expect(ctrlY).toBeDefined();
      expect(ctrlY!.action).toBe("core:redo");
      expect(ctrlShiftZ).toBeDefined();
      expect(ctrlShiftZ!.action).toBe("core:redo");
    });
  });

  // -------------------------------------------------------------------------
  // Cleanup on unmount
  // -------------------------------------------------------------------------

  describe("cleanup on unmount", () => {
    it("unbinds all shortcuts registered through this hook on unmount", () => {
      const handler = vi.fn();
      useActionStore.getState().registerAction({
        id: "core:undo",
        name: "Undo",
        scope: "core",
        handler,
      });

      const { result: kb, unmount } = renderHook(() => useKeyboard());

      act(() => {
        kb.current.bind({
          key: "z",
          modifiers: { ctrl: true },
          action: "core:undo",
        });
      });

      // Before unmount: binding works
      fireKey("z", { ctrlKey: true });
      expect(handler).toHaveBeenCalledTimes(1);

      unmount();

      handler.mockClear();

      // After unmount: binding should be gone
      fireKey("z", { ctrlKey: true });
      expect(handler).not.toHaveBeenCalled();
    });

    it("removes the global keydown listener on unmount", () => {
      const addSpy = vi.spyOn(window, "addEventListener");
      const removeSpy = vi.spyOn(window, "removeEventListener");

      const { unmount } = renderHook(() => useKeyboard());

      expect(addSpy).toHaveBeenCalledWith("keydown", expect.any(Function));

      unmount();

      expect(removeSpy).toHaveBeenCalledWith("keydown", expect.any(Function));

      addSpy.mockRestore();
      removeSpy.mockRestore();
    });
  });
});
