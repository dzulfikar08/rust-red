/**
 * Custom React Flow Edge (Wire) Component
 *
 * Visually matches Node-RED's wire rendering:
 *  - Smooth bezier curve from output port (right) to input port (left)
 *  - Horizontal bias: wires go right from source, then curve to target
 *  - Default wire colour: #999 (grey)
 *  - Selected wire colour: #ff7700 (orange, matching Node-RED)
 *  - Hover: slightly brighter grey
 *  - Animated state: dashed animation for running flows
 *  - Stroke width: 2px
 */

import { memo, useState } from "react";
import {
  getBezierPath,
  BaseEdge,
  Position,
  type EdgeProps,
} from "@xyflow/react";

// ---------------------------------------------------------------------------
// Colour constants (Node-RED palette)
// ---------------------------------------------------------------------------

const WIRE_COLOR_DEFAULT = "#999";
const WIRE_COLOR_SELECTED = "#ff7700";
const WIRE_COLOR_HOVER = "#bbb";
const WIRE_STROKE_WIDTH = 2;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

function WireComponentRaw({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition = Position.Right,
  targetPosition = Position.Left,
  selected = false,
  style,
  ...rest
}: EdgeProps): JSX.Element {
  const [hovered, setHovered] = useState(false);

  // Calculate bezier path with horizontal bias (Right -> Left is the default)
  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  // Determine wire colour based on state priority: selected > hover > default
  const wireColor = selected
    ? WIRE_COLOR_SELECTED
    : hovered
      ? WIRE_COLOR_HOVER
      : WIRE_COLOR_DEFAULT;

  return (
    <g
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* Invisible wider interaction path for easier hovering */}
      <path
        d={edgePath}
        fill="none"
        strokeWidth={12}
        stroke="transparent"
        style={{ cursor: "pointer" }}
      />
      <BaseEdge
        id={id}
        path={edgePath}
        style={{
          stroke: wireColor,
          strokeWidth: WIRE_STROKE_WIDTH,
          ...style,
        }}
        {...rest}
      />
    </g>
  );
}

/**
 * Memoized custom edge component for Node-RED style wires.
 * Register with React Flow's `edgeTypes` under the key `'wire'`.
 */
export const WireComponent = memo(WireComponentRaw);
