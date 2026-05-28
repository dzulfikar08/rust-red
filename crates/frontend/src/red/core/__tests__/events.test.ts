import { describe, it, expect, vi, beforeEach } from "vitest";

/**
 * Minimal event bus placeholder — will be replaced by the real
 * RED.events implementation in Task 2 (Core infrastructure).
 */
function createEventBus() {
  const handlers = new Map<string, Set<(...args: unknown[]) => void>>();

  return {
    on(event: string, handler: (...args: unknown[]) => void) {
      if (!handlers.has(event)) {
        handlers.set(event, new Set());
      }
      handlers.get(event)!.add(handler);
    },
    off(event: string, handler: (...args: unknown[]) => void) {
      handlers.get(event)?.delete(handler);
    },
    emit(event: string, ...args: unknown[]) {
      handlers.get(event)?.forEach((h) => h(...args));
    },
    listenerCount(event: string) {
      return handlers.get(event)?.size ?? 0;
    },
  };
}

describe("Event bus (placeholder)", () => {
  const bus = createEventBus();

  beforeEach(() => {
    // Fresh bus for each test
  });

  it("registers and fires an event handler", () => {
    const callback = vi.fn();
    bus.on("nodes:add", callback);
    bus.emit("nodes:add", { id: "1" });
    expect(callback).toHaveBeenCalledOnce();
    expect(callback).toHaveBeenCalledWith({ id: "1" });
  });

  it("removes a handler with off()", () => {
    const callback = vi.fn();
    bus.on("flows:deploy", callback);
    bus.off("flows:deploy", callback);
    bus.emit("flows:deploy");
    expect(callback).not.toHaveBeenCalled();
  });

  it("tracks listener count", () => {
    const cb1 = vi.fn();
    const cb2 = vi.fn();
    bus.on("test:event", cb1);
    bus.on("test:event", cb2);
    expect(bus.listenerCount("test:event")).toBe(2);
    bus.off("test:event", cb1);
    expect(bus.listenerCount("test:event")).toBe(1);
  });
});
