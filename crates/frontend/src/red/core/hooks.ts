/**
 * Lifecycle hooks system — replaces Node-RED's RED.hooks.
 *
 * Features:
 *  - `addHook(name, callback, priority?)` — register a hook
 *  - `removeHook(name, callback)` — unregister a hook
 *  - `callHooks(name, data)` — execute all hooks for a name in priority order
 *  - Async hooks are supported (callHooks returns a Promise)
 *  - Hook names follow Node-RED convention: `onDeploy`, `nodes:add`, etc.
 */

type HookCallback = (data: unknown) => unknown | Promise<unknown>;

interface HookEntry {
  callback: HookCallback;
  priority: number;
}

export class HookSystem {
  /** Map of hook name -> sorted array of HookEntry (ascending priority). */
  private readonly hooks = new Map<string, HookEntry[]>();

  /**
   * Register a hook callback for `hookName`.
   *
   * @param hookName - The lifecycle hook name (e.g. `onDeploy`, `nodes:add`).
   * @param callback - Function to call. Receives mutable `data` and may return
   *   a value or a Promise.
   * @param priority - Execution order (ascending). Lower runs first. Default 0.
   *   Hooks with equal priority run in insertion order.
   */
  addHook(hookName: string, callback: HookCallback, priority = 0): void {
    let entries = this.hooks.get(hookName);
    if (!entries) {
      entries = [];
      this.hooks.set(hookName, entries);
    }

    // Insert in sorted position (stable: equal priority keeps insertion order)
    const entry: HookEntry = { callback, priority };
    let inserted = false;
    for (let i = 0; i < entries.length; i++) {
      if (entries[i].priority > priority) {
        entries.splice(i, 0, entry);
        inserted = true;
        break;
      }
    }
    if (!inserted) {
      entries.push(entry);
    }
  }

  /**
   * Remove a previously registered hook callback.
   */
  removeHook(hookName: string, callback: HookCallback): void {
    const entries = this.hooks.get(hookName);
    if (!entries) return;
    const idx = entries.findIndex((e) => e.callback === callback);
    if (idx !== -1) {
      entries.splice(idx, 1);
    }
    if (entries.length === 0) {
      this.hooks.delete(hookName);
    }
  }

  /**
   * Execute all hooks registered for `hookName`, in ascending priority order.
   *
   * If any hook callback is async (returns a Promise), the whole chain is
   * awaited sequentially.
   *
   * @param hookName - The hook name to fire.
   * @param data - Data passed to each hook. Hooks may mutate this object.
   * @returns A Promise that resolves when all hooks have completed.
   */
  async callHooks(hookName: string, data: unknown): Promise<void> {
    const entries = this.hooks.get(hookName);
    if (!entries || entries.length === 0) return;

    for (const entry of entries) {
      await entry.callback(data);
    }
  }

  /**
   * Return the number of hooks registered for `hookName`.
   */
  hookCount(hookName: string): number {
    return this.hooks.get(hookName)?.length ?? 0;
  }

  /**
   * Remove all hooks (useful for tests).
   */
  clear(): void {
    this.hooks.clear();
  }
}

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

export const hookSystem = new HookSystem();
export default hookSystem;
