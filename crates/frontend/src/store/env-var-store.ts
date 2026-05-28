/**
 * env-var-store -- Zustand store for environment variable management.
 *
 * Provides CRUD operations for environment variables used in Node-RED flows.
 * Supports ${VAR_NAME} resolution for substituting env var references in
 * node properties.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type EnvVarType = "str" | "num" | "bool";

export interface EnvVar {
  name: string;
  value: string;
  type: EnvVarType;
}

export interface EnvVarStore {
  /** Current list of environment variables. */
  vars: EnvVar[];
  /** Replace the entire variable list. */
  setVars: (vars: EnvVar[]) => void;
  /** Add a new variable. No-op if name already exists. */
  addVar: (name: string, value: string, type?: EnvVarType) => void;
  /** Remove a variable by name. */
  removeVar: (name: string) => void;
  /** Update fields on an existing variable. */
  updateVar: (name: string, updates: Partial<Omit<EnvVar, "name">>) => void;
  /** Look up a variable by name. */
  getVar: (name: string) => EnvVar | undefined;
  /** Resolve a string that may contain ${VAR} references. */
  resolveValue: (input: string) => string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Match all ${VAR_NAME} occurrences in a string. */
const ENV_REF_RE = /\$\{([^}]+)\}/g;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useEnvVarStore = create<EnvVarStore>((set, get) => ({
  vars: [],

  setVars: (vars) => {
    set({ vars });
  },

  addVar: (name, value, type = "str") => {
    const { vars } = get();
    if (vars.some((v) => v.name === name)) return;
    set({ vars: [...vars, { name, value, type }] });
  },

  removeVar: (name) => {
    set({ vars: get().vars.filter((v) => v.name !== name) });
  },

  updateVar: (name, updates) => {
    set({
      vars: get().vars.map((v) => (v.name === name ? { ...v, ...updates } : v)),
    });
  },

  getVar: (name) => {
    return get().vars.find((v) => v.name === name);
  },

  resolveValue: (input) => {
    const { vars } = get();
    return input.replace(ENV_REF_RE, (_match, varName: string) => {
      const found = vars.find((v) => v.name === varName);
      return found ? found.value : "";
    });
  },
}));
