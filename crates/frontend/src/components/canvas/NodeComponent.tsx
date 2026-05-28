/**
 * Custom React Flow Node Component
 *
 * Visually matches Node-RED's node appearance:
 *  - Rounded rectangle body coloured by category
 *  - Label with optional icon on the left
 *  - Input port(s) on the left, output port(s) on the right
 *  - Status indicator dot at the bottom-left
 *  - Selected / disabled / hover states
 */

import { memo, useMemo } from "react";
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";

// ---------------------------------------------------------------------------
// Data shape carried by every NR node on the React Flow canvas
// ---------------------------------------------------------------------------

export interface NodeComponentData {
  /** Node-RED type key, e.g. "inject" */
  type: string;
  /** Display label */
  label: string;
  /** Category / background colour (hex) */
  color: string;
  /** Font Awesome icon class, e.g. "fa-solid fa-arrow-right" */
  icon?: string;
  /** Number of input ports (0 or 1) */
  inputs: number;
  /** Number of output ports (0+) */
  outputs: number;
  /** Runtime status indicator */
  status?: "ok" | "warning" | "error" | "disconnected";
  /** Whether the node is disabled / greyed out */
  disabled?: boolean;
}

export type NRNode = Node<NodeComponentData, "nrNode">;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Map status to a fill colour for the indicator dot. */
function statusColor(status: NodeComponentData["status"]): string {
  switch (status) {
    case "ok":
      return "#5ea652";
    case "warning":
      return "#d7d240";
    case "error":
      return "#e05555";
    case "disconnected":
      return "#aaa";
    default:
      return "";
  }
}

/** Darken a hex colour by a fixed amount for port circles. */
function darken(hex: string, amount = 40): string {
  const raw = hex.replace("#", "");
  const r = Math.max(0, parseInt(raw.substring(0, 2), 16) - amount);
  const g = Math.max(0, parseInt(raw.substring(2, 4), 16) - amount);
  const b = Math.max(0, parseInt(raw.substring(4, 6), 16) - amount);
  return `rgb(${r},${g},${b})`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

function NodeComponentRaw({ data, selected }: NodeProps<NRNode>) {
  const d = data;
  const bgColor = d.color || "#a6bbcf";
  const portColor = darken(bgColor, 50);

  // Calculate dynamic width based on label length (min 80, max 200)
  const nodeWidth = useMemo(() => {
    const charWidth = 7;
    const padding = 40; // icon + internal padding
    const calculated = d.label.length * charWidth + padding;
    return Math.min(200, Math.max(80, calculated));
  }, [d.label]);

  const nodeHeight = 32;

  return (
    <div
      className="relative group"
      style={{ width: nodeWidth, height: nodeHeight }}
    >
      {/* ----- Node body ----- */}
      <div
        className={`
          absolute inset-0 rounded-[6px] border-2
          flex items-center gap-1 px-2 overflow-hidden
          transition-colors duration-100
          ${selected
            ? "border-[#4a90d9] shadow-[0_0_6px_rgba(74,144,217,0.5)]"
            : "border-[rgba(0,0,0,0.15)]"
          }
          ${d.disabled ? "opacity-50" : ""}
        `}
        style={{
          backgroundColor: bgColor,
        }}
      >
        {/* Hover overlay */}
        <div
          className="
            absolute inset-0 rounded-[4px] opacity-0
            group-hover:opacity-100 transition-opacity duration-100
            pointer-events-none
          "
          style={{ backgroundColor: "rgba(255,255,255,0.12)" }}
        />

        {/* Icon */}
        {d.icon && (
          <i
            className={`${d.icon} text-[11px] shrink-0`}
            style={{ color: darken(bgColor, 60) }}
          />
        )}

        {/* Label */}
        <span
          className="
            text-[11px] font-medium leading-none truncate
            relative z-10 select-none pointer-events-none
          "
          style={{ color: darken(bgColor, 100) }}
        >
          {d.label}
        </span>
      </div>

      {/* ----- Input handles (left, type="target") ----- */}
      {d.inputs > 0 &&
        Array.from({ length: d.inputs }).map((_, i) => {
          const top = d.inputs === 1
            ? "50%"
            : `${((i + 1) / (d.inputs + 1)) * 100}%`;
          return (
            <Handle
              key={`in-${i}`}
              type="target"
              position={Position.Left}
              id={`input-${i}`}
              style={{
                top,
                width: 8,
                height: 8,
                left: -4,
                background: portColor,
                border: "1.5px solid rgba(255,255,255,0.6)",
                borderRadius: "50%",
              }}
            />
          );
        })}

      {/* ----- Output handles (right, type="source") ----- */}
      {d.outputs > 0 &&
        Array.from({ length: d.outputs }).map((_, i) => {
          const top = d.outputs === 1
            ? "50%"
            : `${((i + 1) / (d.outputs + 1)) * 100}%`;
          return (
            <Handle
              key={`out-${i}`}
              type="source"
              position={Position.Right}
              id={`output-${i}`}
              style={{
                top,
                width: 8,
                height: 8,
                right: -4,
                background: portColor,
                border: "1.5px solid rgba(255,255,255,0.6)",
                borderRadius: "50%",
              }}
            />
          );
        })}

      {/* ----- Status indicator dot ----- */}
      {d.status && (
        <div
          className="absolute bottom-[-5px] left-[6px] w-[8px] h-[8px] rounded-full z-20"
          style={{
            backgroundColor: statusColor(d.status),
            border: "1px solid rgba(0,0,0,0.15)",
          }}
          data-testid="node-status-indicator"
        />
      )}
    </div>
  );
}

export const NodeComponent = memo(NodeComponentRaw);
