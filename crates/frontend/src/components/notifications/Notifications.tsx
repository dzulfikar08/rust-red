/**
 * Notifications -- toast notification container.
 *
 * Renders active notifications from the notification store as stacked toasts
 * in the top-right corner.  Matches Node-RED's visual style:
 * semi-transparent dark background, white text, coloured accent by type.
 */

import { useEffect, useRef, useState, useCallback } from "react";
import { X, CheckCircle, AlertTriangle, AlertCircle, Info } from "lucide-react";
import { useNotificationStore } from "../../store/notification-store";
import type { Notification } from "../../store/notification-store";

// ---------------------------------------------------------------------------
// Type -> style & icon mapping
// ---------------------------------------------------------------------------

const TYPE_CONFIG: Record<
  Notification["type"],
  { icon: typeof CheckCircle; accent: string; progress: string }
> = {
  success: {
    icon: CheckCircle,
    accent: "border-l-green-500",
    progress: "bg-green-400",
  },
  warning: {
    icon: AlertTriangle,
    accent: "border-l-amber-500",
    progress: "bg-amber-400",
  },
  error: {
    icon: AlertCircle,
    accent: "border-l-red-500",
    progress: "bg-red-400",
  },
  info: {
    icon: Info,
    accent: "border-l-blue-500",
    progress: "bg-blue-400",
  },
};

// ---------------------------------------------------------------------------
// Single toast
// ---------------------------------------------------------------------------

function Toast({ notification }: { notification: Notification }) {
  const removeNotification = useNotificationStore((s) => s.removeNotification);
  const [exiting, setExiting] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const startRef = useRef(Date.now());
  const [progress, setProgress] = useState(100);

  const dismiss = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setExiting(true);
    // Wait for slide-out animation before removing
    setTimeout(() => removeNotification(notification.id), 250);
  }, [notification.id, removeNotification]);

  // Auto-dismiss timer
  useEffect(() => {
    if (notification.timeout <= 0) return;

    startRef.current = Date.now();
    const duration = notification.timeout;

    // Progress updates
    const interval = setInterval(() => {
      const elapsed = Date.now() - startRef.current;
      const pct = Math.max(0, 100 - (elapsed / duration) * 100);
      setProgress(pct);
      if (pct <= 0) clearInterval(interval);
    }, 50);

    timerRef.current = setTimeout(() => {
      clearInterval(interval);
      dismiss();
    }, duration);

    return () => {
      clearInterval(interval);
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [notification.timeout, dismiss]);

  const config = TYPE_CONFIG[notification.type];
  const Icon = config.icon;

  return (
    <div
      role="alert"
      className={`
        pointer-events-auto flex w-80 overflow-hidden rounded-lg shadow-lg
        border-l-4 ${config.accent}
        bg-gray-900/90 text-white
        transition-all duration-250 ease-in-out
        ${exiting ? "translate-x-[120%] opacity-0" : "translate-x-0 opacity-100"}
      `}
    >
      {/* Icon */}
      <div className="flex items-center px-3">
        <Icon size={18} className="shrink-0 opacity-80" />
      </div>

      {/* Body */}
      <div className="flex-1 py-3 pr-2 min-w-0">
        <p className="text-sm font-medium leading-tight">{notification.title}</p>
        {notification.message && (
          <p className="mt-1 text-xs text-gray-300 leading-snug">
            {notification.message}
          </p>
        )}
        {notification.actions && notification.actions.length > 0 && (
          <div className="mt-2 flex gap-2">
            {notification.actions.map((action, i) => (
              <button
                key={i}
                type="button"
                onClick={action.onClick}
                className="rounded bg-white/15 px-2 py-0.5 text-xs font-medium text-white
                           hover:bg-white/25 transition-colors"
              >
                {action.label}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Close button */}
      {notification.dismissible && (
        <button
          type="button"
          onClick={dismiss}
          className="flex items-start pt-3 pr-2 text-gray-400 hover:text-white transition-colors"
          aria-label="Dismiss notification"
        >
          <X size={14} />
        </button>
      )}

      {/* Progress bar */}
      {notification.timeout > 0 && (
        <div className="absolute bottom-0 left-0 right-0 h-[2px] bg-gray-700">
          <div
            className={`h-full ${config.progress} transition-none`}
            style={{ width: `${progress}%` }}
          />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

export function Notifications() {
  const notifications = useNotificationStore((s) => s.notifications);

  if (notifications.length === 0) return null;

  return (
    <div
      className="pointer-events-none fixed top-3 right-3 z-[9999] flex flex-col gap-2"
      aria-live="polite"
    >
      {notifications.map((n) => (
        <Toast key={n.id} notification={n} />
      ))}
    </div>
  );
}

export default Notifications;
