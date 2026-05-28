/**
 * Keyboard store — Zustand store backing the keyboard shortcut system.
 *
 * Manages key bindings and the current scope. The useKeyboard hook
 * attaches the global keydown listener that dispatches against this store.
 *
 * Replaces Node-RED's RED.keyboard.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface KeyBinding {
  key: string; // e.g. 'z', 'Delete', 'Escape', 'a'
  modifiers?: {
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
    meta?: boolean;
  };
  action: string; // Action ID to invoke
  scope?: string; // Only active when scope matches (undefined = all scopes)
  preventDefault?: boolean;
}

/** Normalised key for the bindings map. */
function bindingKey(b: KeyBinding): string {
  const mods: string[] = [];
  if (b.modifiers?.ctrl) mods.push("ctrl");
  if (b.modifiers?.shift) mods.push("shift");
  if (b.modifiers?.alt) mods.push("alt");
  if (b.modifiers?.meta) mods.push("meta");
  return `${mods.join("+")}:${b.key.toLowerCase()}`;
}

interface KeyboardState {
  bindings: Map<string, KeyBinding[]>;
  scope: string;

  addBinding: (binding: KeyBinding) => void;
  removeBinding: (binding: KeyBinding) => void;
  setScope: (scope: string) => void;
  getScope: () => string;
  /** Find a matching binding for a keyboard event. */
  resolve: (
    key: string,
    ctrl: boolean,
    shift: boolean,
    alt: boolean,
    meta: boolean,
  ) => KeyBinding | undefined;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useKeyboardStore = create<KeyboardState>((set, get) => ({
  bindings: new Map(),
  scope: "default",

  addBinding: (binding) => {
    const k = bindingKey(binding);
    set((state) => {
      const next = new Map(state.bindings);
      const list = next.get(k) ?? [];
      next.set(k, [...list, binding]);
      return { bindings: next };
    });
  },

  removeBinding: (binding) => {
    const k = bindingKey(binding);
    set((state) => {
      const next = new Map(state.bindings);
      const list = next.get(k);
      if (!list) return state;
      const filtered = list.filter(
        (b) =>
          !(
            b.action === binding.action &&
            b.scope === binding.scope &&
            b.key === binding.key
          ),
      );
      if (filtered.length === 0) {
        next.delete(k);
      } else {
        next.set(k, filtered);
      }
      return { bindings: next };
    });
  },

  setScope: (scope) => set({ scope }),

  getScope: () => get().scope,

  resolve: (key, ctrl, shift, alt, meta) => {
    const state = get();

    // Build normalised key with current modifiers
    const mods: string[] = [];
    if (ctrl) mods.push("ctrl");
    if (shift) mods.push("shift");
    if (alt) mods.push("alt");
    if (meta) mods.push("meta");
    const k = `${mods.join("+")}:${key.toLowerCase()}`;

    const candidates = state.bindings.get(k);
    if (!candidates || candidates.length === 0) return undefined;

    const currentScope = state.scope;

    // Prefer scope-specific bindings, fall back to global (scope=undefined)
    for (const b of candidates) {
      if (b.scope === currentScope) return b;
    }
    for (const b of candidates) {
      if (b.scope === undefined) return b;
    }
    return undefined;
  },
}));

export default useKeyboardStore;
