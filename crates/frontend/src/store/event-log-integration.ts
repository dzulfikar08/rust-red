/**
 * Event log integration -- subscribes to the event bus and logs events.
 *
 * Call `initEventLogIntegration()` once during app bootstrap to start
 * capturing events. Returns a cleanup function to unsubscribe.
 */

import { eventBus } from "../red/core/events";
import { useEventLogStore } from "../store/event-log-store";

// ---------------------------------------------------------------------------
// Event -> LogEntry mapping
// ---------------------------------------------------------------------------

interface EventDescriptor {
  /** Friendly description for the log message. */
  message: string;
  /** Log level. */
  level: "info" | "warn" | "error";
}

/**
 * Build a descriptive message from event data using a template.
 * Supports `{key}` placeholders resolved from the data object.
 */
function resolveMessage(
  template: string,
  data: unknown,
): string {
  if (!data || typeof data !== "object") return template;
  return template.replace(/\{(\w+)\}/g, (_match, key: string) => {
    const val = (data as Record<string, unknown>)[key];
    return val !== undefined ? String(val) : `{${key}}`;
  });
}

// ---------------------------------------------------------------------------
// Event subscriptions
// ---------------------------------------------------------------------------

type Level = "info" | "warn" | "error";

interface Subscription {
  event: string;
  level: Level;
  message: string;
}

const SUBSCRIPTIONS: Subscription[] = [
  // Nodes
  { event: "nodes:added", level: "info", message: "Node added: {type} ({id})" },
  { event: "nodes:removed", level: "info", message: "Node removed: {id}" },
  { event: "nodes:changed", level: "info", message: "Node changed: {id}" },

  // Canvas
  { event: "canvas:context-menu", level: "info", message: "Context menu opened" },

  // Flows
  { event: "flows:deploy", level: "info", message: "Flow deployed (revision: {revision})" },
  { event: "flows:imported", level: "info", message: "Flows imported ({count} items)" },
  { event: "flows:cleared", level: "info", message: "Flows cleared" },

  // Deploy
  { event: "deploy:start", level: "info", message: "Deploy started ({mode})" },
  { event: "deploy:success", level: "info", message: "Deploy succeeded ({mode})" },
  { event: "deploy:error", level: "error", message: "Deploy error: {error}" },

  // Editor
  { event: "editor:select", level: "info", message: "Editor selection changed: {id}" },
  { event: "editor:zoom", level: "info", message: "Editor zoom: {level}" },

  // Connections
  { event: "connections:added", level: "info", message: "Connection added: {source} -> {target}" },
  { event: "connections:removed", level: "info", message: "Connection removed: {id}" },

  // Workspace
  { event: "workspace:changed", level: "info", message: "Workspace changed: {id}" },

  // Status
  { event: "status:changed", level: "info", message: "Status changed: {id}" },

  // Node interactions
  { event: "node:edit-requested", level: "info", message: "Edit requested: {id}" },
  { event: "node:select-requested", level: "info", message: "Select requested: {id}" },
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

let initialized = false;
const cleanups: (() => void)[] = [];

/**
 * Initialize the event log integration. Subscribes to all known events
 * on the event bus and logs them to the event log store.
 *
 * Returns a cleanup function that unsubscribes all listeners.
 */
export function initEventLogIntegration(): () => void {
  if (initialized) {
    return () => {};
  }
  initialized = true;

  const addEntry = useEventLogStore.getState().addEntry;

  for (const sub of SUBSCRIPTIONS) {
    const unsub = eventBus.on(sub.event, (data) => {
      addEntry({
        type: sub.event,
        message: resolveMessage(sub.message, data),
        level: sub.level,
        data:
          data !== null && data !== undefined && typeof data === "object"
            ? (data as Record<string, unknown>)
            : undefined,
      });
    });
    cleanups.push(unsub);
  }

  return () => {
    for (const cleanup of cleanups) {
      cleanup();
    }
    cleanups.length = 0;
    initialized = false;
  };
}

/**
 * Reset initialization state (for tests).
 */
export function _resetEventLogIntegration(): void {
  for (const cleanup of cleanups) {
    cleanup();
  }
  cleanups.length = 0;
  initialized = false;
}
