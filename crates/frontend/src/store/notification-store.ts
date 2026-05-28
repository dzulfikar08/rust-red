/**
 * Notification store -- replaces Node-RED's RED.notifications.
 *
 * A Zustand store for managing toast notifications.
 * Supports four types (success / warning / error / info),
 * auto-dismiss with configurable timeout, and action buttons.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Notification {
  id: string;
  type: "success" | "warning" | "error" | "info";
  title: string;
  message?: string;
  timeout: number; // ms, 0 = manual only
  timestamp: number;
  dismissible: boolean;
  actions?: { label: string; onClick: () => void }[];
}

export type NotificationInput = Omit<Notification, "id" | "timestamp">;

export interface NotificationStore {
  notifications: Notification[];

  /** Add a notification. Returns the generated id. */
  addNotification: (input: NotificationInput) => string;

  /** Remove a notification by id. */
  removeNotification: (id: string) => void;

  /** Remove all notifications. */
  clearAll: () => void;

  // Convenience helpers (default timeout 5000, dismissible true)
  success: (title: string, message?: string) => string;
  warning: (title: string, message?: string) => string;
  error: (title: string, message?: string) => string;
  info: (title: string, message?: string) => string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let nextId = 0;
function uid(): string {
  return `notif-${Date.now()}-${++nextId}`;
}

const DEFAULT_TIMEOUT = 5000;

function makeConvenience(
  type: Notification["type"],
  get: () => NotificationStore,
  title: string,
  message?: string,
): string {
  return get().addNotification({ type, title, message, timeout: DEFAULT_TIMEOUT, dismissible: true });
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useNotificationStore = create<NotificationStore>()((set, get) => ({
  notifications: [],

  addNotification: (input) => {
    const id = uid();
    const notification: Notification = {
      ...input,
      id,
      timestamp: Date.now(),
    };
    set((state) => ({
      notifications: [...state.notifications, notification],
    }));
    return id;
  },

  removeNotification: (id) => {
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    }));
  },

  clearAll: () => {
    set({ notifications: [] });
  },

  success: (title, message) => makeConvenience("success", () => get(), title, message),
  warning: (title, message) => makeConvenience("warning", () => get(), title, message),
  error: (title, message) => makeConvenience("error", () => get(), title, message),
  info: (title, message) => makeConvenience("info", () => get(), title, message),
}));

export default useNotificationStore;
