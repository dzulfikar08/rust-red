/**
 * Status Store
 *
 * Zustand store for tracking the WebSocket connection status
 * displayed in the editor status bar.
 */
import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ConnectionStatus = "connected" | "disconnected" | "connecting";

interface StatusStore {
  connectionStatus: ConnectionStatus;
  setConnectionStatus: (status: ConnectionStatus) => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useStatusStore = create<StatusStore>((set) => ({
  connectionStatus: "disconnected",

  setConnectionStatus: (status) => {
    set({ connectionStatus: status });
  },
}));
