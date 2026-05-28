/**
 * StatusBar -- Thin status bar at the bottom of the editor window.
 *
 * Shows:
 *   - Connection status (green/red dot + label)
 *   - Last deploy time
 *   - Node count for the active flow
 *   - Current zoom level (hidden on small screens)
 *
 * Reads from: useStatusStore, useDeployStore, useFlowStore, and zoom
 * passed down as a prop from the canvas area.
 */

import { useMemo } from "react";
import { useStatusStore } from "../../store/status-store";
import { useDeployStore } from "../../store/deploy-store";
import { useFlowStore } from "../../store/flow-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface StatusBarProps {
  /** Current zoom level (0-1 range, displayed as percentage). */
  zoomLevel?: number;
  className?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDeployTime(timestamp: number | null): string {
  if (timestamp === null) return "Not deployed";
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function connectionDotColor(status: string): string {
  switch (status) {
    case "connected":
      return "bg-green-500";
    case "connecting":
      return "bg-yellow-500";
    default:
      return "bg-red-500";
  }
}

function connectionLabel(status: string): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "connecting":
      return "Connecting...";
    default:
      return "Disconnected";
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function StatusBar({ zoomLevel, className }: StatusBarProps) {
  const connectionStatus = useStatusStore((s) => s.connectionStatus);
  const lastDeployTime = useDeployStore((s) => s.lastDeployTime);
  const nodeCount = useFlowStore((s) => s.nodes.length);

  const deployLabel = useMemo(
    () => `Deployed: ${formatDeployTime(lastDeployTime)}`,
    [lastDeployTime],
  );

  const zoomPercent =
    zoomLevel !== undefined ? `${Math.round(zoomLevel * 100)}%` : "100%";

  return (
    <footer
      data-testid="status-bar"
      className={[
        "flex h-6 shrink-0 items-center bg-gray-900 text-xs text-gray-400 select-none",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {/* Left: connection status */}
      <div
        data-testid="status-bar-connection"
        className="flex items-center gap-1.5 px-3"
      >
        <span
          data-testid="status-bar-connection-dot"
          className={`inline-block h-2 w-2 rounded-full ${connectionDotColor(connectionStatus)}`}
        />
        <span>{connectionLabel(connectionStatus)}</span>
      </div>

      <span className="mx-1 h-3 w-px bg-gray-700" />

      {/* Center: deploy time */}
      <div data-testid="status-bar-deploy" className="px-3">
        {deployLabel}
      </div>

      <span className="mx-1 h-3 w-px bg-gray-700" />

      {/* Node count */}
      <div data-testid="status-bar-nodes" className="px-3">
        {nodeCount} node{nodeCount !== 1 ? "s" : ""}
      </div>

      {/* Right: zoom level (hidden on small screens) */}
      <div
        data-testid="status-bar-zoom"
        className="ml-auto hidden px-3 sm:block"
      >
        {zoomPercent}
      </div>
    </footer>
  );
}
