import { describe, it, expect, vi, beforeEach } from "vitest";
import { EventBus } from "../events";

describe("EventBus", () => {
  let bus: EventBus;

  beforeEach(() => {
    bus = new EventBus();
  });

  // -----------------------------------------------------------------------
  // on / emit
  // -----------------------------------------------------------------------

  describe("on() / emit()", () => {
    it("registers and fires an event handler", () => {
      const callback = vi.fn();
      bus.on("nodes:added", callback);
      bus.emit("nodes:added", { id: "1", type: "inject" });
      expect(callback).toHaveBeenCalledOnce();
      expect(callback).toHaveBeenCalledWith({ id: "1", type: "inject" });
    });

    it("supports multiple listeners on the same event", () => {
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      bus.on("test", cb1);
      bus.on("test", cb2);
      bus.emit("test", 42);
      expect(cb1).toHaveBeenCalledWith(42);
      expect(cb2).toHaveBeenCalledWith(42);
    });

    it("does not fire listeners for different events", () => {
      const callback = vi.fn();
      bus.on("nodes:added", callback);
      bus.emit("nodes:removed", { id: "1" });
      expect(callback).not.toHaveBeenCalled();
    });

    it("returns an unsubscribe function from on()", () => {
      const callback = vi.fn();
      const unsub = bus.on("test", callback);
      unsub();
      bus.emit("test");
      expect(callback).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // off
  // -----------------------------------------------------------------------

  describe("off()", () => {
    it("removes a specific handler", () => {
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      bus.on("test", cb1);
      bus.on("test", cb2);
      bus.off("test", cb1);
      bus.emit("test");
      expect(cb1).not.toHaveBeenCalled();
      expect(cb2).toHaveBeenCalledOnce();
    });

    it("does nothing when removing a non-existent handler", () => {
      const callback = vi.fn();
      expect(() => bus.off("nonexistent", callback)).not.toThrow();
    });
  });

  // -----------------------------------------------------------------------
  // once
  // -----------------------------------------------------------------------

  describe("once()", () => {
    it("fires only once then auto-removes", () => {
      const callback = vi.fn();
      bus.once("test", callback);
      bus.emit("test", "first");
      bus.emit("test", "second");
      expect(callback).toHaveBeenCalledOnce();
      expect(callback).toHaveBeenCalledWith("first");
    });

    it("returns an unsubscribe function that prevents the handler from firing", () => {
      const callback = vi.fn();
      const unsub = bus.once("test", callback);
      unsub();
      bus.emit("test");
      expect(callback).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // Wildcards
  // -----------------------------------------------------------------------

  describe("wildcard listeners", () => {
    it("matches events with a prefix:* pattern", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.emit("nodes:added", { id: "1" });
      bus.emit("nodes:removed", { id: "2" });
      expect(callback).toHaveBeenCalledTimes(2);
    });

    it("does not match events with a different prefix", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.emit("flows:deploy", { revision: "abc" });
      expect(callback).not.toHaveBeenCalled();
    });

    it("does not match the bare prefix event itself", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.emit("nodes" as never, undefined);
      expect(callback).not.toHaveBeenCalled();
    });

    it("matches multi-segment events (e.g. nodes:config:changed)", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.emit("nodes:config:changed" as never, { id: "1" });
      expect(callback).toHaveBeenCalledOnce();
    });

    it("* alone matches every event", () => {
      const callback = vi.fn();
      bus.on("*", callback);
      bus.emit("nodes:added", { id: "1" });
      bus.emit("flows:deploy", { revision: "r1" });
      bus.emit("editor:select", { id: null });
      expect(callback).toHaveBeenCalledTimes(3);
    });

    it("off() removes a wildcard listener", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.off("nodes:*", callback);
      bus.emit("nodes:added", { id: "1" });
      expect(callback).not.toHaveBeenCalled();
    });

    it("receives correct data for each matching event", () => {
      const callback = vi.fn();
      bus.on("nodes:*", callback);
      bus.emit("nodes:added", { id: "a" });
      bus.emit("nodes:removed", { id: "b" });
      expect(callback).toHaveBeenNthCalledWith(1, { id: "a" });
      expect(callback).toHaveBeenNthCalledWith(2, { id: "b" });
    });

    it("works alongside exact-match listeners", () => {
      const exactCb = vi.fn();
      const wildcardCb = vi.fn();
      bus.on("nodes:added", exactCb);
      bus.on("nodes:*", wildcardCb);
      bus.emit("nodes:added", { id: "1" });
      expect(exactCb).toHaveBeenCalledOnce();
      expect(wildcardCb).toHaveBeenCalledOnce();
    });
  });

  // -----------------------------------------------------------------------
  // listenerCount
  // -----------------------------------------------------------------------

  describe("listenerCount()", () => {
    it("returns 0 for events with no listeners", () => {
      expect(bus.listenerCount("nothing")).toBe(0);
    });

    it("counts exact-match listeners", () => {
      bus.on("test", vi.fn());
      bus.on("test", vi.fn());
      expect(bus.listenerCount("test")).toBe(2);
    });

    it("includes matching wildcard listeners in the count", () => {
      bus.on("nodes:*", vi.fn());
      bus.on("nodes:added", vi.fn());
      expect(bus.listenerCount("nodes:added")).toBe(2);
    });
  });

  // -----------------------------------------------------------------------
  // clear
  // -----------------------------------------------------------------------

  describe("clear()", () => {
    it("removes all listeners", () => {
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      bus.on("test", cb1);
      bus.on("nodes:*", cb2);
      bus.clear();
      bus.emit("test");
      bus.emit("nodes:added", { id: "1" });
      expect(cb1).not.toHaveBeenCalled();
      expect(cb2).not.toHaveBeenCalled();
    });
  });
});
