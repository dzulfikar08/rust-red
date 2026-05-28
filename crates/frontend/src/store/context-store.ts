/**
 * context-store -- Zustand store for flow/global context data.
 *
 * Provides context key-value pairs fetched from the API endpoint
 * GET /context which returns { flow: {}, global: {} }.
 * Falls back to mock data when the API is unavailable.
 */

import { create } from "zustand";
import { client } from "../api/client";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ContextStore {
  /** Key-value pairs for the current flow context. */
  flowContext: Record<string, unknown> | null;
  /** Key-value pairs for the global context. */
  globalContext: Record<string, unknown> | null;
  /** True while a fetch is in progress. */
  isLoading: boolean;
  /** Error message if the last fetch failed. */
  error: string | null;

  /** Fetch context from the API (or use mock data). */
  refresh(): Promise<void>;
  /** Directly set context data (useful for tests / WebSocket push). */
  setContext(
    flow: Record<string, unknown>,
    global: Record<string, unknown>,
  ): void;
}

// ---------------------------------------------------------------------------
// Mock data (used when the API endpoint is not yet available)
// ---------------------------------------------------------------------------

const MOCK_FLOW: Record<string, unknown> = {
  counter: 42,
  settings: { debug: true, interval: 5000 },
  name: "My Flow",
};

const MOCK_GLOBAL: Record<string, unknown> = {
  shared: { version: "1.0" },
  startTime: 1700000000,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useContextStore = create<ContextStore>((set) => ({
  flowContext: null,
  globalContext: null,
  isLoading: false,
  error: null,

  refresh: async () => {
    set({ isLoading: true, error: null });
    try {
      const data = await client.get<{
        flow?: unknown;
        global?: unknown;
      }>("/context");

      const flow = isRecord(data.flow) ? data.flow : null;
      const global = isRecord(data.global) ? data.global : null;

      set({
        flowContext: flow,
        globalContext: global,
        isLoading: false,
      });
    } catch {
      // API not available -- fall back to mock data
      set({
        flowContext: MOCK_FLOW,
        globalContext: MOCK_GLOBAL,
        isLoading: false,
      });
    }
  },

  setContext: (flow, global) => {
    set({ flowContext: flow, globalContext: global, error: null });
  },
}));
