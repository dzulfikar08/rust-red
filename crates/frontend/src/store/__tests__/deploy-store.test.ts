import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { useDeployStore } from "../deploy-store";
import { useFlowStore } from "../flow-store";
import { useWorkspaceStore } from "../workspace-store";
import { useNotificationStore } from "../notification-store";
import { eventBus } from "../../red/core/events";

// ---------------------------------------------------------------------------
// Mock the flows API module
// ---------------------------------------------------------------------------

vi.mock("../../api/flows", () => ({
  flowsApi: {
    postFlows: vi.fn(),
    getFlows: vi.fn(),
  },
}));

// Import the mocked module so we can control it in tests
import { flowsApi } from "../../api/flows";

const mockedPostFlows = vi.mocked(flowsApi.postFlows);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetDeployStore() {
  useDeployStore.setState({
    status: "idle",
    lastDeployTime: null,
    hasUnsavedChanges: false,
    errorMessage: null,
  });
}

function resetAllStores() {
  resetDeployStore();
  useFlowStore.setState({ nodes: [], edges: [], revision: "test-rev-1" });
  useWorkspaceStore.setState({ flows: [], activeFlowId: null });
  useNotificationStore.setState({ notifications: [] });
  eventBus.clear();
}

describe("useDeployStore", () => {
  beforeEach(() => {
    resetAllStores();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with idle status", () => {
      expect(useDeployStore.getState().status).toBe("idle");
    });

    it("starts with no last deploy time", () => {
      expect(useDeployStore.getState().lastDeployTime).toBeNull();
    });

    it("starts with no unsaved changes", () => {
      expect(useDeployStore.getState().hasUnsavedChanges).toBe(false);
    });

    it("starts with no error message", () => {
      expect(useDeployStore.getState().errorMessage).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // setHasUnsavedChanges
  // -----------------------------------------------------------------------

  describe("setHasUnsavedChanges()", () => {
    it("sets unsaved changes to true", () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      expect(useDeployStore.getState().hasUnsavedChanges).toBe(true);
    });

    it("sets unsaved changes to false", () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      useDeployStore.getState().setHasUnsavedChanges(false);
      expect(useDeployStore.getState().hasUnsavedChanges).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // getStatus
  // -----------------------------------------------------------------------

  describe("getStatus()", () => {
    it("returns current status", () => {
      expect(useDeployStore.getState().getStatus()).toBe("idle");
    });

    it("reflects status changes", () => {
      useDeployStore.setState({ status: "deploying" });
      expect(useDeployStore.getState().getStatus()).toBe("deploying");
    });
  });

  // -----------------------------------------------------------------------
  // deploy - success path
  // -----------------------------------------------------------------------

  describe("deploy() - success", () => {
    it("transitions status from idle -> deploying -> success", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      const promise = useDeployStore.getState().deploy();
      expect(useDeployStore.getState().status).toBe("deploying");

      await promise;
      expect(useDeployStore.getState().status).toBe("success");
    });

    it("updates lastDeployTime on success", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      const before = Date.now();
      await useDeployStore.getState().deploy();
      const after = Date.now();

      const deployTime = useDeployStore.getState().lastDeployTime!;
      expect(deployTime).toBeGreaterThanOrEqual(before);
      expect(deployTime).toBeLessThanOrEqual(after);
    });

    it("clears hasUnsavedChanges on success", async () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().hasUnsavedChanges).toBe(false);
    });

    it("clears errorMessage on success", async () => {
      useDeployStore.setState({ errorMessage: "previous error" });
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().errorMessage).toBeNull();
    });

    it("posts flow data to the API", async () => {
      useFlowStore.setState({
        nodes: [
          {
            id: "node-1",
            type: "inject",
            position: { x: 100, y: 200 },
            data: { label: "Inject", type: "inject" },
          },
        ],
        edges: [],
        revision: "rev-123",
      });
      useWorkspaceStore.getState().addFlow("Test Flow");

      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      await useDeployStore.getState().deploy();

      expect(mockedPostFlows).toHaveBeenCalledOnce();
      const payload = mockedPostFlows.mock.calls[0][0];
      expect(payload.rev).toBe("rev-123");
      expect(payload.flows).toBeDefined();
      expect(payload.flows.length).toBeGreaterThan(0);
    });

    it("emits deploy:start and deploy:success events", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      const startHandler = vi.fn();
      const successHandler = vi.fn();
      eventBus.on("deploy:start", startHandler);
      eventBus.on("deploy:success", successHandler);

      await useDeployStore.getState().deploy();

      expect(startHandler).toHaveBeenCalledOnce();
      expect(startHandler).toHaveBeenCalledWith({ mode: "full" });
      expect(successHandler).toHaveBeenCalledOnce();
      expect(successHandler).toHaveBeenCalledWith({ mode: "full" });
    });

    it("shows success notification", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      await useDeployStore.getState().deploy();

      const notifications = useNotificationStore.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe("success");
      expect(notifications[0].title).toBe("Deploy successful");
    });
  });

  // -----------------------------------------------------------------------
  // deploy - error path
  // -----------------------------------------------------------------------

  describe("deploy() - error", () => {
    it("transitions status to error on API failure", async () => {
      mockedPostFlows.mockRejectedValueOnce(new Error("Network error"));

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().status).toBe("error");
    });

    it("stores error message on failure", async () => {
      mockedPostFlows.mockRejectedValueOnce(new Error("Server error 500"));

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().errorMessage).toBe("Server error 500");
    });

    it("handles non-Error thrown values", async () => {
      mockedPostFlows.mockRejectedValueOnce("string error");

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().errorMessage).toBe("Deploy failed");
    });

    it("emits deploy:start and deploy:error events", async () => {
      mockedPostFlows.mockRejectedValueOnce(new Error("fail"));

      const startHandler = vi.fn();
      const errorHandler = vi.fn();
      eventBus.on("deploy:start", startHandler);
      eventBus.on("deploy:error", errorHandler);

      await useDeployStore.getState().deploy();

      expect(startHandler).toHaveBeenCalledOnce();
      expect(errorHandler).toHaveBeenCalledOnce();
      expect(errorHandler).toHaveBeenCalledWith({
        mode: "full",
        error: "fail",
      });
    });

    it("shows error notification on failure", async () => {
      mockedPostFlows.mockRejectedValueOnce(new Error("Deploy failed"));

      await useDeployStore.getState().deploy();

      const notifications = useNotificationStore.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe("error");
      expect(notifications[0].title).toBe("Deploy failed");
    });

    it("does not clear unsaved changes on error", async () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      mockedPostFlows.mockRejectedValueOnce(new Error("fail"));

      await useDeployStore.getState().deploy();
      expect(useDeployStore.getState().hasUnsavedChanges).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // deploy - double deploy prevention
  // -----------------------------------------------------------------------

  describe("deploy() - double deploy prevention", () => {
    it("prevents concurrent deploys", async () => {
      let resolveFirst: (value: unknown) => void;
      const firstCall = new Promise((resolve) => {
        resolveFirst = resolve;
      });

      mockedPostFlows.mockImplementationOnce(() => firstCall as Promise<any>);

      // Start first deploy
      useDeployStore.getState().deploy();
      expect(useDeployStore.getState().status).toBe("deploying");

      // Attempt second deploy while first is running
      await useDeployStore.getState().deploy();
      expect(mockedPostFlows).toHaveBeenCalledOnce();

      // Resolve the first deploy
      resolveFirst!({ rev: "new-rev", nodes: [], edges: [] });
    });
  });

  // -----------------------------------------------------------------------
  // deploy - mode parameter
  // -----------------------------------------------------------------------

  describe("deploy() - mode parameter", () => {
    it("defaults to 'full' mode", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      const handler = vi.fn();
      eventBus.on("deploy:start", handler);

      await useDeployStore.getState().deploy();

      expect(handler).toHaveBeenCalledWith({ mode: "full" });
    });

    it("passes the specified mode to events", async () => {
      mockedPostFlows.mockResolvedValueOnce({ rev: "new-rev", nodes: [], edges: [] });

      const handler = vi.fn();
      eventBus.on("deploy:start", handler);

      await useDeployStore.getState().deploy("nodes");

      expect(handler).toHaveBeenCalledWith({ mode: "nodes" });
    });
  });
});
