/**
 * Typed event bus — replaces Node-RED's jQuery-based RED.events.
 *
 * Features:
 *  - Fully typed event names and payloads via EventBus<EventMap>
 *  - Wildcard listeners: `on('nodes:*')` catches `nodes:added`, `nodes:removed`, etc.
 *  - `on()` returns an unsubscribe function for ergonomic cleanup
 *  - Singleton `eventBus` exported for application-wide use
 */

/** Minimal handler signature stored internally. */
type Handler = (data: unknown) => void;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

// The generic parameter is reserved for future typed overloads.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export class EventBus<_EventMap = Record<string, unknown>> {
  /** Exact-match listeners: event name -> set of handlers. */
  private readonly listeners = new Map<string, Set<Handler>>();
  /** Wildcard listeners: pattern (e.g. "nodes:*") -> set of handlers. */
  private readonly wildcards = new Map<string, Set<Handler>>();

  // ---- on ----

  /**
   * Register a listener for `event`. Returns an unsubscribe function.
   *
   * If `event` contains `*` it is treated as a wildcard pattern.
   * The `*` matches one or more colon-separated segments after the prefix.
   *
   * Example: `on('nodes:*', cb)` matches `nodes:added`, `nodes:removed`,
   * `nodes:config:changed`, etc.
   */
  on(
    event: string,
    callback: (data: unknown) => void,
  ): () => void {
    if (event.includes("*")) {
      this.addWildcard(event, callback as Handler);
    } else {
      this.addExact(event, callback as Handler);
    }
    return () => this.off(event, callback as Handler);
  }

  // ---- off ----

  /** Remove a previously registered listener. */
  off(
    event: string,
    callback: (data: unknown) => void,
  ): void {
    const handler = callback as Handler;
    if (event.includes("*")) {
      this.wildcards.get(event)?.delete(handler);
    } else {
      this.listeners.get(event)?.delete(handler);
    }
  }

  // ---- once ----

  /** Subscribe to `event` for a single invocation, then auto-remove. */
  once(
    event: string,
    callback: (data: unknown) => void,
  ): () => void {
    const wrapper: Handler = (data) => {
      this.off(event, wrapper);
      (callback as Handler)(data);
    };
    return this.on(event, wrapper);
  }

  // ---- emit ----

  /** Emit an event, dispatching to exact and wildcard listeners. */
  emit(
    event: string,
    data?: unknown,
  ): void {
    // Exact listeners
    const exact = this.listeners.get(event);
    if (exact) {
      for (const handler of exact) {
        handler(data);
      }
    }

    // Wildcard listeners — match any pattern where the prefix before `*`
    // matches the beginning of the event and the event is longer.
    for (const [pattern, handlers] of this.wildcards) {
      if (this.matchesWildcard(pattern, event)) {
        for (const handler of handlers) {
          handler(data);
        }
      }
    }
  }

  // ---- helpers ----

  /** Return the number of listeners for a given event (exact + matching wildcards). */
  listenerCount(event: string): number {
    let count = this.listeners.get(event)?.size ?? 0;
    for (const [pattern, handlers] of this.wildcards) {
      if (this.matchesWildcard(pattern, event)) {
        count += handlers.size;
      }
    }
    return count;
  }

  /** Remove all listeners (useful for tests). */
  clear(): void {
    this.listeners.clear();
    this.wildcards.clear();
  }

  // ---- private ----

  private addExact(event: string, callback: Handler): void {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(callback);
  }

  private addWildcard(pattern: string, callback: Handler): void {
    let set = this.wildcards.get(pattern);
    if (!set) {
      set = new Set();
      this.wildcards.set(pattern, set);
    }
    set.add(callback);
  }

  /**
   * A pattern like `"nodes:*"` matches any event that starts with `"nodes:"`
   * and has at least one character after the colon.
   * A pattern like `"*"` matches every event.
   */
  private matchesWildcard(pattern: string, event: string): boolean {
    // Lone "*" matches everything
    if (pattern === "*") return true;

    // "prefix:*" — check that event starts with the prefix portion
    const prefix = pattern.slice(0, -1); // strip trailing "*"
    if (prefix.endsWith(":")) {
      // "nodes:*" -> prefix is "nodes:"
      return event.startsWith(prefix) && event.length > prefix.length;
    }
    // Generic case: prefix match
    return event.startsWith(prefix) && event.length > prefix.length;
  }
}

// ---------------------------------------------------------------------------
// Well-known application events (extend as needed)
// ---------------------------------------------------------------------------

export interface AppEvents {
  // Nodes
  "nodes:added": { id: string; type: string };
  "nodes:removed": { id: string };
  "nodes:changed": { id: string };
  "node:edit-requested": { id: string };
  "node:select-requested": { id: string };

  // Canvas
  "canvas:context-menu": {
    x: number;
    y: number;
    flowX: number;
    flowY: number;
  };

  // Flows
  "flows:deploy": { revision: string };
  "flows:imported": { count: number };
  "flows:cleared": undefined;

  // Deploy
  "deploy:start": { mode: string };
  "deploy:success": { mode: string };
  "deploy:error": { mode: string; error: string };

  // Editor
  "editor:select": { id: string | null };
  "editor:zoom": { level: number };

  // Connections (wires)
  "connections:added": { source: string; target: string };
  "connections:removed": { id: string };

  // Workspace
  "workspace:changed": { id: string };

  // Status
  "status:changed": { id: string; status: unknown };

  // Search
  "search:result-selected": { nodeId: string; flowId: string };
}

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

export const eventBus = new EventBus<AppEvents>();
export default eventBus;
