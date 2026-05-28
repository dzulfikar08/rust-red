/**
 * useKeyboard — React hook that manages keyboard shortcuts.
 *
 * Attaches a global keydown listener on mount and dispatches matching
 * actions via the action store. Removes the listener on unmount.
 *
 * Replaces Node-RED's RED.keyboard.
 */

import { useCallback, useEffect, useRef } from "react";
import { useKeyboardStore } from "../store/keyboard-store";
import { useActionStore } from "../store/action-store";
import type { KeyBinding } from "../store/keyboard-store";

export type { KeyBinding };

export interface UseKeyboardReturn {
  bind: (binding: KeyBinding) => () => void;
  unbind: (binding: KeyBinding) => void;
  setScope: (scope: string) => void;
  getScope: () => string;
}

// ---------------------------------------------------------------------------
// Default Node-RED keybindings
// ---------------------------------------------------------------------------

export const DEFAULT_KEYBINDINGS: KeyBinding[] = [
  { key: "z", modifiers: { ctrl: true }, action: "core:undo", preventDefault: true },
  { key: "z", modifiers: { ctrl: true, shift: true }, action: "core:redo", preventDefault: true },
  { key: "y", modifiers: { ctrl: true }, action: "core:redo", preventDefault: true },
  { key: "Delete", action: "core:delete-selected", preventDefault: true },
  { key: "Backspace", action: "core:delete-selected", preventDefault: true },
  { key: "a", modifiers: { ctrl: true }, action: "core:select-all", preventDefault: true },
  { key: "c", modifiers: { ctrl: true }, action: "core:copy", preventDefault: true },
  { key: "v", modifiers: { ctrl: true }, action: "core:paste", preventDefault: true },
  { key: "x", modifiers: { ctrl: true }, action: "core:cut", preventDefault: true },
  { key: "Escape", action: "core:deselect" },
  { key: "s", modifiers: { ctrl: true }, action: "core:deploy", preventDefault: true },
  { key: "?", action: "core:show-help" },
  { key: "p", modifiers: { ctrl: true, shift: true }, action: "core:action-list", preventDefault: true },
  { key: " ", modifiers: { ctrl: true, shift: true }, action: "core:action-list", preventDefault: true },
];

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useKeyboard(): UseKeyboardReturn {
  const unbindFns = useRef<(() => void)[]>([]);
  const keyboardStore = useKeyboardStore;
  const actionStore = useActionStore;

  // Global keydown handler
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const match = keyboardStore
        .getState()
        .resolve(e.key, e.ctrlKey || e.metaKey, e.shiftKey, e.altKey, e.metaKey);

      if (match) {
        if (match.preventDefault) {
          e.preventDefault();
        }
        actionStore.getState().invokeAction(match.action);
      }
    };

    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, [keyboardStore, actionStore]);

  // Cleanup bound shortcuts on unmount
  useEffect(() => {
    const fns = unbindFns.current;
    return () => {
      fns.forEach((fn) => fn());
      fns.length = 0;
    };
  }, []);

  const bind = useCallback(
    (binding: KeyBinding): (() => void) => {
      keyboardStore.getState().addBinding(binding);
      const unbindFn = () => {
        keyboardStore.getState().removeBinding(binding);
      };
      unbindFns.current.push(unbindFn);
      return unbindFn;
    },
    [keyboardStore],
  );

  const unbind = useCallback(
    (binding: KeyBinding) => {
      keyboardStore.getState().removeBinding(binding);
    },
    [keyboardStore],
  );

  const setScope = useCallback(
    (scope: string) => {
      keyboardStore.getState().setScope(scope);
    },
    [keyboardStore],
  );

  const getScope = useCallback(() => keyboardStore.getState().getScope(), [keyboardStore]);

  return { bind, unbind, setScope, getScope };
}

export default useKeyboard;
