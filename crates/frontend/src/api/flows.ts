import { client } from "./client";
import type {
  FlowsResponse,
  FlowsPayload,
  FlowResponse,
} from "./types";

export const flowsApi = {
  getFlows(): Promise<FlowsResponse> {
    return client.get<FlowsResponse>("/flows");
  },

  postFlows(payload: Omit<FlowsPayload, "rev"> & { rev: string }): Promise<FlowsResponse> {
    return client.post<FlowsResponse>("/flows", payload);
  },

  getState(): Promise<{ state: string }> {
    return client.get<{ state: string }>("/flows/state");
  },

  setState(state: string): Promise<{ state: string }> {
    return client.post<{ state: string }>("/flows/state", { state });
  },

  getFlow(id: string): Promise<FlowResponse> {
    return client.get<FlowResponse>(`/flow/${id}`);
  },

  createFlow(label: string): Promise<FlowResponse> {
    return client.post<FlowResponse>("/flow", { label });
  },

  updateFlow(id: string, data: Partial<FlowResponse>): Promise<FlowResponse> {
    return client.put<FlowResponse>(`/flow/${id}`, data);
  },

  deleteFlow(id: string): Promise<void> {
    return client.delete<void>(`/flow/${id}`);
  },
};

// ---------------------------------------------------------------------------
// Convenience helpers for deploy flow
// ---------------------------------------------------------------------------

/**
 * Deploy all flows by posting to the /flows endpoint.
 * Accepts a simplified payload with flow definitions and nodes.
 */
export async function deployFlows(
  flows: FlowsPayload,
): Promise<FlowsResponse> {
  return flowsApi.postFlows(flows);
}

/**
 * Fetch the current flow state from the server.
 */
export async function getFlows(): Promise<FlowsResponse> {
  return flowsApi.getFlows();
}
