/**
 * Deploy Store
 *
 * Zustand store for managing the deploy lifecycle.
 * Collects flow state from workspace-store and flow-store,
 * posts to the /flows API, and tracks deploy status.
 */
import { create } from "zustand";
import { flowsApi } from "../api/flows";
import { eventBus } from "../red/core/events";
import { useNotificationStore } from "./notification-store";
import { useFlowStore } from "./flow-store";
import { useWorkspaceStore } from "./workspace-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type DeployStatus = "idle" | "deploying" | "success" | "error";
export type DeployMode = "full" | "flows" | "nodes";

interface DeployStore {
  status: DeployStatus;
  lastDeployTime: number | null;
  hasUnsavedChanges: boolean;
  errorMessage: string | null;

  setHasUnsavedChanges: (has: boolean) => void;
  deploy: (mode?: DeployMode) => Promise<void>;
  getStatus: () => DeployStatus;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useDeployStore = create<DeployStore>((set, get) => ({
  status: "idle",
  lastDeployTime: null,
  hasUnsavedChanges: false,
  errorMessage: null,

  setHasUnsavedChanges: (has: boolean) => {
    set({ hasUnsavedChanges: has });
  },

  deploy: async (mode: DeployMode = "full") => {
    const currentState = get();

    // Prevent double-deploy
    if (currentState.status === "deploying") {
      return;
    }

    set({ status: "deploying", errorMessage: null });
    eventBus.emit("deploy:start", { mode });

    try {
      // Collect current flow state from stores
      const { nodes, edges, revision } = useFlowStore.getState();
      const { flows } = useWorkspaceStore.getState();

      // Build the deploy payload in Node-RED's expected format
      const payload = {
        rev: revision,
        flows: [
          // Flow tab definitions
          ...flows.map((f) => ({
            id: f.id,
            label: f.label,
            type: "tab" as const,
            disabled: f.disabled,
            info: f.info,
          })),
          // Nodes (React Flow nodes -> Node-RED node format)
          ...nodes.map((n) => ({
            id: n.id,
            type: n.data?.type ?? n.type,
            ...(n.data ?? {}),
            // Include position
            ...(n.position ? { x: n.position.x, y: n.position.y } : {}),
          })),
        ],
        credentials: {},
      };

      await flowsApi.postFlows(payload);

      set({
        status: "success",
        lastDeployTime: Date.now(),
        hasUnsavedChanges: false,
        errorMessage: null,
      });

      eventBus.emit("deploy:success", { mode });
      useNotificationStore.getState().success("Deploy successful");
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "Deploy failed";

      set({
        status: "error",
        errorMessage: message,
      });

      eventBus.emit("deploy:error", { mode, error: message });
      useNotificationStore.getState().error("Deploy failed", message);
    }
  },

  getStatus: () => get().status,
}));

export default useDeployStore;
