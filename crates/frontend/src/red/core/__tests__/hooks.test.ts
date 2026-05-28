import { describe, it, expect, vi, beforeEach } from "vitest";
import { HookSystem } from "../hooks";

describe("HookSystem", () => {
  let hooks: HookSystem;

  beforeEach(() => {
    hooks = new HookSystem();
  });

  // -----------------------------------------------------------------------
  // addHook / callHooks
  // -----------------------------------------------------------------------

  describe("addHook() / callHooks()", () => {
    it("calls a registered hook", async () => {
      const callback = vi.fn();
      hooks.addHook("onDeploy", callback);
      await hooks.callHooks("onDeploy", { revision: "1" });
      expect(callback).toHaveBeenCalledOnce();
      expect(callback).toHaveBeenCalledWith({ revision: "1" });
    });

    it("calls multiple hooks for the same name", async () => {
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      hooks.addHook("onDeploy", cb1);
      hooks.addHook("onDeploy", cb2);
      await hooks.callHooks("onDeploy", {});
      expect(cb1).toHaveBeenCalledOnce();
      expect(cb2).toHaveBeenCalledOnce();
    });

    it("does not call hooks registered under a different name", async () => {
      const callback = vi.fn();
      hooks.addHook("onDeploy", callback);
      await hooks.callHooks("nodes:add", {});
      expect(callback).not.toHaveBeenCalled();
    });

    it("does nothing when no hooks are registered for a name", async () => {
      await expect(hooks.callHooks("nonexistent", {})).resolves.toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Priority ordering
  // -----------------------------------------------------------------------

  describe("priority ordering", () => {
    it("executes hooks in ascending priority order", async () => {
      const order: number[] = [];
      hooks.addHook("test", () => { order.push(10); }, 10);
      hooks.addHook("test", () => { order.push(0); }, 0);
      hooks.addHook("test", () => { order.push(5); }, 5);

      await hooks.callHooks("test", {});
      expect(order).toEqual([0, 5, 10]);
    });

    it("preserves insertion order for hooks with equal priority", async () => {
      const order: string[] = [];
      hooks.addHook("test", () => { order.push("a"); }, 0);
      hooks.addHook("test", () => { order.push("b"); }, 0);
      hooks.addHook("test", () => { order.push("c"); }, 0);

      await hooks.callHooks("test", {});
      expect(order).toEqual(["a", "b", "c"]);
    });

    it("uses default priority of 0", async () => {
      const order: string[] = [];
      hooks.addHook("test", () => { order.push("default"); });
      hooks.addHook("test", () => { order.push("negative"); }, -1);
      hooks.addHook("test", () => { order.push("positive"); }, 1);

      await hooks.callHooks("test", {});
      expect(order).toEqual(["negative", "default", "positive"]);
    });

    it("handles negative priorities", async () => {
      const order: number[] = [];
      hooks.addHook("test", () => { order.push(5); }, 5);
      hooks.addHook("test", () => { order.push(-10); }, -10);
      hooks.addHook("test", () => { order.push(0); }, 0);

      await hooks.callHooks("test", {});
      expect(order).toEqual([-10, 0, 5]);
    });
  });

  // -----------------------------------------------------------------------
  // removeHook
  // -----------------------------------------------------------------------

  describe("removeHook()", () => {
    it("removes a specific hook callback", async () => {
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      hooks.addHook("test", cb1);
      hooks.addHook("test", cb2);
      hooks.removeHook("test", cb1);

      await hooks.callHooks("test", {});
      expect(cb1).not.toHaveBeenCalled();
      expect(cb2).toHaveBeenCalledOnce();
    });

    it("does nothing when removing a non-existent callback", () => {
      expect(() => hooks.removeHook("nonexistent", vi.fn())).not.toThrow();
    });

    it("does nothing when removing from a non-existent hook name", () => {
      const callback = vi.fn();
      expect(() => hooks.removeHook("missing", callback)).not.toThrow();
    });
  });

  // -----------------------------------------------------------------------
  // Async hooks
  // -----------------------------------------------------------------------

  describe("async hooks", () => {
    it("awaits async hooks sequentially", async () => {
      const order: string[] = [];

      hooks.addHook("test", async () => {
        order.push("start-1");
        await Promise.resolve();
        order.push("end-1");
      }, 0);

      hooks.addHook("test", async () => {
        order.push("start-2");
        await Promise.resolve();
        order.push("end-2");
      }, 1);

      await hooks.callHooks("test", {});
      expect(order).toEqual(["start-1", "end-1", "start-2", "end-2"]);
    });

    it("handles a mix of sync and async hooks", async () => {
      const order: string[] = [];

      hooks.addHook("test", () => {
        order.push("sync");
      }, 0);

      hooks.addHook("test", async () => {
        order.push("async");
      }, 1);

      await hooks.callHooks("test", {});
      expect(order).toEqual(["sync", "async"]);
    });
  });

  // -----------------------------------------------------------------------
  // Data mutation
  // -----------------------------------------------------------------------

  describe("data mutation", () => {
    it("allows hooks to mutate the data object", async () => {
      const data = { value: 0 };
      hooks.addHook("test", (d) => {
        (d as { value: number }).value += 1;
      });
      hooks.addHook("test", (d) => {
        (d as { value: number }).value += 10;
      });

      await hooks.callHooks("test", data);
      expect(data.value).toBe(11);
    });
  });

  // -----------------------------------------------------------------------
  // hookCount
  // -----------------------------------------------------------------------

  describe("hookCount()", () => {
    it("returns 0 when no hooks registered", () => {
      expect(hooks.hookCount("test")).toBe(0);
    });

    it("returns the number of registered hooks", () => {
      hooks.addHook("test", vi.fn());
      hooks.addHook("test", vi.fn());
      expect(hooks.hookCount("test")).toBe(2);
    });

    it("decrements after removal", () => {
      const cb = vi.fn();
      hooks.addHook("test", cb);
      expect(hooks.hookCount("test")).toBe(1);
      hooks.removeHook("test", cb);
      expect(hooks.hookCount("test")).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // clear
  // -----------------------------------------------------------------------

  describe("clear()", () => {
    it("removes all hooks", async () => {
      const cb = vi.fn();
      hooks.addHook("test", cb);
      hooks.addHook("other", cb);
      hooks.clear();
      await hooks.callHooks("test", {});
      await hooks.callHooks("other", {});
      expect(cb).not.toHaveBeenCalled();
    });
  });
});
