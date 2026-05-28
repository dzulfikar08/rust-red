import { describe, it, expect, beforeEach } from "vitest";
import {
  useDebugStore,
  selectFilteredMessages,
  selectUniqueFlows,
  type DebugMessage,
} from "../debug-store";

function makeMsg(overrides: Partial<DebugMessage> = {}): DebugMessage {
  return {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    msg: { payload: "test" },
    ...overrides,
  };
}

function resetStore() {
  useDebugStore.setState({
    messages: [],
    filter: "",
    flowFilter: null,
    maxMessages: 1000,
  });
}

describe("useDebugStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with empty messages", () => {
      expect(useDebugStore.getState().messages).toEqual([]);
    });

    it("starts with empty filter", () => {
      expect(useDebugStore.getState().filter).toBe("");
    });

    it("starts with null flowFilter", () => {
      expect(useDebugStore.getState().flowFilter).toBeNull();
    });

    it("starts with maxMessages of 1000", () => {
      expect(useDebugStore.getState().maxMessages).toBe(1000);
    });
  });

  // -----------------------------------------------------------------------
  // addMessage
  // -----------------------------------------------------------------------

  describe("addMessage()", () => {
    it("adds a message to the store", () => {
      const msg = makeMsg();
      useDebugStore.getState().addMessage(msg);
      expect(useDebugStore.getState().messages).toHaveLength(1);
      expect(useDebugStore.getState().messages[0].id).toBe(msg.id);
    });

    it("appends messages in order", () => {
      const msg1 = makeMsg({ id: "first" });
      const msg2 = makeMsg({ id: "second" });
      useDebugStore.getState().addMessage(msg1);
      useDebugStore.getState().addMessage(msg2);
      const msgs = useDebugStore.getState().messages;
      expect(msgs[0].id).toBe("first");
      expect(msgs[1].id).toBe("second");
    });

    it("trims messages to maxMessages", () => {
      useDebugStore.setState({ maxMessages: 3 });
      for (let i = 0; i < 5; i++) {
        useDebugStore.getState().addMessage(makeMsg({ id: `msg-${i}` }));
      }
      const msgs = useDebugStore.getState().messages;
      expect(msgs).toHaveLength(3);
      // Keeps the most recent (last 3)
      expect(msgs[0].id).toBe("msg-2");
      expect(msgs[1].id).toBe("msg-3");
      expect(msgs[2].id).toBe("msg-4");
    });
  });

  // -----------------------------------------------------------------------
  // clearMessages
  // -----------------------------------------------------------------------

  describe("clearMessages()", () => {
    it("removes all messages", () => {
      useDebugStore.getState().addMessage(makeMsg());
      useDebugStore.getState().addMessage(makeMsg());
      expect(useDebugStore.getState().messages).toHaveLength(2);

      useDebugStore.getState().clearMessages();
      expect(useDebugStore.getState().messages).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // setFilter
  // -----------------------------------------------------------------------

  describe("setFilter()", () => {
    it("sets the text filter", () => {
      useDebugStore.getState().setFilter("hello");
      expect(useDebugStore.getState().filter).toBe("hello");
    });
  });

  // -----------------------------------------------------------------------
  // setFlowFilter
  // -----------------------------------------------------------------------

  describe("setFlowFilter()", () => {
    it("sets the flow filter to a flow id", () => {
      useDebugStore.getState().setFlowFilter("flow-1");
      expect(useDebugStore.getState().flowFilter).toBe("flow-1");
    });

    it("resets the flow filter to null (All flows)", () => {
      useDebugStore.getState().setFlowFilter("flow-1");
      useDebugStore.getState().setFlowFilter(null);
      expect(useDebugStore.getState().flowFilter).toBeNull();
    });
  });
});

// ---------------------------------------------------------------------------
// Selectors
// ---------------------------------------------------------------------------

describe("selectFilteredMessages", () => {
  beforeEach(() => {
    resetStore();
  });

  it("returns all messages in reverse order when no filters", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ id: "a" }));
    state.addMessage(makeMsg({ id: "b" }));
    state.addMessage(makeMsg({ id: "c" }));

    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered.map((m) => m.id)).toEqual(["c", "b", "a"]);
  });

  it("filters by text matching topic", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ id: "a", topic: "greeting" }));
    state.addMessage(makeMsg({ id: "b", topic: "warning" }));
    state.addMessage(makeMsg({ id: "c", topic: "data" }));

    useDebugStore.setState({ filter: "greet" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("a");
  });

  it("filters by text matching sourceNode name", () => {
    const state = useDebugStore.getState();
    state.addMessage(
      makeMsg({ id: "a", sourceNode: { id: "n1", name: "MyDebug", type: "debug" } }),
    );
    state.addMessage(
      makeMsg({ id: "b", sourceNode: { id: "n2", name: "Other", type: "debug" } }),
    );

    useDebugStore.setState({ filter: "mydebug" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("a");
  });

  it("filters by text matching payload", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ id: "a", msg: { payload: "Hello World" } }));
    state.addMessage(makeMsg({ id: "b", msg: { payload: "Goodbye" } }));

    useDebugStore.setState({ filter: "hello" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("a");
  });

  it("filters by text matching any msg key in JSON", () => {
    const state = useDebugStore.getState();
    state.addMessage(
      makeMsg({ id: "a", msg: { payload: "x", _msgid: "secret123" } }),
    );
    state.addMessage(makeMsg({ id: "b", msg: { payload: "y" } }));

    useDebugStore.setState({ filter: "secret123" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("a");
  });

  it("filters by flow", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ id: "a", sourceFlow: "Flow 1" }));
    state.addMessage(makeMsg({ id: "b", sourceFlow: "Flow 2" }));
    state.addMessage(makeMsg({ id: "c", sourceFlow: "Flow 1" }));

    useDebugStore.setState({ flowFilter: "Flow 1" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(2);
    expect(filtered.map((m) => m.id)).toEqual(["c", "a"]);
  });

  it("combines text and flow filters", () => {
    const state = useDebugStore.getState();
    state.addMessage(
      makeMsg({ id: "a", sourceFlow: "Flow 1", topic: "hello" }),
    );
    state.addMessage(
      makeMsg({ id: "b", sourceFlow: "Flow 1", topic: "goodbye" }),
    );
    state.addMessage(
      makeMsg({ id: "c", sourceFlow: "Flow 2", topic: "hello" }),
    );

    useDebugStore.setState({ filter: "hello", flowFilter: "Flow 1" });
    const filtered = selectFilteredMessages(useDebugStore.getState());
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("a");
  });
});

describe("selectUniqueFlows", () => {
  beforeEach(() => {
    resetStore();
  });

  it("returns empty array when no messages", () => {
    const flows = selectUniqueFlows(useDebugStore.getState());
    expect(flows).toEqual([]);
  });

  it("returns unique sorted flow names", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ sourceFlow: "Beta" }));
    state.addMessage(makeMsg({ sourceFlow: "Alpha" }));
    state.addMessage(makeMsg({ sourceFlow: "Beta" }));

    const flows = selectUniqueFlows(useDebugStore.getState());
    expect(flows).toEqual(["Alpha", "Beta"]);
  });

  it("skips messages without sourceFlow", () => {
    const state = useDebugStore.getState();
    state.addMessage(makeMsg({ sourceFlow: "Flow 1" }));
    state.addMessage(makeMsg({}));

    const flows = selectUniqueFlows(useDebugStore.getState());
    expect(flows).toEqual(["Flow 1"]);
  });
});
