import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DebugMessage {
  id: string;
  timestamp: number;
  topic?: string;
  msg: Record<string, unknown>;
  sourceNode?: { id: string; name: string; type: string };
  sourceFlow?: string;
  level?: "debug" | "warn" | "error" | "info";
}

interface DebugStore {
  /** Debug messages in chronological order (oldest first). */
  messages: DebugMessage[];
  /** Text filter applied to message content. */
  filter: string;
  /** Flow filter -- null means "All flows". */
  flowFilter: string | null;
  /** Maximum number of messages to keep. */
  maxMessages: number;

  addMessage(msg: DebugMessage): void;
  clearMessages(): void;
  setFilter(filter: string): void;
  setFlowFilter(flowId: string | null): void;
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_MAX_MESSAGES = 1000;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useDebugStore = create<DebugStore>((set, get) => ({
  messages: [],
  filter: "",
  flowFilter: null,
  maxMessages: DEFAULT_MAX_MESSAGES,

  addMessage: (msg) => {
    set((state) => {
      const next = [...state.messages, msg];
      // Trim to maxMessages, keeping the most recent ones
      if (next.length > state.maxMessages) {
        return { messages: next.slice(next.length - state.maxMessages) };
      }
      return { messages: next };
    });
  },

  clearMessages: () => {
    set({ messages: [] });
  },

  setFilter: (filter) => {
    set({ filter });
  },

  setFlowFilter: (flowId) => {
    set({ flowFilter: flowId });
  },
}));

// ---------------------------------------------------------------------------
// Selectors
// ---------------------------------------------------------------------------

/** Return messages filtered by text content and flow, in reverse chronological order. */
export function selectFilteredMessages(state: DebugStore): DebugMessage[] {
  let msgs = state.messages;

  // Flow filter
  if (state.flowFilter !== null) {
    msgs = msgs.filter((m) => m.sourceFlow === state.flowFilter);
  }

  // Text filter
  if (state.filter) {
    const q = state.filter.toLowerCase();
    msgs = msgs.filter((m) => {
      // Search topic
      if (m.topic && m.topic.toLowerCase().includes(q)) return true;
      // Search source node name
      if (m.sourceNode?.name && m.sourceNode.name.toLowerCase().includes(q)) return true;
      // Search payload (msg.payload)
      const payload = m.msg?.payload;
      if (payload !== undefined && String(payload).toLowerCase().includes(q)) return true;
      // Search full msg JSON
      if (JSON.stringify(m.msg).toLowerCase().includes(q)) return true;
      return false;
    });
  }

  // Reverse: newest first
  return [...msgs].reverse();
}

/** Return unique flow names from current messages (for the flow filter dropdown). */
export function selectUniqueFlows(state: DebugStore): string[] {
  const flows = new Set<string>();
  for (const m of state.messages) {
    if (m.sourceFlow) flows.add(m.sourceFlow);
  }
  return Array.from(flows).sort();
}
